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

async fn trade_volume_klines(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    assert_eq!(q.get("interval").map(String::as_str), Some("1h"));
    assert_eq!(q.get("limit").map(String::as_str), Some("25"));
    let start = q["startTime"].parse::<i64>().unwrap();
    let symbol = q["symbol"].as_str();
    Json(Value::Array(
        (1..=25)
            .map(|index| {
                let hours = index + usize::from(symbol == "GAPUSDT" && index >= 2);
                let open = start + hours as i64 * 3_600_000;
                let mut row = json!([
                    open,
                    "100",
                    "120",
                    "90",
                    "110",
                    "10",
                    open + 3_600_000 - 1,
                    if index == 24 { "1017" } else { "1000" }
                ]);
                if symbol == "MISSINGUSDT" && index == 1 {
                    row.as_array_mut().unwrap().truncate(7);
                }
                if symbol == "MISMATCHUSDT" && index == 1 {
                    row[7] = json!("0");
                }
                if symbol == "ZEROUSDT" && index == 1 {
                    row[5] = json!("0");
                    row[7] = json!("0");
                }
                if symbol == "BADOHLCUSDT" && index == 1 {
                    row[2] = json!("99");
                }
                row
            })
            .collect(),
    ))
}

async fn trade_volume_server_time() -> Json<Value> {
    Json(json!({"serverTime": Utc::now().timestamp_millis()}))
}

async fn trade_volume_feed() -> (HttpFeed, tokio::task::JoinHandle<()>, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/api/v3/klines", get(trade_volume_klines))
                .route("/api/v3/time", get(trade_volume_server_time)),
        )
        .await
        .unwrap()
    });
    let dir = format!("target/test-trade-volume-{}", uuid::Uuid::new_v4());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    (feed, server, dir)
}

#[tokio::test]
async fn trade_volume_fetch_uses_field_seven_and_excludes_the_unfinished_row() {
    let (feed, server, dir) = trade_volume_feed().await;
    let bars = feed
        .fetch_completed_binance_trade_bars("BTCUSDT")
        .await
        .unwrap();
    assert_eq!(bars.len(), 24);
    assert_eq!(bars.last().unwrap().quote_volume, 1_017.0);
    assert_ne!(
        bars.last().unwrap().quote_volume,
        bars.last().unwrap().bar.close * 10.0
    );
    assert!(bars
        .windows(2)
        .all(|pair| pair[0].bar.timestamp < pair[1].bar.timestamp));
    assert!(feed
        .fetch_completed_binance_trade_bars("MISSINGUSDT")
        .await
        .unwrap_err()
        .to_string()
        .contains("quote asset volume"));
    assert!(feed
        .fetch_completed_binance_trade_bars("MISMATCHUSDT")
        .await
        .unwrap_err()
        .to_string()
        .contains("base and quote volumes disagree"));
    let zero_bars = feed
        .fetch_completed_binance_trade_bars("ZEROUSDT")
        .await
        .unwrap();
    assert!(zero_bars
        .iter()
        .any(|bar| bar.bar.volume == 0.0 && bar.quote_volume == 0.0));
    assert!(feed
        .fetch_completed_binance_trade_bars("BADOHLCUSDT")
        .await
        .unwrap_err()
        .to_string()
        .contains("invalid Binance trade-volume OHLCV"));
    assert!(feed
        .fetch_completed_binance_trade_bars("GAPUSDT")
        .await
        .unwrap_err()
        .to_string()
        .contains("not continuous"));
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn trade_volume_refuses_an_unavailable_binance_time_endpoint() {
    let (feed, server, dir) = start_mock_feed().await;
    assert!(feed
        .fetch_completed_binance_trade_bars("BTCUSDT")
        .await
        .unwrap_err()
        .to_string()
        .contains("server time"));
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn trade_volume_keeps_the_pre_request_exchange_cutoff_across_an_hour_boundary() {
    let at = |hour, minute, second| {
        Utc.with_ymd_and_hms(2024, 1, 1, hour, minute, second)
            .unwrap()
            .timestamp_millis()
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let times = Arc::new(vec![at(10, 59, 59), at(11, 0, 1)]);
    let index = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/api/v3/klines", get(trade_volume_klines))
                .route("/api/v3/time", get(scripted_binance_time))
                .with_state((times, index)),
        )
        .await
        .unwrap()
    });
    let dir = format!("target/test-trade-rollover-{}", uuid::Uuid::new_v4());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    let bars = feed
        .fetch_completed_binance_trade_bars("BTCUSDT")
        .await
        .unwrap();
    assert_eq!(bars.len(), 24);
    assert_eq!(
        bars.last().unwrap().bar.timestamp,
        Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap()
    );
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}
