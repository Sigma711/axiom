use axiom::broker::{new_order, Broker, BrokerConfig, SimulatedBroker};
use axiom::engine::{BacktestEngine, EngineConfig};
use axiom::metrics::compute_metrics;
use axiom::paper::{PaperConfigP, PaperState};
use axiom::risk::RiskConfig;
use axiom::strategy::{
    BuyAndHoldStrategy, DonchianBreakoutStrategy, ElderRayStrategy, Strategy, VwapReversionStrategy,
};
use axiom::types::{BacktestResult, Bar, EquityPoint, OrderType, Side};
use chrono::{Duration, TimeZone, Utc};

fn bar(hour: i64, close: f64, high: f64, low: f64, volume: f64) -> Bar {
    Bar {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap() + Duration::hours(hour),
        open: close,
        high,
        low,
        close,
        volume,
    }
}

#[test]
fn paper_state_processes_each_closed_bar_timestamp_once() {
    let mut state = PaperState::new(PaperConfigP::default(), Box::new(BuyAndHoldStrategy::new()));
    let closed_bar = bar(0, 100.0, 101.0, 99.0, 10.0);

    state.process_bar(closed_bar).unwrap();
    let first_cash = state.portfolio.broker.get_cash();
    let first_size = state.portfolio.position().size;
    state.process_bar(closed_bar).unwrap();

    assert_eq!(state.equity_curve.len(), 1);
    assert_eq!(state.portfolio.broker.get_cash(), first_cash);
    assert_eq!(state.portfolio.position().size, first_size);
}

#[test]
fn donchian_uses_prior_channel_for_entry_and_exit_windows() {
    let mut strategy = DonchianBreakoutStrategy::new(3, 2);
    for bar in [
        bar(0, 9.0, 10.0, 8.0, 1.0),
        bar(1, 10.0, 11.0, 9.0, 1.0),
        bar(2, 11.0, 12.0, 10.0, 1.0),
    ] {
        assert_eq!(strategy.on_bar(&bar).side, Side::Hold);
    }

    assert_eq!(
        strategy.on_bar(&bar(3, 13.0, 13.0, 12.0, 1.0)).side,
        Side::Buy
    );
    assert_eq!(
        strategy.on_bar(&bar(4, 8.0, 9.0, 8.0, 1.0)).side,
        Side::Sell
    );
}

#[test]
fn vwap_reversion_uses_its_configured_rolling_window() {
    let mut strategy = VwapReversionStrategy::new(2, 5.0);
    assert_eq!(
        strategy.on_bar(&bar(0, 100.0, 100.0, 100.0, 100.0)).side,
        Side::Hold
    );
    assert_eq!(
        strategy.on_bar(&bar(1, 100.0, 100.0, 100.0, 100.0)).side,
        Side::Hold
    );
    assert_eq!(
        strategy.on_bar(&bar(2, 200.0, 200.0, 200.0, 1.0)).side,
        Side::Sell
    );

    // The two-bar VWAP is now exactly 200.  A whole-history VWAP would still
    // be close to 101 and incorrectly keep emitting a sell signal here.
    assert_eq!(
        strategy.on_bar(&bar(3, 200.0, 200.0, 200.0, 1.0)).side,
        Side::Hold
    );
}

#[test]
fn elder_ray_emits_sell_when_entire_bar_is_below_ema() {
    let mut strategy = ElderRayStrategy::new(2);
    assert_eq!(
        strategy.on_bar(&bar(0, 100.0, 101.0, 99.0, 1.0)).side,
        Side::Hold
    );
    assert_eq!(
        strategy.on_bar(&bar(1, 100.0, 101.0, 99.0, 1.0)).side,
        Side::Hold
    );

    assert_eq!(
        // EMA2 = 98 2/3; the high must also be below that EMA.
        strategy.on_bar(&bar(2, 98.0, 98.5, 97.0, 1.0)).side,
        Side::Sell
    );
}

#[test]
fn metrics_measure_return_from_configured_capital_before_first_trade_cost() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let result = BacktestResult {
        config: serde_json::json!({"initial_capital": 100.0}),
        // The first point already includes a 5-unit entry cost.
        equity_curve: vec![
            EquityPoint {
                timestamp: start,
                cash: 0.0,
                position_value: 95.0,
                equity: 95.0,
            },
            EquityPoint {
                timestamp: start + Duration::hours(1),
                cash: 0.0,
                position_value: 110.0,
                equity: 110.0,
            },
        ],
        trades: vec![],
        fills: vec![],
        signals: vec![],
        metrics: serde_json::Value::Null,
    };

    let metrics = compute_metrics(&result);
    assert_eq!(metrics["初始资金"].as_f64(), Some(100.0));
    assert!((metrics["总收益率"].as_f64().unwrap() - 0.10).abs() < 1e-12);
}

#[test]
fn broker_realized_pnl_includes_prorated_entry_and_exit_commission() {
    let mut broker = SimulatedBroker::new(
        BrokerConfig {
            commission_rate: 0.01,
            slippage_rate: 0.0,
            allow_short: false,
        },
        2_000.0,
    );
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    broker.set_market_price("BTCUSDT", 100.0);
    assert_eq!(
        broker
            .place_order(new_order(
                "BTCUSDT",
                Side::Buy,
                10.0,
                start,
                OrderType::Market,
                None
            ))
            .size,
        10.0
    );
    broker.set_market_price("BTCUSDT", 110.0);
    assert_eq!(
        broker
            .place_order(new_order(
                "BTCUSDT",
                Side::Sell,
                4.0,
                start + Duration::hours(1),
                OrderType::Market,
                None
            ))
            .size,
        4.0
    );

    // Gross profit 40, allocated entry fee 4, exit fee 4.4.
    assert!((broker.get_position("BTCUSDT").realized_pnl - 31.6).abs() < 1e-9);
    // Six units remain with 54 of unrealized PnL. Total equity includes it.
    assert!((broker.get_cash() - 1425.6).abs() < 1e-9);
    assert!(
        (broker.get_cash() + broker.get_position("BTCUSDT").market_value(110.0) - 2085.6).abs()
            < 1e-9
    );
}

#[test]
fn backtest_executes_close_signal_at_next_bar_open_and_leaves_last_signal_unfilled() {
    let mut strategy = BuyAndHoldStrategy::new();
    let engine = BacktestEngine::new(
        EngineConfig {
            symbol: "BTCUSDT".into(),
            initial_capital: 1_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
        },
        RiskConfig {
            max_position_pct: 0.5,
            ..RiskConfig::default()
        },
    );
    let result = engine.run(
        &mut strategy,
        &[
            Bar {
                open: 10.0,
                ..bar(0, 100.0, 101.0, 99.0, 1.0)
            },
            Bar {
                open: 200.0,
                ..bar(1, 210.0, 211.0, 199.0, 1.0)
            },
        ],
    );

    assert_eq!(result.signals[0].side, Side::Buy);
    assert_eq!(result.fills.len(), 1);
    assert_eq!(result.fills[0].timestamp, result.equity_curve[1].timestamp);
    assert!((result.fills[0].price - 200.0).abs() < 1e-12);
    assert!((result.fills[0].size - 2.5).abs() < 1e-12);

    let last_only = engine.run(
        &mut strategy,
        &[Bar {
            open: 10.0,
            ..bar(2, 100.0, 101.0, 99.0, 1.0)
        }],
    );
    assert_eq!(last_only.signals[0].side, Side::Buy);
    assert!(last_only.fills.is_empty());
}
