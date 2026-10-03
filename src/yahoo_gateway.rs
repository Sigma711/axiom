//! Restricted daily-chart gateway for clients whose region cannot reach Yahoo.
//! The upstream is fixed; this endpoint cannot proxy caller-selected URLs.
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
static REQUESTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(3);

#[derive(Clone)]
pub struct GatewayConfig {
    pub chart_url: String,
}
impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            chart_url: "https://query2.finance.yahoo.com/v8/finance/chart".into(),
        }
    }
}

pub fn router(config: GatewayConfig) -> Router {
    Router::new()
        .route("/yahoo-chart", get(chart))
        .with_state(config)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChartQuery {
    symbol: String,
    period1: i64,
    period2: i64,
}
type Error = (StatusCode, String);
fn upstream_error(message: impl ToString) -> Error {
    (StatusCode::BAD_GATEWAY, message.to_string())
}

async fn chart(
    State(config): State<GatewayConfig>,
    Query(query): Query<ChartQuery>,
) -> Result<Json<Value>, Error> {
    crate::api_validation::market(&query.symbol, "us_stock", 1, 1)?;
    if !query.symbol.as_bytes()[0].is_ascii_uppercase()
        || query.symbol.split(['.', '-']).any(str::is_empty)
        || query.period1 < 0
        || query.period2 <= query.period1
        || query.period2 > Utc::now().timestamp() + 86400
        || query.period2 - query.period1 > 16000 * 86400
    {
        return Err(crate::api_validation::bad("daily chart requires a valid US ticker and an ordered window of at most 16000 days, ending no later than tomorrow"));
    }
    let _permit = REQUESTS.try_acquire().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Market reader is busy; retry shortly".into(),
        )
    })?;
    let provider_symbol = query.symbol.replace('.', "-");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(upstream_error)?;
    let response = client
        .get(format!(
            "{}/{}",
            config.chart_url.trim_end_matches('/'),
            provider_symbol
        ))
        .header("User-Agent", "AXIOM educational market reader/1.0")
        .query(&[
            ("period1", query.period1.to_string()),
            ("period2", query.period2.to_string()),
            ("interval", "1d".into()),
            ("includePrePost", "false".into()),
            ("events", "div,splits".into()),
        ])
        .send()
        .await
        .map_err(upstream_error)?
        .error_for_status()
        .map_err(upstream_error)?;
    let source_url = response.url().to_string();
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(upstream_error)?;
        if bytes.len().saturating_add(chunk.len()) > 2_000_000 {
            return Err(upstream_error(
                "Yahoo daily chart exceeds the response limit",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let payload: Value = serde_json::from_slice(&bytes).map_err(upstream_error)?;
    let results = payload["chart"]["result"]
        .as_array()
        .filter(|rows| rows.len() == 1)
        .ok_or_else(|| upstream_error("Yahoo must return one matching stock chart"))?;
    let result = &results[0];
    if !payload["chart"]["error"].is_null()
        || result["meta"]["symbol"] != provider_symbol
        || result["meta"]["currency"] != "USD"
        || !matches!(
            result["meta"]["instrumentType"].as_str(),
            Some("EQUITY" | "ETF")
        )
        || result["timestamp"]
            .as_array()
            .is_none_or(|rows| rows.is_empty() || rows.len() > 12000)
    {
        return Err(upstream_error("Yahoo chart identity, currency, instrument or observations do not match the restricted request"));
    }
    let mut previous = None;
    for timestamp in result["timestamp"].as_array().unwrap() {
        let timestamp = timestamp
            .as_i64()
            .filter(|value| {
                *value >= query.period1
                    && *value < query.period2
                    && previous.is_none_or(|previous| *value > previous)
            })
            .ok_or_else(|| {
                upstream_error(
                    "Yahoo chart timestamps must be ordered integers within the requested window",
                )
            })?;
        previous = Some(timestamp);
    }
    Ok(Json(
        json!({"provider":"yahoo", "retrieval":"restricted_server_relay", "source_url":source_url,
        "fetched_at":Utc::now(), "payload":payload}),
    ))
}
