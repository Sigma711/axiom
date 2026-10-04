use axiom::market_tick::{calculate_crypto_tick, CRYPTO_TICK_UNIVERSE};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;

fn trades() -> Vec<(String, Value)> {
    [
        ("BTCUSDT", "100", "101"),
        ("ETHUSDT", "200", "199"),
        ("BNBUSDT", "300", "300"),
    ]
    .into_iter()
    .map(|(symbol, first, second)| {
        (
            symbol.to_string(),
            json!([
                {"a":1,"p":first,"T":1_600_000_098_000_i64},
                {"a":2,"p":second,"T":1_600_000_099_000_i64}
            ]),
        )
    })
    .collect()
}

#[test]
fn fixed_crypto_basket_counts_real_last_trade_directions_at_one_cutoff() {
    let sample_at = Utc.timestamp_millis_opt(1_600_000_100_000).unwrap();
    let result = calculate_crypto_tick(sample_at, &trades()).unwrap();
    assert_eq!(result.universe, CRYPTO_TICK_UNIVERSE);
    assert_eq!(result.up_ticks, 1);
    assert_eq!(result.down_ticks, 1);
    assert_eq!(result.unchanged_ticks, 1);
    assert_eq!(result.net_tick, 0);
    assert_eq!(result.members.len(), 3);
    assert_eq!(result.members[0].latest_price, 101.0);
    assert_eq!(result.members[1].previous_price, 200.0);
    assert_eq!(result.members[2].direction, "unchanged");
}

#[test]
fn incomplete_or_stale_trade_baskets_never_publish_partial_tick() {
    let sample_at = Utc.timestamp_millis_opt(1_600_000_100_000).unwrap();
    let mut missing = trades();
    missing.pop();
    assert!(calculate_crypto_tick(sample_at, &missing).is_err());

    let mut stale = trades();
    stale[1].1[1]["T"] = json!(1_600_000_050_000_i64);
    assert!(calculate_crypto_tick(sample_at, &stale).is_err());

    let mut future = trades();
    future[0].1[1]["T"] = json!(1_600_000_101_000_i64);
    assert!(calculate_crypto_tick(sample_at, &future).is_err());
}

#[test]
fn malformed_price_or_unsorted_trades_are_rejected() {
    let sample_at = Utc.timestamp_millis_opt(1_600_000_100_000).unwrap();
    let mut bad_price = trades();
    bad_price[0].1[1]["p"] = json!("0");
    assert!(calculate_crypto_tick(sample_at, &bad_price).is_err());

    let mut unsorted = trades();
    unsorted[0].1[1]["a"] = json!(0);
    assert!(calculate_crypto_tick(sample_at, &unsorted).is_err());

    let mut duplicate = trades();
    duplicate[1].0 = duplicate[0].0.clone();
    assert!(calculate_crypto_tick(sample_at, &duplicate).is_err());

    let mut too_short = trades();
    too_short[0].1 = json!([{"a":1,"p":"100","T":1_600_000_099_000_i64}]);
    assert!(calculate_crypto_tick(sample_at, &too_short).is_err());
}

#[tokio::test]
async fn public_http_fetch_uses_one_exchange_cutoff_and_preserves_trade_sources() {
    use axum::{extract::Query, http::StatusCode, routing::get, Json, Router};

    async fn exchange_time() -> Json<Value> {
        Json(json!({"serverTime":1_600_000_103_000_i64}))
    }
    async fn aggregate_trades(
        Query(query): Query<HashMap<String, String>>,
    ) -> Result<Json<Value>, StatusCode> {
        if query.get("endTime").map(String::as_str) != Some("1600000100000")
            || query.get("limit").map(String::as_str) != Some("100")
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        trades()
            .into_iter()
            .find(|(symbol, _)| Some(symbol) == query.get("symbol"))
            .map(|(_, rows)| Json(rows))
            .ok_or(StatusCode::NOT_FOUND)
    }
    let app = Router::new()
        .route("/api/v3/time", get(exchange_time))
        .route("/api/v3/aggTrades", get(aggregate_trades));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut feed = axiom::data::HttpFeed::new("target/tick-test-unused-cache");
    feed.base_url = format!("http://{address}");

    let result = axiom::market_tick::fetch_crypto_tick(&feed).await.unwrap();
    assert_eq!(result.sample_at.timestamp_millis(), 1_600_000_100_000);
    assert_eq!(result.net_tick, 0);
    assert_eq!(result.members.len(), 3);
    assert!(result.members[0]
        .source_url
        .contains("symbol=BTCUSDT&endTime=1600000100000&limit=100"));
    server.abort();
}
