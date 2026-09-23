use axiom::data::{AsyncDataFeed, CsvFeed, HttpFeed};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

async fn endpoint(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    if q.get("symbol").is_some_and(|s| s == "BROKEN") {
        return Json(json!([[0, "100", "99", "101", "100", "10"]]));
    }
    if q.get("symbol").is_some_and(|s| s == "EMPTY") {
        return Json(json!([]));
    }
    let start = q["startTime"].parse::<i64>().unwrap();
    let limit = q["limit"].parse::<usize>().unwrap();
    let count = if q
        .get("symbol")
        .is_some_and(|s| s == "SHORT" || s == "FUTURE")
    {
        1
    } else if q.get("symbol").is_some_and(|s| s == "LIVE") {
        2
    } else {
        limit
    };
    let start = if q.get("symbol").is_some_and(|s| s == "FUTURE") {
        Utc::now().timestamp_millis()
    } else if q.get("symbol").is_some_and(|s| s == "LIVE") {
        Utc::now().timestamp_millis() - 3 * 3600000
    } else {
        start
    };
    let mut rows = Value::Array(
        (0..count)
            .map(|i| {
                json!([
                    start + i as i64 * 3600000,
                    "100",
                    "102",
                    "99",
                    "101",
                    "10",
                    start + (i as i64 + 1) * 3600000 - 1
                ])
            })
            .collect(),
    );
    if q.get("symbol").is_some_and(|s| s == "INVALID_COMPLETED") {
        rows[0][3] = json!("-1");
    }
    if q.get("symbol").is_some_and(|s| s == "INVALID_PROVISIONAL") && count >= 2 {
        rows[1][4] = json!("0");
    }
    Json(rows)
}

async fn start_mock_feed() -> (HttpFeed, tokio::task::JoinHandle<()>, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/v3/klines", get(endpoint)),
        )
        .await
        .unwrap()
    });
    let dir = format!("target/test-feed-{}", uuid::Uuid::new_v4());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    (feed, server, dir)
}

#[tokio::test]
async fn historical_pagination_returns_requested_closed_bars_and_cache_is_reusable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/v3/klines", get(endpoint)),
        )
        .await
        .unwrap()
    });
    let dir = format!("target/test-feed-{}", uuid::Uuid::new_v4());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    let since = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let bars = feed
        .fetch_historical_async("BTCUSDT", since, 1500)
        .await
        .unwrap();
    assert_eq!(bars.len(), 1500);
    assert_eq!(bars[1499].timestamp, since + chrono::Duration::hours(1499));
    assert!(
        feed.fetch_historical_async("BROKEN", since, 1)
            .await
            .is_err(),
        "invalid OHLC may not be silently skipped"
    );
    server.abort();
    let cached = feed
        .fetch_historical_async("BTCUSDT", since, 1500)
        .await
        .unwrap();
    assert_eq!(cached.len(), 1500);
    assert_eq!(cached[0].close, 101.0);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn remote_empty_short_and_unfinished_batches_follow_completion_contract() {
    let (feed, server, dir) = start_mock_feed().await;
    let since = Utc::now() - chrono::Duration::hours(3);
    assert!(feed
        .fetch_historical_async("EMPTY", since, 3)
        .await
        .unwrap()
        .is_empty());
    let short = feed
        .fetch_historical_async("SHORT", since, 3)
        .await
        .unwrap();
    assert_eq!(short.len(), 1);
    assert!(feed.stream_live_async("FUTURE").await.unwrap().is_empty());
    assert_eq!(feed.stream_live_async("LIVE").await.unwrap().len(), 2);
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn malformed_csv_returns_error_instead_of_panicking_and_paths_stay_inside_cache() {
    let dir = format!("target/test-csv-{}", uuid::Uuid::new_v4());
    let feed = CsvFeed::new(&dir);
    std::fs::write(
        format!("{dir}/X_1h.csv"),
        "timestamp,open\n2024-01-01T00:00:00Z,100\n",
    )
    .unwrap();
    assert!(feed.load("X", "1h").is_err());
    let path = feed.save("../../escape", "../../secret", &[]).unwrap();
    assert_eq!(path.parent(), Some(std::path::Path::new(&dir)));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn provisional_binance_snapshot_is_current_hour_and_is_never_a_final_close() {
    let (feed, server, dir) = start_mock_feed().await;
    let as_of = Utc.with_ymd_and_hms(2024, 1, 1, 10, 23, 45).unwrap();
    let pair = feed
        .fetch_open_candle_pair_at("BTCUSDT", as_of)
        .await
        .unwrap();
    assert_eq!(
        pair.provisional.candle.timestamp,
        Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()
    );
    assert_eq!(pair.provisional.candle.open, 100.0);
    assert_eq!(pair.provisional.candle.close, 101.0);
    assert_eq!(pair.provisional.fetched_at, as_of);
    assert_eq!(
        pair.provisional.expected_close_at,
        as_of.date_naive().and_hms_opt(11, 0, 0).unwrap().and_utc()
    );
    assert!(pair.provisional.fetched_at < pair.provisional.expected_close_at);
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn open_candle_pair_rejects_nonpositive_prices_in_either_row() {
    let (feed, server, dir) = start_mock_feed().await;
    let as_of = Utc.with_ymd_and_hms(2024, 1, 1, 10, 23, 45).unwrap();
    for symbol in ["INVALID_COMPLETED", "INVALID_PROVISIONAL"] {
        let error = feed
            .fetch_open_candle_pair_at(symbol, as_of)
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("invalid OHLCV"),
            "{symbol}: {error}"
        );
    }
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

async fn scripted_binance_time(
    State((script, index)): State<(Arc<Vec<i64>>, Arc<AtomicUsize>)>,
) -> Json<Value> {
    let current = index.fetch_add(1, Ordering::SeqCst);
    Json(json!({"serverTime": script[current.min(script.len() - 1)]}))
}

async fn failed_binance_time() -> StatusCode {
    StatusCode::BAD_GATEWAY
}

async fn scripted_open_candle_feed(
    times: Vec<i64>,
) -> (HttpFeed, tokio::task::JoinHandle<()>, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let script = Arc::new(times);
    let index = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/api/v3/klines", get(endpoint))
                .route("/api/v3/time", get(scripted_binance_time))
                .with_state((script, index)),
        )
        .await
        .unwrap();
    });
    let dir = format!("target/test-open-candle-{}", uuid::Uuid::new_v4());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    (feed, server, dir)
}

#[tokio::test]
async fn open_candle_pair_retries_after_hour_rollover_and_keeps_second_snapshot() {
    let at = |hour, minute, second| {
        Utc.with_ymd_and_hms(2024, 1, 1, hour, minute, second)
            .unwrap()
            .timestamp_millis()
    };
    let (feed, server, dir) = scripted_open_candle_feed(vec![
        at(10, 59, 59),
        at(11, 0, 1),
        at(11, 0, 2),
        at(11, 0, 3),
    ])
    .await;
    let pair = feed.fetch_open_candle_pair_async("BTCUSDT").await.unwrap();
    assert_eq!(
        pair.last_completed.timestamp,
        Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()
    );
    assert_eq!(
        pair.provisional.candle.timestamp,
        Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap()
    );
    assert_eq!(
        pair.provisional.fetched_at,
        Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 3).unwrap()
    );
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn open_candle_pair_rejects_when_second_attempt_also_crosses_close() {
    let at = |hour, minute, second| {
        Utc.with_ymd_and_hms(2024, 1, 1, hour, minute, second)
            .unwrap()
            .timestamp_millis()
    };
    let (feed, server, dir) = scripted_open_candle_feed(vec![
        at(10, 59, 59),
        at(11, 0, 1),
        at(11, 59, 59),
        at(12, 0, 1),
    ])
    .await;
    let error = feed
        .fetch_open_candle_pair_async("BTCUSDT")
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("hour changed"));
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn open_candle_pair_rejects_when_exchange_time_service_fails() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/v3/time", get(failed_binance_time)),
        )
        .await
        .unwrap();
    });
    let dir = format!("target/test-open-time-{}", uuid::Uuid::new_v4());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    assert!(feed
        .fetch_open_candle_pair_async("BTCUSDT")
        .await
        .unwrap_err()
        .to_string()
        .contains("server time"));
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}
