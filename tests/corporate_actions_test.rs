use axiom::corporate_actions::{HistoricalPriceBasis, SourceBackedStockSplit, StockSplitSchedule};
use axiom::data::{StockAdjustmentEvidence, StockAdjustmentObservation, StockSplitEvent};
use axiom::execution::ExecutionProfile;
use axiom::paper::{PaperConfigP, PaperState};
use axiom::strategy::Strategy;
use axiom::types::{Bar, Position, Side, Signal};
use axiom::{BacktestEngine, EngineConfig, RiskConfig};
use chrono::{TimeZone, Utc};
use std::collections::HashMap;

fn evidence(numerator: f64, denominator: f64) -> StockAdjustmentEvidence {
    let effective_at = Utc.with_ymd_and_hms(2020, 8, 31, 4, 0, 0).unwrap();
    StockAdjustmentEvidence {
        provider: "yahoo",
        endpoint: "https://query1.finance.yahoo.com/v8/finance/chart/AAPL".into(),
        fetched_at: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
        scope: "aapl_2020_4_for_1_split_historical_window",
        event: StockSplitEvent {
            kind: "split",
            effective_at,
            effective_trading_date: effective_at.date_naive(),
            numerator,
            denominator,
            split_ratio: format!("{numerator}:{denominator}"),
        },
        issuer_confirmation_url:
            "https://www.apple.com/newsroom/2020/07/apple-reports-third-quarter-results/",
        issuer_confirmation:
            "Apple announced a four-for-one split with split-adjusted trading beginning 2020-08-31",
        observations: vec![StockAdjustmentObservation {
            timestamp: effective_at,
            open: 127.58,
            high: 131.0,
            low: 126.0,
            close: 129.04,
            volume: 225_702_700.0,
            adjusted_close: 125.40,
        }],
        quote_basis: "provider_quote_semantics_unverified_for_split_adjustment",
        adjusted_close_basis: "provider_adjusted_close_semantics_unverified_for_total_return",
        calculation: "split_only_price_multiplier = denominator / numerator",
    }
}

#[test]
fn maps_provider_evidence_into_a_source_backed_split() {
    let split = SourceBackedStockSplit::from_evidence("aapl", &evidence(4.0, 1.0)).unwrap();

    assert_eq!(split.symbol(), "AAPL");
    assert_eq!(split.effective_trading_date().to_string(), "2020-08-31");
    assert_eq!(split.ratio(), 4.0);
    assert_eq!(split.source().provider, "yahoo");
    assert!(split.source().endpoint.ends_with("/AAPL"));
    assert!(split.source().issuer_confirmation_url.contains("apple.com"));
    assert!(split.source().issuer_confirmation.contains("four-for-one"));
}

#[test]
fn forward_split_preserves_cost_and_pnl_instead_of_creating_a_trade() {
    let split = SourceBackedStockSplit::from_evidence("AAPL", &evidence(4.0, 1.0)).unwrap();
    let mut position = Position {
        symbol: "AAPL".into(),
        size: 10.0,
        avg_entry_price: 100.0,
        realized_pnl: 12.5,
    };
    let unrealized_before = position.unrealized_pnl(108.0);

    let application = split.apply_to_position(&mut position).unwrap();

    assert_eq!(position.size, 40.0);
    assert_eq!(position.avg_entry_price, 25.0);
    assert_eq!(position.realized_pnl, 12.5);
    assert_eq!(position.unrealized_pnl(27.0), unrealized_before);
    assert_eq!(application.cost_basis_before, 1_000.0);
    assert_eq!(application.cost_basis_after, 1_000.0);
    assert_eq!(application.realized_pnl_change, 0.0);
}

#[test]
fn reverse_split_preserves_a_short_positions_economics() {
    let split = SourceBackedStockSplit::from_evidence("AAPL", &evidence(1.0, 5.0)).unwrap();
    let mut position = Position {
        symbol: "aapl".into(),
        size: -20.0,
        avg_entry_price: 10.0,
        realized_pnl: -3.0,
    };
    let unrealized_before = position.unrealized_pnl(8.0);

    let application = split.apply_to_position(&mut position).unwrap();

    assert_eq!(position.size, -4.0);
    assert_eq!(position.avg_entry_price, 50.0);
    assert_eq!(position.unrealized_pnl(40.0), unrealized_before);
    assert_eq!(application.realized_pnl_change, 0.0);
}

#[test]
fn split_on_a_flat_position_does_not_invent_a_cost_basis() {
    let split = SourceBackedStockSplit::from_evidence("AAPL", &evidence(4.0, 1.0)).unwrap();
    let mut position = Position {
        symbol: "AAPL".into(),
        ..Position::default()
    };

    let application = split.apply_to_position(&mut position).unwrap();

    assert!(position.is_flat());
    assert_eq!(position.avg_entry_price, 0.0);
    assert_eq!(application.cost_basis_before, 0.0);
    assert_eq!(application.cost_basis_after, 0.0);
}

#[test]
fn rejects_unusable_evidence_positions_and_duplicate_events() {
    let mut invalid_ratio = evidence(f64::NAN, 1.0);
    assert!(SourceBackedStockSplit::from_evidence("AAPL", &invalid_ratio).is_err());
    invalid_ratio.event.numerator = 4.0;
    invalid_ratio.provider = "";
    assert!(SourceBackedStockSplit::from_evidence("AAPL", &invalid_ratio).is_err());
    let underflow_ratio = evidence(f64::MIN_POSITIVE, f64::MAX);
    assert!(SourceBackedStockSplit::from_evidence("AAPL", &underflow_ratio).is_err());

    let split = SourceBackedStockSplit::from_evidence("AAPL", &evidence(4.0, 1.0)).unwrap();
    assert_eq!(split.split_ratio(), "4:1");
    assert_eq!(
        split.effective_at().date_naive(),
        split.effective_trading_date()
    );
    assert!(StockSplitSchedule::new(
        vec![split.clone(), split.clone()],
        HistoricalPriceBasis::Raw,
    )
    .is_err());

    let schedule = StockSplitSchedule::new(vec![split.clone()], HistoricalPriceBasis::Raw).unwrap();
    assert_eq!(schedule.events(), std::slice::from_ref(&split));
    let mut wrong_symbol = Position {
        symbol: "MSFT".into(),
        size: 1.0,
        avg_entry_price: 1.0,
        realized_pnl: 0.0,
    };
    assert!(split.apply_to_position(&mut wrong_symbol).is_err());
}

struct BuyOnSecondClose {
    seen: usize,
}

impl Strategy for BuyOnSecondClose {
    fn name(&self) -> &str {
        "buy_on_second_close"
    }

    fn params(&self) -> HashMap<String, f64> {
        HashMap::new()
    }

    fn on_bar(&mut self, bar: &Bar) -> Signal {
        let side = if self.seen == 1 {
            Side::Buy
        } else {
            Side::Hold
        };
        self.seen += 1;
        Signal {
            timestamp: bar.timestamp,
            side,
            strength: 1.0,
            reason: "buy across reverse split".into(),
            target_size: (side == Side::Buy).then_some(1_100.0),
        }
    }

    fn reset(&mut self) {
        self.seen = 0;
    }
}

fn reverse_split_schedule(symbol: &str) -> StockSplitSchedule {
    StockSplitSchedule::new(
        vec![SourceBackedStockSplit::from_evidence(symbol, &evidence(10.0, 11.0)).unwrap()],
        HistoricalPriceBasis::Raw,
    )
    .unwrap()
}

#[test]
fn split_date_reference_price_does_not_masquerade_as_a_price_limit() {
    let bars = [(27, 10.0), (28, 10.0), (31, 11.0)].map(|(day, price)| Bar {
        timestamp: Utc.with_ymd_and_hms(2020, 8, day, 0, 0, 0).unwrap(),
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 1_000.0,
    });
    let profile = ExecutionProfile::for_market("a_share", "600000");
    let config = EngineConfig {
        symbol: "600000".into(),
        initial_capital: 20_000.0,
        commission_rate: 0.0,
        slippage_rate: 0.0,
    };
    let result = BacktestEngine::new(
        config.clone(),
        RiskConfig {
            max_position_pct: 1.0,
            ..RiskConfig::default()
        },
    )
    .with_execution_profile(profile.clone())
    .with_stock_splits(reverse_split_schedule("600000"))
    .run(&mut BuyOnSecondClose { seen: 0 }, &bars);
    assert_eq!(result.fills.len(), 1);
    assert_eq!(result.fills[0].size, 1_000.0);

    let mut paper = PaperState::new_with_execution_profile(
        PaperConfigP {
            symbol: config.symbol,
            initial_capital: config.initial_capital,
            commission_rate: 0.0,
            slippage_rate: 0.0,
            risk: RiskConfig {
                max_position_pct: 1.0,
                ..RiskConfig::default()
            },
            ..PaperConfigP::default()
        },
        Box::new(BuyOnSecondClose { seen: 0 }),
        profile,
    )
    .with_stock_splits(reverse_split_schedule("600000"));
    for bar in bars {
        paper.process_bar(bar).unwrap();
    }
    assert_eq!(paper.snapshot().last_fill.unwrap().size, 1_000.0);
}

#[test]
fn schedule_maps_non_trading_day_events_to_the_next_observed_session_once() {
    let mut weekend = evidence(2.0, 1.0);
    weekend.event.effective_at = Utc.with_ymd_and_hms(2024, 6, 8, 12, 0, 0).unwrap();
    weekend.event.effective_trading_date = weekend.event.effective_at.date_naive();
    weekend.event.split_ratio = "2:1".into();
    let schedule = StockSplitSchedule::new(
        vec![SourceBackedStockSplit::from_evidence("AAPL", &weekend).unwrap()],
        HistoricalPriceBasis::Raw,
    )
    .unwrap();

    let friday = Utc.with_ymd_and_hms(2024, 6, 7, 20, 0, 0).unwrap();
    let monday = Utc.with_ymd_and_hms(2024, 6, 10, 13, 30, 0).unwrap();
    assert_eq!(schedule.due_between("AAPL", Some(friday), monday).len(), 1);
    assert!(schedule
        .due_between("AAPL", Some(monday), monday)
        .is_empty());
}

#[test]
fn adjusted_or_unverified_prices_cannot_double_apply_explicit_splits() {
    let split = SourceBackedStockSplit::from_evidence("AAPL", &evidence(4.0, 1.0)).unwrap();

    assert!(
        StockSplitSchedule::new(vec![split.clone()], HistoricalPriceBasis::SplitAdjusted,).is_err()
    );
    assert!(StockSplitSchedule::new(vec![split], HistoricalPriceBasis::Unverified).is_err());
    assert!(StockSplitSchedule::empty().events().is_empty());
}

struct TargetedRoundTrip {
    seen: usize,
}

impl TargetedRoundTrip {
    fn new() -> Self {
        Self { seen: 0 }
    }
}

impl Strategy for TargetedRoundTrip {
    fn name(&self) -> &str {
        "targeted_round_trip"
    }

    fn params(&self) -> HashMap<String, f64> {
        HashMap::new()
    }

    fn on_bar(&mut self, bar: &Bar) -> Signal {
        let (side, target_size) = match self.seen {
            0 => (Side::Buy, Some(10.0)),
            1 => (Side::Sell, Some(10.0)),
            _ => (Side::Hold, None),
        };
        self.seen += 1;
        Signal {
            timestamp: bar.timestamp,
            side,
            strength: 1.0,
            reason: "corporate-action accounting test".into(),
            target_size,
        }
    }

    fn reset(&mut self) {
        self.seen = 0;
    }
}

fn raw_split_bars() -> Vec<Bar> {
    [(27, 100.0), (28, 100.0), (31, 25.0)]
        .into_iter()
        .map(|(day, price)| Bar {
            timestamp: Utc.with_ymd_and_hms(2020, 8, day, 0, 0, 0).unwrap(),
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 1_000.0,
        })
        .collect()
}

fn raw_aapl_split_schedule() -> StockSplitSchedule {
    StockSplitSchedule::new(
        vec![SourceBackedStockSplit::from_evidence("AAPL", &evidence(4.0, 1.0)).unwrap()],
        HistoricalPriceBasis::Raw,
    )
    .unwrap()
}

#[test]
fn backtest_and_paper_apply_the_same_source_backed_split_without_fake_pnl() {
    let bars = raw_split_bars();
    let profile = ExecutionProfile::for_market("us_stock", "AAPL");
    let result = BacktestEngine::new(
        EngineConfig {
            symbol: "AAPL".into(),
            initial_capital: 10_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
        },
        RiskConfig::default(),
    )
    .with_execution_profile(profile.clone())
    .with_stock_splits(raw_aapl_split_schedule())
    .run(&mut TargetedRoundTrip::new(), &bars);

    assert_eq!(result.fills.len(), 2);
    assert_eq!(result.fills[0].size, 10.0);
    assert_eq!(result.fills[1].size, 40.0);
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].size, 40.0);
    assert_eq!(result.trades[0].entry_price, 25.0);
    assert_eq!(result.trades[0].pnl(), 0.0);
    assert_eq!(result.final_equity(), 10_000.0);
    assert_eq!(
        result.config["corporate_actions"]["price_basis"],
        "caller_verified_raw"
    );
    assert_eq!(
        result.config["corporate_actions"]["stock_splits"][0]["source"]["provider"],
        "yahoo"
    );

    let mut paper = PaperState::new_with_execution_profile(
        PaperConfigP {
            symbol: "AAPL".into(),
            initial_capital: 10_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
            ..Default::default()
        },
        Box::new(TargetedRoundTrip::new()),
        profile,
    )
    .with_stock_splits(raw_aapl_split_schedule());
    for bar in &bars {
        paper.process_bar(*bar).unwrap();
    }
    let snapshot = paper.snapshot();
    assert_eq!(snapshot.position_size, 0.0);
    assert_eq!(snapshot.completed_trades_count, 1);
    assert_eq!(snapshot.equity, 10_000.0);
    assert_eq!(snapshot.stock_splits.len(), 1);
    assert!(snapshot
        .log
        .iter()
        .any(|entry| entry.message.contains("AAPL 2020-08-31")));
    assert_eq!(
        serde_json::to_value(snapshot.equity_curve).unwrap(),
        serde_json::to_value(result.equity_curve).unwrap()
    );
}

#[test]
fn backtest_without_an_explicit_schedule_does_not_claim_raw_prices() {
    let result = BacktestEngine::new(
        EngineConfig {
            symbol: "AAPL".into(),
            ..Default::default()
        },
        RiskConfig::default(),
    )
    .run(&mut TargetedRoundTrip::new(), &raw_split_bars()[..1]);

    assert_eq!(result.config["corporate_actions"]["mode"], "none");
    assert_eq!(
        result.config["corporate_actions"]["price_basis"],
        "not_applicable"
    );
}
