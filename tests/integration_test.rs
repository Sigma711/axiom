//! 集成测试 —— 跑完整链路,验证核心功能

use axiom::broker::{new_order, Broker, BrokerConfig, SimulatedBroker};
use axiom::data::{DataFeed, SyntheticFeed};
use axiom::engine::{compare_strategies, BacktestEngine, EngineConfig};
use axiom::metrics::compute_metrics;
use axiom::portfolio::{Portfolio, PortfolioConfig};
use axiom::risk::{RiskConfig, RiskManager};
use axiom::strategy::{
    BollingerBandsStrategy, BuyAndHoldStrategy, DonchianBreakoutStrategy, KdjStrategy,
    MacdStrategy, RandomStrategy, RsiStrategy, SmaCrossStrategy, Strategy, SupertrendStrategy,
    VwapReversionStrategy,
};
use axiom::types::{Side, Trade};
use chrono::{TimeZone, Utc};

#[test]
fn test_synthetic_feed_is_deterministic() {
    let f1 = SyntheticFeed::new(42);
    let f2 = SyntheticFeed::new(42);
    let since = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let bars1 = f1.fetch_historical("BTCUSDT", since, 50).unwrap();
    let bars2 = f2.fetch_historical("BTCUSDT", since, 50).unwrap();
    assert_eq!(bars1.len(), 50);
    assert_eq!(bars2.len(), 50);
    for (b1, b2) in bars1.iter().zip(bars2.iter()) {
        assert!((b1.close - b2.close).abs() < 1e-9);
    }
}

#[test]
fn test_broker_market_order_with_commission_and_slippage() {
    let cfg = BrokerConfig {
        commission_rate: 0.001,
        slippage_rate: 0.0005,
        allow_short: false,
    };
    let mut broker = SimulatedBroker::new(cfg, 10000.0);
    broker.set_market_price("BTCUSDT", 100.0);

    let order = new_order(
        "BTCUSDT",
        Side::Buy,
        0.5,
        Utc::now(),
        axiom::types::OrderType::Market,
        None,
    );
    let fill = broker.place_order(order);
    // slippage 让买入价变 100.05
    assert!((fill.price - 100.05).abs() < 1e-6);
    // 手续费 = 100.05 * 0.5 * 0.001 ≈ 0.05
    assert!((fill.commission - 0.05).abs() < 1e-3);
    // 现金减少 = 成交额 + 手续费 = 50.025 + 0.05 ≈ 50.075
    assert!((10000.0 - broker.get_cash() - 50.075).abs() < 0.01);
}

#[test]
fn test_broker_rejects_oversold() {
    let mut broker = SimulatedBroker::new(BrokerConfig::default(), 50.0);
    broker.set_market_price("BTCUSDT", 100.0);
    let order = new_order(
        "BTCUSDT",
        Side::Buy,
        1.0,
        Utc::now(),
        axiom::types::OrderType::Market,
        None,
    );
    let fill = broker.place_order(order);
    assert_eq!(fill.size, 0.0, "现金不足时不应成交");
}

#[test]
fn test_sma_cross_strategy_emits_signals() {
    let mut strat = SmaCrossStrategy::new(3, 10);
    // 先下跌后上涨,触发金叉
    let mut bars = Vec::new();
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    for i in 0..200 {
        let close = if i < 100 {
            // 前 100 根下跌
            200.0 - i as f64
        } else {
            // 后 100 根上涨
            100.0 + (i - 100) as f64
        };
        bars.push(axiom::types::Bar {
            timestamp: start + chrono::Duration::hours(i),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1.0,
        });
    }
    let mut buy_signals = 0;
    let mut sell_signals = 0;
    for bar in &bars {
        let sig = strat.on_bar(bar);
        if sig.side == Side::Buy {
            buy_signals += 1;
        }
        if sig.side == Side::Sell {
            sell_signals += 1;
        }
    }
    assert!(buy_signals >= 1, "先跌后涨应至少有 1 次买入信号");
    assert_eq!(
        sell_signals, 0,
        "先跌后涨只有一次向上穿越，不应误发卖出信号"
    );
}

#[test]
fn test_backtest_engine_end_to_end() {
    let bars = SyntheticFeed::new(42)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            200,
        )
        .unwrap();
    let engine = BacktestEngine::new(EngineConfig::default(), RiskConfig::default());
    let mut strat = BuyAndHoldStrategy::new();
    let mut result = engine.run(&mut strat, &bars);
    result.metrics = compute_metrics(&result);
    assert_eq!(result.equity_curve.len(), 200);
    assert_eq!(result.fills.len(), 1);
    assert!(result.metrics.get("初始资金").is_some());
    assert!(result.metrics.get("最终净值").is_some());
}

#[test]
fn test_compare_strategies_picks_winner() {
    let bars = SyntheticFeed::new(7)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            300,
        )
        .unwrap();
    let strategies = vec![
        (
            "bh".to_string(),
            Box::new(BuyAndHoldStrategy::new()) as Box<dyn axiom::strategy::Strategy>,
        ),
        ("sma".to_string(), Box::new(SmaCrossStrategy::new(5, 20))),
        (
            "rsi".to_string(),
            Box::new(RsiStrategy::new(14, 70.0, 30.0)),
        ),
        (
            "random".to_string(),
            Box::new(RandomStrategy::new(99, 0.05, 0.05)),
        ),
    ];
    let out = compare_strategies(
        strategies,
        &bars,
        EngineConfig::default(),
        RiskConfig::default(),
    );
    assert_eq!(out.results.len(), 4);
    let (best, _) = out.best_by("总收益率").expect("应有最佳策略");
    assert!(["bh", "sma", "rsi", "random"].contains(&best.as_str()));
}

#[test]
fn strategy_and_parameter_changes_produce_distinct_executed_equity_paths() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let bars = (0..160)
        .map(|index| {
            let close = if index < 80 {
                180.0 - index as f64
            } else {
                100.0 + (index - 80) as f64
            };
            axiom::types::Bar {
                timestamp: start + chrono::Duration::hours(index),
                open: close,
                high: close + 1.0,
                low: close - 1.0,
                close,
                volume: 100.0,
            }
        })
        .collect::<Vec<_>>();
    let engine = BacktestEngine::new(EngineConfig::default(), RiskConfig::default());
    let buy_hold = engine.run(&mut BuyAndHoldStrategy::new(), &bars);
    let fast = engine.run(&mut SmaCrossStrategy::new(3, 10), &bars);
    let slow = engine.run(&mut SmaCrossStrategy::new(20, 50), &bars);
    assert_eq!(buy_hold.equity_curve.len(), bars.len());
    assert_eq!(fast.equity_curve.len(), bars.len());
    assert_eq!(slow.equity_curve.len(), bars.len());
    assert_ne!(
        buy_hold
            .equity_curve
            .iter()
            .map(|point| point.equity)
            .collect::<Vec<_>>(),
        fast.equity_curve
            .iter()
            .map(|point| point.equity)
            .collect::<Vec<_>>(),
        "buy-and-hold and signal strategy must use distinct fills"
    );
    assert_ne!(
        fast.equity_curve
            .iter()
            .map(|point| point.equity)
            .collect::<Vec<_>>(),
        slow.equity_curve
            .iter()
            .map(|point| point.equity)
            .collect::<Vec<_>>(),
        "changing the moving-average periods must change executed results on a known reversal"
    );
    assert_ne!(
        fast.fills.first().map(|fill| fill.timestamp),
        slow.fills.first().map(|fill| fill.timestamp)
    );
}

#[test]
fn test_macd_strategy_runs() {
    let bars = SyntheticFeed::new(42)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            200,
        )
        .unwrap();
    let engine = BacktestEngine::new(EngineConfig::default(), RiskConfig::default());
    let mut strat = MacdStrategy::new(12, 26, 9);
    let result = engine.run(&mut strat, &bars);
    assert_eq!(result.equity_curve.len(), 200);
}

#[test]
fn test_bollinger_strategy_runs() {
    let bars = SyntheticFeed::new(42)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            200,
        )
        .unwrap();
    let engine = BacktestEngine::new(EngineConfig::default(), RiskConfig::default());
    let mut strat = BollingerBandsStrategy::new(20, 2.0);
    let result = engine.run(&mut strat, &bars);
    assert_eq!(result.equity_curve.len(), 200);
}

#[test]
fn test_all_new_strategies() {
    let bars = SyntheticFeed::new(99)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            200,
        )
        .unwrap();
    let engine = BacktestEngine::new(EngineConfig::default(), RiskConfig::default());
    let strategies: Vec<(&str, Box<dyn Strategy>)> = vec![
        ("macd", Box::new(MacdStrategy::new(12, 26, 9))),
        ("boll", Box::new(BollingerBandsStrategy::new(20, 2.0))),
        ("st", Box::new(SupertrendStrategy::new(10, 3.0))),
        ("don", Box::new(DonchianBreakoutStrategy::new(20, 10))),
        ("vwap", Box::new(VwapReversionStrategy::new(20, 1.5))),
        ("kdj", Box::new(KdjStrategy::new(9, 3, 3))),
    ];
    for (name, mut strat) in strategies {
        let result = engine.run(strat.as_mut(), &bars);
        assert_eq!(result.equity_curve.len(), 200, "{} 净值曲线长度错", name);
    }
}

#[test]
fn test_risk_manager_stop_loss() {
    let mut broker = SimulatedBroker::new(
        BrokerConfig {
            commission_rate: 0.0,
            slippage_rate: 0.0,
            allow_short: false,
        },
        10000.0,
    );
    broker.set_market_price("BTCUSDT", 100.0);
    let fill = broker.place_order(new_order(
        "BTCUSDT",
        Side::Buy,
        10.0,
        Utc::now(),
        axiom::types::OrderType::Market,
        None,
    ));
    assert_eq!(fill.size, 10.0);
    let mut pf = Portfolio::new(
        Box::new(broker),
        PortfolioConfig {
            symbol: "BTCUSDT".into(),
            ..Default::default()
        },
    );
    let rm = RiskManager::new(
        RiskConfig {
            stop_loss_pct: 0.05,
            take_profit_pct: 0.10,
            max_position_pct: 0.95,
        },
        "BTCUSDT".into(),
    );
    assert!(rm.force_close_reason(&pf, pf.broker.as_ref()).is_none());
    pf.broker.set_market_price("BTCUSDT", 96.0);
    assert!(rm.force_close_reason(&pf, pf.broker.as_ref()).is_none());
    pf.broker.set_market_price("BTCUSDT", 90.0);
    assert!(rm
        .force_close_reason(&pf, pf.broker.as_ref())
        .unwrap()
        .contains("止损"));
    pf.broker.set_market_price("BTCUSDT", 112.0);
    assert!(rm
        .force_close_reason(&pf, pf.broker.as_ref())
        .unwrap()
        .contains("止盈"));
}

#[test]
fn test_trade_pnl_calculation() {
    let entry = 100.0;
    let exit = 120.0;
    let size = 0.5;
    let trade = Trade {
        symbol: "BTCUSDT".to_string(),
        side: Side::Buy,
        entry_time: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        exit_time: Some(Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()),
        entry_price: entry,
        exit_price: Some(exit),
        size,
        entry_commission: 0.05,
        exit_commission: 0.06,
    };
    let pnl = trade.pnl();
    assert!((pnl - ((exit - entry) * size - 0.11)).abs() < 1e-9);
    assert!(trade.is_closed());
    assert_eq!(trade.duration_seconds(), Some(86400.0));
}
