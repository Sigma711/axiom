use axiom::data::{AsyncDataFeed, CsvFeed, DataFeed, HttpFeed, SyntheticFeed};
use axiom::paper::{PaperConfigP, PaperLogLevel, PaperState};
use axiom::strategy::{BuyAndHoldStrategy, RandomStrategy};
use axiom::symbols;
use axiom::types::Bar;
use chrono::{Duration, TimeZone, Timelike, Utc};

fn bar(timestamp: chrono::DateTime<Utc>, close: f64) -> Bar {
    Bar {
        timestamp,
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 10.0,
    }
}

#[test]
fn synthetic_and_csv_feeds_expose_completed_bars_through_datafeed() {
    let since = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let synthetic = SyntheticFeed::new(23);
    let live = synthetic.stream_live("BTCUSDT").unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].timestamp.minute(), 0);
    assert_eq!(live[0].timestamp.second(), 0);

    let dir = format!("target/backend-feed-{}", uuid::Uuid::new_v4());
    let csv = CsvFeed::new(&dir);
    csv.save(
        "BTC/USDT",
        "1h",
        &[
            bar(since, 100.0),
            bar(since + Duration::hours(1), 101.0),
            bar(since + Duration::hours(2), 102.0),
        ],
    )
    .unwrap();
    let selected = csv
        .fetch_historical("BTC/USDT", since + Duration::hours(1), 1)
        .unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].close, 101.0);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn public_symbol_search_validates_requests_without_network_side_effects() {
    let invalid = symbols::search("crypto", "", 0, 10).await;
    assert!(invalid.is_err());
    let too_long = symbols::search("a_share", &"x".repeat(65), 0, 10).await;
    assert!(too_long.is_err());
}

#[tokio::test]
async fn http_feed_rejects_invalid_limits_before_remote_fetch() {
    let dir = format!("target/backend-http-{}", uuid::Uuid::new_v4());
    let feed = HttpFeed::new(&dir);
    let since = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    assert!(feed
        .fetch_historical_async("BTCUSDT", since, 0)
        .await
        .is_err());
    assert!(feed
        .fetch_historical_async("BTCUSDT", since, 5001)
        .await
        .is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn paper_reconfigure_resets_account_and_snapshot_keeps_public_state_consistent() {
    let mut paper = PaperState::new(PaperConfigP::default(), Box::new(BuyAndHoldStrategy::new()));
    let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    paper.process_bar(bar(timestamp, 100.0)).unwrap();
    paper.reconfigure_market(
        "us_stock".into(),
        "AAPL".into(),
        Box::new(RandomStrategy::new(7, 0.1, 0.2)),
    );
    let snapshot = paper.snapshot();
    assert_eq!(snapshot.source, "us_stock");
    assert_eq!(snapshot.symbol, "AAPL");
    assert_eq!(snapshot.strategy, "random");
    assert_eq!(snapshot.initial_capital, 10_000.0);
    assert!(snapshot.current_bar.is_none());
    assert!(snapshot.bars.is_empty());
    assert_eq!(snapshot.trades_count, 0);
    paper.log(PaperLogLevel::Error, "feed failed".into());
    assert_eq!(paper.snapshot().log[0].level, "ERROR");
}
