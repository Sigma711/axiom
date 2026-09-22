use axiom::types::Bar;
use axiom::{
    data::{AsyncDataFeed, DataFeed, HttpFeed, SyntheticFeed},
    paper::{run_market_paper_loop, run_paper_loop, PaperConfigP, PaperState},
    strategy::{BuyAndHoldStrategy, RandomStrategy},
    BacktestEngine, EngineConfig, RiskConfig,
};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

fn bar(hour: i64, open: f64, close: f64) -> Bar {
    Bar {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
            + chrono::Duration::hours(hour),
        open,
        high: open.max(close) + 1.0,
        low: open.min(close) - 1.0,
        close,
        volume: 1.0,
    }
}

#[test]
fn paper_and_backtest_share_identical_execution_and_accounting() {
    let bars = SyntheticFeed::new(17)
        .fetch_historical("X", Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(), 100)
        .unwrap();
    let mut paper = PaperState::new(
        PaperConfigP {
            symbol: "X".into(),
            ..Default::default()
        },
        Box::new(RandomStrategy::new(9, 0.2, 0.2)),
    );
    for b in &bars {
        paper.process_bar(*b).unwrap();
        paper.process_bar(*b).unwrap();
    }
    let result = BacktestEngine::new(
        EngineConfig {
            symbol: "X".into(),
            ..Default::default()
        },
        RiskConfig::default(),
    )
    .run(&mut RandomStrategy::new(9, 0.2, 0.2), &bars);
    let snap = paper.snapshot();
    assert_eq!(snap.bars.len(), 100);
    assert_eq!(
        serde_json::to_value(snap.equity_curve).unwrap(),
        serde_json::to_value(result.equity_curve).unwrap()
    );
    assert_eq!(snap.completed_trades_count, result.trades.len());
    assert_eq!(
        snap.trades_count,
        result.trades.len() * 2 + usize::from(!paper.portfolio.is_flat())
    );
}

#[test]
fn snapshot_counts_fills_before_a_trade_is_closed() {
    let bars = SyntheticFeed::new(3)
        .fetch_historical("X", Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(), 2)
        .unwrap();
    let mut paper = PaperState::new(
        PaperConfigP {
            symbol: "X".into(),
            ..Default::default()
        },
        Box::new(BuyAndHoldStrategy::new()),
    );
    paper.process_bar(bars[0]).unwrap(); // schedules the buy
    paper.process_bar(bars[1]).unwrap(); // fills it at the next open
    let snapshot = paper.snapshot();
    assert_eq!(snapshot.trades_count, 1, "one executed buy fill");
    assert_eq!(
        snapshot.completed_trades_count, 0,
        "open position is not closed trade"
    );
}

struct DelayedFeed {
    started: Notify,
    release: Notify,
    bar: Bar,
}
#[async_trait::async_trait]
impl AsyncDataFeed for DelayedFeed {
    async fn fetch_historical_async(
        &self,
        _: &str,
        _: chrono::DateTime<Utc>,
        _: usize,
    ) -> anyhow::Result<Vec<Bar>> {
        unreachable!()
    }
    async fn stream_live_async(&self, _: &str) -> anyhow::Result<Vec<Bar>> {
        self.started.notify_one();
        self.release.notified().await;
        Ok(vec![self.bar])
    }
}
#[tokio::test]
async fn stop_while_market_fetch_is_pending_does_not_process_the_late_response() {
    let bar = SyntheticFeed::default()
        .fetch_historical("X", Utc::now(), 1)
        .unwrap()[0];
    let feed = Arc::new(DelayedFeed {
        started: Notify::new(),
        release: Notify::new(),
        bar,
    });
    let state = Arc::new(RwLock::new(PaperState::new(
        PaperConfigP::default(),
        Box::new(BuyAndHoldStrategy::new()),
    )));
    state.write().await.set_running(true);
    let worker = tokio::spawn(run_paper_loop(feed.clone(), state.clone()));
    feed.started.notified().await;
    state.write().await.set_running(false);
    feed.release.notify_one();
    tokio::task::yield_now().await;
    assert!(state.read().await.snapshot().current_bar.is_none());
    worker.abort();
}

#[test]
fn paper_state_switches_strategy_and_keeps_a_bounded_newest_first_audit_log() {
    use axiom::paper::PaperLogLevel;
    let mut paper = PaperState::new(PaperConfigP::default(), Box::new(BuyAndHoldStrategy::new()));
    assert!(!paper.snapshot().is_running);
    paper.set_running(true);
    paper.set_running(true);
    paper.replace_strategy(Box::new(RandomStrategy::new(7, 0.1, 0.2)));
    assert_eq!(paper.snapshot().strategy, "random");
    for level in [
        PaperLogLevel::Info,
        PaperLogLevel::Warn,
        PaperLogLevel::Error,
        PaperLogLevel::Fill,
    ] {
        paper.log(level, level.as_str().to_owned());
    }
    assert_eq!(paper.snapshot().log[0].level, "FILL");
    assert_eq!(paper.snapshot().log[3].level, "INFO");

    for index in 0..1_001 {
        paper.log(PaperLogLevel::Info, format!("entry-{index}"));
    }
    let snapshot = paper.snapshot();
    assert_eq!(snapshot.log.len(), 100);
    assert_eq!(snapshot.log[0].message, "entry-1000");
    assert_eq!(paper.log.len(), 1000);
    assert_eq!(paper.log[0].message, "entry-1");
}

#[tokio::test]
async fn offline_paper_loop_marks_its_snapshot_as_synthetic_and_processes_closed_bars() {
    use axiom::paper::run_offline_paper_loop;
    use tokio::time::{sleep, Duration};

    let state = Arc::new(RwLock::new(PaperState::new(
        PaperConfigP::default(),
        Box::new(BuyAndHoldStrategy::new()),
    )));
    state.write().await.set_running(true);
    let worker = tokio::spawn(run_offline_paper_loop(state.clone()));
    sleep(Duration::from_millis(1100)).await;
    let snapshot = state.read().await.snapshot();
    assert_eq!(snapshot.source, "synthetic");
    assert!(snapshot.current_bar.is_some());
    assert!(!snapshot.bars.is_empty());
    worker.abort();
}

struct OutcomeFeed {
    outcome: Outcome,
}

#[derive(Clone, Copy)]
enum Outcome {
    Empty,
    Error,
    Invalid,
}

#[async_trait::async_trait]
impl AsyncDataFeed for OutcomeFeed {
    async fn fetch_historical_async(
        &self,
        _: &str,
        _: chrono::DateTime<Utc>,
        _: usize,
    ) -> anyhow::Result<Vec<Bar>> {
        unreachable!()
    }

    async fn stream_live_async(&self, _: &str) -> anyhow::Result<Vec<Bar>> {
        match self.outcome {
            Outcome::Empty => Ok(vec![]),
            Outcome::Error => Err(anyhow::anyhow!("feed failed")),
            Outcome::Invalid => Ok(vec![Bar {
                timestamp: Utc::now(),
                open: f64::NAN,
                high: 1.0,
                low: 1.0,
                close: 1.0,
                volume: 1.0,
            }]),
        }
    }
}

#[tokio::test]
async fn paper_loop_handles_empty_failed_and_invalid_feed_batches() {
    for outcome in [Outcome::Empty, Outcome::Error, Outcome::Invalid] {
        let state = Arc::new(RwLock::new(PaperState::new(
            PaperConfigP {
                poll_interval_seconds: 1,
                ..Default::default()
            },
            Box::new(BuyAndHoldStrategy::new()),
        )));
        state.write().await.set_running(true);
        let worker = tokio::spawn(run_paper_loop(
            Arc::new(OutcomeFeed { outcome }),
            state.clone(),
        ));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let snapshot = state.read().await.snapshot();
        if matches!(outcome, Outcome::Error | Outcome::Invalid) {
            assert!(!snapshot.log.is_empty());
        } else {
            assert!(snapshot.current_bar.is_none());
        }
        worker.abort();
    }
}

#[tokio::test]
async fn stopped_paper_loops_wait_without_fetching() {
    let state = Arc::new(RwLock::new(PaperState::new(
        PaperConfigP::default(),
        Box::new(BuyAndHoldStrategy::new()),
    )));
    let worker = tokio::spawn(run_paper_loop(
        Arc::new(OutcomeFeed {
            outcome: Outcome::Error,
        }),
        state.clone(),
    ));
    tokio::task::yield_now().await;
    assert!(state.read().await.snapshot().current_bar.is_none());
    worker.abort();

    let state = Arc::new(RwLock::new(PaperState::new(
        PaperConfigP::default(),
        Box::new(BuyAndHoldStrategy::new()),
    )));
    let worker = tokio::spawn(run_market_paper_loop(
        Arc::new(HttpFeed::new("target/paper-stopped")),
        state.clone(),
    ));
    tokio::task::yield_now().await;
    assert!(state.read().await.snapshot().current_bar.is_none());
    worker.abort();
}

#[test]
fn paper_force_closes_on_stop_loss_and_logs_risk_rejections() {
    let mut paper = PaperState::new(
        PaperConfigP {
            symbol: "X".into(),
            commission_rate: 0.0,
            slippage_rate: 0.0,
            risk: RiskConfig {
                stop_loss_pct: 0.05,
                ..RiskConfig::default()
            },
            ..Default::default()
        },
        Box::new(BuyAndHoldStrategy::new()),
    );
    paper.process_bar(bar(0, 100.0, 100.0)).unwrap();
    paper.process_bar(bar(1, 100.0, 100.0)).unwrap();
    paper.process_bar(bar(2, 90.0, 90.0)).unwrap();
    assert_eq!(paper.snapshot().completed_trades_count, 1);
    assert!(paper
        .snapshot()
        .log
        .iter()
        .any(|entry| entry.message.contains("止损")));

    let mut rejected = PaperState::new(
        PaperConfigP {
            symbol: "X".into(),
            risk: RiskConfig {
                max_position_pct: 0.0,
                ..RiskConfig::default()
            },
            ..Default::default()
        },
        Box::new(BuyAndHoldStrategy::new()),
    );
    // Keep the portfolio sizing cap high enough to create an order while the
    // independent risk cap rejects it.
    rejected.portfolio.config.max_position_pct = 1.0;
    rejected.process_bar(bar(0, 100.0, 100.0)).unwrap();
    rejected.process_bar(bar(1, 100.0, 100.0)).unwrap();
    assert!(rejected
        .snapshot()
        .log
        .iter()
        .any(|entry| entry.message.contains("风控拒绝")));
}

#[test]
fn paper_keeps_only_the_newest_thousand_closed_bars() {
    let mut paper = PaperState::new(PaperConfigP::default(), Box::new(BuyAndHoldStrategy::new()));
    for hour in 0..1_001 {
        paper.process_bar(bar(hour, 100.0, 100.0)).unwrap();
    }
    let snapshot = paper.snapshot();
    assert_eq!(snapshot.bars.len(), 1_000);
    assert_eq!(snapshot.bars[0].timestamp, bar(1, 100.0, 100.0).timestamp);
}

#[tokio::test]
async fn market_paper_loop_logs_invalid_source_errors() {
    let state = Arc::new(RwLock::new(PaperState::new(
        PaperConfigP {
            poll_interval_seconds: 1,
            ..Default::default()
        },
        Box::new(BuyAndHoldStrategy::new()),
    )));
    state.write().await.reconfigure_market(
        "unsupported".into(),
        "X".into(),
        Box::new(BuyAndHoldStrategy::new()),
    );
    state.write().await.set_running(true);
    let feed = Arc::new(HttpFeed::new("target/paper-invalid-source"));
    let worker = tokio::spawn(run_market_paper_loop(feed, state.clone()));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(state
        .read()
        .await
        .snapshot()
        .log
        .iter()
        .any(|entry| entry.message.contains("行情失败")));
    worker.abort();
}

#[tokio::test]
async fn market_paper_loop_replays_completed_cached_binance_bars() {
    let cache = format!("target/paper-cached-{}", uuid::Uuid::new_v4());
    let feed = Arc::new(HttpFeed::new(&cache));
    let now = Utc::now();
    // Use a current, strictly ordered completed cache so the market loop can
    // exercise its cache-backed success path without external network I/O.
    let bars: Vec<_> = (0..8)
        .map(|i| bar(0, 100.0 + i as f64, 100.0 + i as f64))
        .enumerate()
        .map(|(i, mut b)| {
            b.timestamp = now - chrono::Duration::hours((8 - i) as i64);
            b
        })
        .collect();
    feed.csv.save("X", "1h", &bars).unwrap();
    let state = Arc::new(RwLock::new(PaperState::new(
        PaperConfigP {
            poll_interval_seconds: 1,
            ..Default::default()
        },
        Box::new(BuyAndHoldStrategy::new()),
    )));
    state.write().await.reconfigure_market(
        "binance".into(),
        "X".into(),
        Box::new(BuyAndHoldStrategy::new()),
    );
    state.write().await.set_running(true);
    let worker = tokio::spawn(run_market_paper_loop(feed, state.clone()));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!state.read().await.snapshot().bars.is_empty());
    worker.abort();
    std::fs::remove_dir_all(cache).unwrap();
}
