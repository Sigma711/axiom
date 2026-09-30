use axiom::broker::{new_order, Broker, BrokerConfig, SimulatedBroker};
use axiom::execution::{ChinaBoard, ExecutionMarket, ExecutionProfile};
use axiom::paper::{PaperConfigP, PaperState};
use axiom::strategy::Strategy;
use axiom::types::{Bar, OrderType, Position, Side, Signal};
use axiom::{BacktestEngine, EngineConfig, Portfolio, PortfolioConfig, RiskConfig};
use chrono::{TimeZone, Utc};

fn ts(day: u32, hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, day, hour, 0, 0).unwrap()
}

fn profile(market: ExecutionMarket) -> ExecutionProfile {
    ExecutionProfile::new(market)
}

#[test]
fn buy_sizing_obeys_each_market_quantity_rule() {
    let main = profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth));
    assert_eq!(main.buy_quantity(999.9), Some(900.0));
    assert_eq!(main.buy_quantity(99.0), None);

    let star = profile(ExecutionMarket::ChinaA(ChinaBoard::Star));
    assert_eq!(star.buy_quantity(201.9), Some(201.0));
    assert_eq!(star.buy_quantity(199.9), None);

    let bse = profile(ExecutionMarket::ChinaA(ChinaBoard::Bse));
    assert_eq!(bse.buy_quantity(101.9), Some(101.0));
    assert_eq!(bse.buy_quantity(99.9), None);

    let us = profile(ExecutionMarket::UsEquity);
    assert_eq!(us.buy_quantity(3.9), Some(3.0));
    assert_eq!(us.buy_quantity(0.9), None);

    let crypto = profile(ExecutionMarket::CryptoSpot);
    assert_eq!(crypto.buy_quantity(0.123), Some(0.123));
}

#[test]
fn source_and_symbol_select_a_stable_execution_profile() {
    assert_eq!(
        ExecutionProfile::for_market("a_share", "920001").market,
        ExecutionMarket::ChinaA(ChinaBoard::Bse)
    );
    assert_eq!(
        ExecutionProfile::for_market("a_share", "430001").market,
        ExecutionMarket::ChinaA(ChinaBoard::Bse)
    );
    assert_eq!(
        ExecutionProfile::for_market("a_share", "688001").market,
        ExecutionMarket::ChinaA(ChinaBoard::Star)
    );
    assert_eq!(
        ExecutionProfile::for_market("a_share", "689009").market,
        ExecutionMarket::Unrestricted
    );
    assert_eq!(
        ExecutionProfile::for_market("a_share", "900901").market,
        ExecutionMarket::Unrestricted
    );
    assert_eq!(
        ExecutionProfile::for_market("us_stock", "AAPL").market,
        ExecutionMarket::UsEquity
    );
    assert_eq!(
        ExecutionProfile::for_market("binance", "BTCUSDT").market,
        ExecutionMarket::CryptoSpot
    );
}

#[test]
fn portfolio_rounds_cash_safe_buys_and_can_exit_integer_odd_lots() {
    let main = profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth));
    let mut portfolio = Portfolio::new_with_execution_profile(
        Box::new(SimulatedBroker::new_with_execution_profile(
            BrokerConfig {
                commission_rate: 0.01,
                slippage_rate: 0.0,
                allow_short: false,
            },
            10_500.0,
            main.clone(),
        )),
        PortfolioConfig {
            max_position_pct: 1.0,
            min_trade_size: 1.0,
            symbol: "600000".into(),
        },
        main,
    );
    portfolio.broker.set_market_price("600000", 100.0);
    let buy = portfolio
        .on_signal(Side::Buy, 100.0, ts(1, 1), 1.0)
        .unwrap();
    assert_eq!(buy.size, 100.0);
    let fill = portfolio.broker.place_order(buy);
    assert_eq!(fill.size, 100.0);
    assert!((portfolio.cash() - 400.0).abs() < 1e-9);

    // A genuine whole-share odd lot can be exited in one order.
    let broker_position = portfolio.broker.get_position("600000");
    assert_eq!(broker_position.size, 100.0);
    // The broker seam validates a complete integer odd-lot exit below.
    let sell = portfolio
        .on_signal(Side::Sell, 100.0, ts(2, 1), 1.0)
        .unwrap();
    assert_eq!(sell.size, 100.0);
}

#[test]
fn a_share_t_plus_one_tracks_lots_in_shanghai_time() {
    let main = profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth));
    let mut broker = SimulatedBroker::new_with_execution_profile(
        BrokerConfig {
            commission_rate: 0.0,
            slippage_rate: 0.0,
            allow_short: false,
        },
        100_000.0,
        main,
    );
    broker.set_market_price("600000", 10.0);

    let first_buy = broker.place_order(new_order(
        "600000",
        Side::Buy,
        100.0,
        ts(1, 15), // 23:00 in Shanghai on Sep 1
        OrderType::Market,
        None,
    ));
    assert_eq!(first_buy.size, 100.0);
    let same_shanghai_day = broker.place_order(new_order(
        "600000",
        Side::Sell,
        100.0,
        ts(1, 15),
        OrderType::Market,
        None,
    ));
    assert_eq!(same_shanghai_day.size, 0.0);

    let next_shanghai_day = broker.place_order(new_order(
        "600000",
        Side::Sell,
        100.0,
        ts(1, 16), // 00:00 in Shanghai on Sep 2
        OrderType::Market,
        None,
    ));
    assert_eq!(next_shanghai_day.size, 100.0);
}

#[test]
fn same_day_top_up_does_not_lock_previously_settled_inventory() {
    let mut broker = SimulatedBroker::new_with_execution_profile(
        BrokerConfig {
            commission_rate: 0.0,
            slippage_rate: 0.0,
            allow_short: false,
        },
        100_000.0,
        profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)),
    );
    broker.set_market_price("600000", 10.0);
    for (day, side, expected) in [
        (1, Side::Buy, 100.0),
        (2, Side::Buy, 100.0),
        (2, Side::Sell, 100.0),
        (2, Side::Sell, 0.0),
    ] {
        let fill = broker.place_order(new_order(
            "600000",
            side,
            100.0,
            ts(day, 1),
            OrderType::Market,
            None,
        ));
        assert_eq!(fill.size, expected);
    }
    assert_eq!(broker.get_position("600000").size, 100.0);
}

#[test]
fn broker_rejects_invalid_units_and_fractional_odd_lot_exits() {
    let main = profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth));
    let mut broker = SimulatedBroker::new_with_execution_profile(
        BrokerConfig {
            commission_rate: 0.0,
            slippage_rate: 0.0,
            allow_short: false,
        },
        100_000.0,
        main,
    );
    broker.set_market_price("600000", 10.0);
    assert_eq!(
        broker
            .place_order(new_order(
                "600000",
                Side::Buy,
                101.0,
                ts(1, 1),
                OrderType::Market,
                None,
            ))
            .size,
        0.0
    );
    broker.positions.insert(
        "600000".into(),
        Position {
            symbol: "600000".into(),
            size: 55.5,
            avg_entry_price: 10.0,
            realized_pnl: 0.0,
        },
    );
    assert_eq!(
        broker
            .place_order(new_order(
                "600000",
                Side::Sell,
                55.5,
                ts(2, 1),
                OrderType::Market,
                None,
            ))
            .size,
        0.0
    );

    broker.positions.get_mut("600000").unwrap().size = 55.0;
    assert_eq!(
        broker
            .place_order(new_order(
                "600000",
                Side::Sell,
                55.0,
                ts(2, 1),
                OrderType::Market,
                None,
            ))
            .size,
        55.0
    );
}

#[test]
fn a_share_rejects_unrepresentable_shanghai_trade_dates_without_panicking() {
    let mut broker = SimulatedBroker::new_with_execution_profile(
        BrokerConfig::default(),
        100_000.0,
        profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)),
    );
    broker.set_market_price("600000", 10.0);
    let fill = broker.place_order(new_order(
        "600000",
        Side::Buy,
        100.0,
        chrono::DateTime::<Utc>::MAX_UTC,
        OrderType::Market,
        None,
    ));
    assert_eq!(fill.size, 0.0);
    assert_eq!(broker.get_cash(), 100_000.0);
}

#[test]
fn assumptions_disclose_material_omissions_without_claiming_dynamic_crypto_filters() {
    let crypto = profile(ExecutionMarket::CryptoSpot).assumptions();
    assert_eq!(crypto[0].id, "crypto_dynamic_filters_unavailable");
    assert!(!crypto[0].simulated);
    assert!(crypto.iter().any(|item| item.id == "taxes_not_simulated"));
    assert!(crypto
        .iter()
        .any(|item| item.id == "price_limits_and_halts_not_simulated"));
    assert!(crypto
        .iter()
        .any(|item| item.id == "pending_signal_executes_next_open"));
    assert!(crypto
        .iter()
        .any(|item| item.id == "open_price_risk_checks_only"));
}

struct BuyThenHold;

impl Strategy for BuyThenHold {
    fn name(&self) -> &str {
        "buy_then_hold"
    }
    fn params(&self) -> std::collections::HashMap<String, f64> {
        std::collections::HashMap::new()
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        Signal {
            timestamp: bar.timestamp,
            side: if bar.timestamp == ts(1, 1) {
                Side::Buy
            } else {
                Side::Hold
            },
            strength: 1.0,
            reason: "test".into(),
            target_size: None,
        }
    }
    fn reset(&mut self) {}
}

struct SizedAshareOrders;

impl Strategy for SizedAshareOrders {
    fn name(&self) -> &str {
        "sized_a_share_orders"
    }
    fn params(&self) -> std::collections::HashMap<String, f64> {
        std::collections::HashMap::new()
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        let (side, target_size) = if bar.timestamp == ts(1, 1) || bar.timestamp == ts(2, 1) {
            (Side::Buy, Some(100.0))
        } else if bar.timestamp == ts(3, 1) || bar.timestamp == ts(3, 2) {
            (Side::Sell, Some(100.0))
        } else if bar.timestamp == ts(3, 3) {
            // Main-board buys must be a multiple of 100 shares.
            (Side::Buy, Some(101.0))
        } else if bar.timestamp == ts(4, 1) {
            (Side::Buy, Some(0.0))
        } else if bar.timestamp == ts(4, 2) {
            (Side::Buy, Some(f64::NAN))
        } else if bar.timestamp == ts(4, 3) {
            (Side::Buy, Some(-100.0))
        } else if bar.timestamp == ts(4, 4) {
            (Side::Sell, Some(100.0))
        } else {
            (Side::Hold, None)
        };
        Signal {
            timestamp: bar.timestamp,
            side,
            strength: 1.0,
            reason: "explicit order quantity".into(),
            target_size,
        }
    }
    fn reset(&mut self) {}
}

fn bar_at(timestamp: chrono::DateTime<Utc>, open: f64, close: f64) -> Bar {
    Bar {
        timestamp,
        open,
        high: open.max(close),
        low: open.min(close),
        close,
        volume: 1.0,
    }
}

#[test]
fn backtest_applies_profile_to_sizing_t_plus_one_and_result_assumptions() {
    let engine = BacktestEngine::new(
        EngineConfig {
            symbol: "600000".into(),
            initial_capital: 10_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
        },
        RiskConfig {
            stop_loss_pct: 0.05,
            max_position_pct: 1.0,
            ..RiskConfig::default()
        },
    )
    .with_execution_profile(profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)));
    let result = engine.run(
        &mut BuyThenHold,
        &[
            bar_at(ts(1, 1), 10.0, 10.0),
            bar_at(ts(1, 2), 10.0, 10.0),
            bar_at(ts(1, 3), 9.0, 9.0),
            bar_at(ts(1, 16), 9.0, 9.0),
        ],
    );
    assert_eq!(result.fills.len(), 2);
    assert_eq!(result.fills[0].size, 1_000.0);
    assert_eq!(result.fills[1].timestamp, ts(1, 16));
    assert_eq!(result.config["execution_profile"]["market"], "china_a");
    assert!(result.config["execution_assumptions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == "china_a_t_plus_one_sellable_inventory"));
}

#[test]
fn backtest_executes_explicit_add_on_and_partial_sell_quantities() {
    let engine = BacktestEngine::new(
        EngineConfig {
            symbol: "600000".into(),
            initial_capital: 10_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
        },
        RiskConfig {
            max_position_pct: 1.0,
            ..RiskConfig::default()
        },
    )
    .with_execution_profile(profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)));

    let result = engine.run(
        &mut SizedAshareOrders,
        &[
            bar_at(ts(1, 1), 10.0, 10.0),
            bar_at(ts(2, 1), 10.0, 10.0),
            bar_at(ts(3, 1), 10.0, 10.0),
            bar_at(ts(3, 2), 10.0, 10.0),
            bar_at(ts(3, 3), 10.0, 10.0),
            bar_at(ts(4, 1), 10.0, 10.0),
            bar_at(ts(4, 2), 10.0, 10.0),
            bar_at(ts(4, 3), 10.0, 10.0),
            bar_at(ts(4, 4), 10.0, 10.0),
            bar_at(ts(4, 5), 10.0, 10.0),
        ],
    );

    assert_eq!(result.fills.len(), 4);
    assert_eq!(
        result
            .fills
            .iter()
            .map(|fill| (fill.timestamp, fill.side, fill.size))
            .collect::<Vec<_>>(),
        vec![
            (ts(2, 1), Side::Buy, 100.0),
            (ts(3, 1), Side::Buy, 100.0),
            (ts(3, 2), Side::Sell, 100.0),
            (ts(4, 5), Side::Sell, 100.0),
        ]
    );
    assert_eq!(result.trades.len(), 2);
    assert!(result.trades.iter().all(|trade| trade.size == 100.0));
    assert_eq!(result.equity_curve.last().unwrap().cash, 10_000.0);
    assert_eq!(result.equity_curve.last().unwrap().position_value, 0.0);
}

#[test]
fn paper_uses_the_same_profile_and_exposes_assumptions() {
    let mut paper = PaperState::new_with_execution_profile(
        PaperConfigP {
            symbol: "AAPL".into(),
            initial_capital: 1_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
            ..PaperConfigP::default()
        },
        Box::new(BuyThenHold),
        profile(ExecutionMarket::UsEquity),
    );
    paper.process_bar(bar_at(ts(1, 1), 300.0, 300.0)).unwrap();
    paper.process_bar(bar_at(ts(1, 2), 300.0, 300.0)).unwrap();
    let snapshot = paper.snapshot();
    assert_eq!(snapshot.last_fill.unwrap().size, 3.0);
    assert_eq!(snapshot.market_provenance.provider, "unknown");
    assert!(snapshot
        .execution_assumptions
        .iter()
        .any(|item| item.id == "us_whole_share_product_default"));
}

#[test]
fn paper_executes_explicit_quantities_with_the_same_market_rules() {
    let mut paper = PaperState::new_with_execution_profile(
        PaperConfigP {
            symbol: "600000".into(),
            initial_capital: 10_000.0,
            commission_rate: 0.0,
            slippage_rate: 0.0,
            risk: RiskConfig {
                max_position_pct: 1.0,
                ..RiskConfig::default()
            },
            ..PaperConfigP::default()
        },
        Box::new(SizedAshareOrders),
        profile(ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)),
    );
    for timestamp in [
        ts(1, 1),
        ts(2, 1),
        ts(3, 1),
        ts(3, 2),
        ts(3, 3),
        ts(4, 1),
        ts(4, 2),
        ts(4, 3),
        ts(4, 4),
        ts(4, 5),
    ] {
        paper.process_bar(bar_at(timestamp, 10.0, 10.0)).unwrap();
    }

    let snapshot = paper.snapshot();
    assert_eq!(snapshot.trades_count, 4);
    assert_eq!(snapshot.completed_trades_count, 2);
    assert_eq!(snapshot.position_size, 0.0);
    assert_eq!(snapshot.cash, 10_000.0);
    assert_eq!(snapshot.last_fill.unwrap().timestamp, ts(4, 5));
    assert!(paper.portfolio.broker.trade_log().is_empty());
}

#[test]
fn portfolio_tracks_added_inventory_and_partial_exits_without_phantom_closure() {
    let mut portfolio = Portfolio::new(
        Box::new(SimulatedBroker::new(BrokerConfig::default(), 10_000.0)),
        PortfolioConfig {
            symbol: "X".into(),
            ..PortfolioConfig::default()
        },
    );
    for fill in [
        axiom::Fill {
            order_id: "buy-1".into(),
            timestamp: ts(1, 1),
            symbol: "X".into(),
            side: Side::Buy,
            size: 100.0,
            price: 10.0,
            commission: 10.0,
        },
        axiom::Fill {
            order_id: "buy-2".into(),
            timestamp: ts(2, 1),
            symbol: "X".into(),
            side: Side::Buy,
            size: 50.0,
            price: 20.0,
            commission: 10.0,
        },
    ] {
        portfolio.on_fill(&fill);
    }
    let open = portfolio.open_trade().unwrap();
    assert_eq!(open.size, 150.0);
    assert!((open.entry_price - 13.333_333_333_333_334).abs() < 1e-12);
    assert_eq!(open.entry_commission, 20.0);

    portfolio.on_fill(&axiom::Fill {
        order_id: "sell-1".into(),
        timestamp: ts(3, 1),
        symbol: "X".into(),
        side: Side::Sell,
        size: 100.0,
        price: 30.0,
        commission: 30.0,
    });
    let closed = &portfolio.closed_trades()[0];
    assert_eq!(closed.size, 100.0);
    assert!((closed.entry_commission - 13.333_333_333_333_334).abs() < 1e-12);
    assert_eq!(closed.exit_commission, 30.0);
    let remaining = portfolio.open_trade().unwrap();
    assert_eq!(remaining.size, 50.0);
    assert!((remaining.entry_commission - 6.666_666_666_666_666).abs() < 1e-12);

    portfolio.on_fill(&axiom::Fill {
        order_id: "sell-2".into(),
        timestamp: ts(4, 1),
        symbol: "X".into(),
        side: Side::Sell,
        size: 50.0,
        price: 40.0,
        commission: 20.0,
    });
    assert!(portfolio.open_trade().is_none());
    assert_eq!(portfolio.closed_trades().len(), 2);
    assert!(
        (portfolio
            .closed_trades()
            .iter()
            .map(|trade| trade.pnl())
            .sum::<f64>()
            - 2_930.0)
            .abs()
            < 1e-9
    );
    assert!(
        (portfolio
            .closed_trades()
            .iter()
            .map(|trade| trade.entry_commission)
            .sum::<f64>()
            - 20.0)
            .abs()
            < 1e-9
    );
}
