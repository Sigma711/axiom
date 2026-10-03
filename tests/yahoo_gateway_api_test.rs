use axiom::yahoo_gateway::{router, GatewayConfig};
use axum::{
    body::{to_bytes, Body},
    extract::{Path, Query},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tower::ServiceExt;

async fn provider(
    Path((mode, symbol)): Path<(String, String)>,
    Query(q): Query<HashMap<String, String>>,
) -> axum::response::Response {
    if mode == "slow" {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(q["interval"], "1d");
    assert_eq!(q["includePrePost"], "false");
    assert_eq!(q["events"], "div,splits");
    if mode == "http_error" {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if mode == "malformed" {
        return "not JSON".into_response();
    }
    if mode == "oversized" {
        return "x".repeat(2_000_001).into_response();
    }
    let mut payload = json!({"chart":{"error":null,"result":[{
        "meta":{"symbol":symbol,"currency":"USD","instrumentType":"EQUITY"},
        "timestamp":[1598880600],"events":{"splits":{"1598880600":{"date":1598880600,"numerator":4,"denominator":1,"splitRatio":"4:1"}}},
        "indicators":{"quote":[{"open":[127.58],"high":[131.0],"low":[126.0],"close":[129.04],"volume":[225702700]}]}
    }]}});
    match mode.as_str() {
        "wrong_symbol" => payload["chart"]["result"][0]["meta"]["symbol"] = json!("OTHER"),
        "wrong_currency" => payload["chart"]["result"][0]["meta"]["currency"] = json!("HKD"),
        "wrong_instrument" => {
            payload["chart"]["result"][0]["meta"]["instrumentType"] = json!("CURRENCY")
        }
        "chart_error" => payload["chart"]["error"] = json!({"code":"Not Found"}),
        "empty_results" => payload["chart"]["result"] = json!([]),
        "multiple_results" => {
            let row = payload["chart"]["result"][0].clone();
            payload["chart"]["result"] = json!([row, row]);
        }
        "empty_timestamps" => payload["chart"]["result"][0]["timestamp"] = json!([]),
        "many_timestamps" => {
            payload["chart"]["result"][0]["timestamp"] = json!(vec![1598880600; 12001])
        }
        "invalid_timestamp" => payload["chart"]["result"][0]["timestamp"] = json!(["bad"]),
        "out_of_window" => payload["chart"]["result"][0]["timestamp"] = json!([1601510400]),
        "unordered" => payload["chart"]["result"][0]["timestamp"] = json!([1598880600, 1598880600]),
        "etf" => payload["chart"]["result"][0]["meta"]["instrumentType"] = json!("ETF"),
        _ => {}
    }
    Json(payload).into_response()
}
async fn read(app: &Router, query: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/yahoo-chart?{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 3_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&body).unwrap_or_else(|_| json!(String::from_utf8_lossy(&body))),
    )
}

#[tokio::test]
async fn restricted_gateway_preserves_provider_identity_and_fails_closed_for_invalid_requests_or_responses(
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/:mode/:symbol", get(provider)),
        )
        .await
        .unwrap();
    });
    let valid = "symbol=AAPL&period1=1595808000&period2=1601510400";
    for (mode, symbol) in [
        ("ok", "AAPL"),
        ("ok", "BRK.B"),
        ("etf", "SPY"),
        ("ok", "SMCI"),
    ] {
        let app = router(GatewayConfig {
            chart_url: format!("{base}/{mode}"),
        });
        let (status, result) = read(&app, &valid.replace("AAPL", symbol)).await;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(result["provider"], "yahoo");
        assert_eq!(result["retrieval"], "restricted_server_relay");
        assert_eq!(
            result["payload"]["chart"]["result"][0]["meta"]["symbol"],
            symbol.replace('.', "-")
        );
        let url = reqwest::Url::parse(result["source_url"].as_str().unwrap()).unwrap();
        let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params["period1"], "1595808000");
        assert_eq!(params["period2"], "1601510400");
        assert_eq!(params["events"], "div,splits");
        assert!(result["fetched_at"].as_str().is_some());
    }
    let app = router(GatewayConfig {
        chart_url: format!("{base}/ok"),
    });
    // The product accepts up to 5,000 trading sessions and requests calendar
    // runway for weekends/holidays. Its provider path must preserve that range.
    assert_eq!(
        read(&app, "symbol=AAPL&period1=473385600&period2=1601510400")
            .await
            .0,
        StatusCode::OK
    );
    for query in [
        "symbol=&period1=0&period2=1",
        "symbol=..&period1=0&period2=1",
        "symbol=A..B&period1=0&period2=1",
        "symbol=aapl&period1=0&period2=1",
        "symbol=http%3A%2F%2Fevil&period1=0&period2=1",
        "symbol=1A&period1=0&period2=1",
        "symbol=AAPL&period1=-1&period2=1",
        "symbol=AAPL&period1=1&period2=1",
        "symbol=AAPL&period1=2&period2=1",
        "symbol=AAPL&period1=0&period2=9223372036854775807",
        "symbol=AAPL&period1=0&period2=1382400001",
        "symbol=AAPL&period1=bad&period2=1",
        "symbol=AAPL&period1=0",
        "symbol=AAPL&period1=0&period2=1&url=http://evil",
        "symbol=AAPL&period1=0&period2=1&interval=1m",
        "symbol=AAPL&period1=0&period2=1&events=none",
    ] {
        assert_eq!(
            read(&app, query).await.0,
            StatusCode::BAD_REQUEST,
            "{query}"
        );
    }
    for mode in [
        "http_error",
        "malformed",
        "oversized",
        "wrong_symbol",
        "wrong_currency",
        "wrong_instrument",
        "chart_error",
        "empty_results",
        "multiple_results",
        "empty_timestamps",
        "many_timestamps",
        "invalid_timestamp",
        "out_of_window",
        "unordered",
    ] {
        let app = router(GatewayConfig {
            chart_url: format!("{base}/{mode}"),
        });
        assert_eq!(read(&app, valid).await.0, StatusCode::BAD_GATEWAY, "{mode}");
    }
    let app = router(GatewayConfig {
        chart_url: "http://127.0.0.1:1/unreachable".into(),
    });
    assert_eq!(read(&app, valid).await.0, StatusCode::BAD_GATEWAY);
    let app = router(GatewayConfig {
        chart_url: format!("{base}/slow"),
    });
    let responses = futures::future::join_all((0..4).map(|_| read(&app, valid))).await;
    assert_eq!(
        responses
            .iter()
            .filter(|(status, _)| *status == StatusCode::SERVICE_UNAVAILABLE)
            .count(),
        1
    );
    assert_eq!(
        responses
            .iter()
            .filter(|(status, _)| *status == StatusCode::OK)
            .count(),
        3
    );
    server.abort();
}
