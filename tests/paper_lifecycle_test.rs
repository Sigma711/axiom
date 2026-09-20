use axiom::types::Bar;
use axiom::{
    data::{AsyncDataFeed, DataFeed, SyntheticFeed},
    paper::{run_paper_loop, PaperConfigP, PaperState},
    strategy::{BuyAndHoldStrategy, RandomStrategy},
    BacktestEngine, EngineConfig, RiskConfig,
};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

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
