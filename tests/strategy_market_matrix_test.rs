// Market/parameter matrix checks the public engine seam with an independent
// OHLCV fixture. It tests execution timing and accounting without claiming the
// fixture is a historical market observation.
use axiom::execution::{ChinaBoard, ExecutionMarket, ExecutionProfile};
use axiom::strategy::{create_strategy, StrategyKind};
use axiom::types::Bar;
use axiom::{BacktestEngine, EngineConfig, RiskConfig};
use chrono::{Duration, TimeZone, Utc};

fn bars() -> Vec<Bar> {
    let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    (0..90)
        .map(|i| {
            let close = 100.0 + i as f64 * 0.08 + (i as f64 * 0.38).sin() * 8.0;
            let open = close - 0.25;
            Bar {
                timestamp: start + Duration::days(i),
                open,
                high: close + 1.25,
                low: open - 1.25,
                close,
                volume: 1_000.0 + i as f64 * 13.0,
            }
        })
        .collect()
}

fn strategies() -> Vec<StrategyKind> {
    vec![
        StrategyKind::BuyAndHold,
        StrategyKind::SmaCross { fast: 5, slow: 20 },
        StrategyKind::Rsi {
            period: 14,
            overbought: 70.0,
            oversold: 30.0,
        },
        StrategyKind::Random {
            seed: 42,
            buy_prob: 0.05,
            sell_prob: 0.05,
        },
        StrategyKind::Macd {
            fast: 12,
            slow: 26,
            signal: 9,
        },
        StrategyKind::Bollinger {
            period: 20,
            num_std: 2.0,
        },
        StrategyKind::Supertrend {
            period: 10,
            multiplier: 3.0,
        },
        StrategyKind::DonchianBreakout {
            entry_period: 20,
            exit_period: 10,
        },
        StrategyKind::VwapReversion {
            period: 20,
            threshold_pct: 1.5,
        },
        StrategyKind::Kdj { n: 9, m1: 3, m2: 3 },
        StrategyKind::Ichimoku {
            tenkan: 9,
            kijun: 26,
            senkou_b: 52,
            displacement: 26,
        },
        StrategyKind::Ppo {
            fast: 12,
            slow: 26,
            signal: 9,
        },
        StrategyKind::Vortex { period: 14 },
        StrategyKind::ElderRay { period: 13 },
        // One shorter/faster parameter class per configurable family. The
        // fixture is deliberately oscillatory so warm-up and crossover timing
        // differ from the defaults.
        StrategyKind::SmaCross { fast: 3, slow: 9 },
        StrategyKind::Rsi {
            period: 5,
            overbought: 60.0,
            oversold: 40.0,
        },
        StrategyKind::Random {
            seed: 7,
            buy_prob: 0.1,
            sell_prob: 0.1,
        },
        StrategyKind::Macd {
            fast: 5,
            slow: 16,
            signal: 6,
        },
        StrategyKind::Bollinger {
            period: 10,
            num_std: 1.5,
        },
        StrategyKind::Supertrend {
            period: 7,
            multiplier: 2.0,
        },
        StrategyKind::DonchianBreakout {
            entry_period: 10,
            exit_period: 5,
        },
        StrategyKind::VwapReversion {
            period: 10,
            threshold_pct: 0.8,
        },
        StrategyKind::Kdj { n: 5, m1: 2, m2: 2 },
        StrategyKind::Ichimoku {
            tenkan: 5,
            kijun: 13,
            senkou_b: 26,
            displacement: 13,
        },
        StrategyKind::Ppo {
            fast: 5,
            slow: 16,
            signal: 6,
        },
        StrategyKind::Vortex { period: 7 },
        StrategyKind::ElderRay { period: 7 },
    ]
}

#[test]
fn every_strategy_respects_next_open_causality_and_reconciles_equity_in_each_market() {
    let baseline = bars();
    let mut changed_future = baseline.clone();
    for bar in &mut changed_future[60..] {
        bar.open *= 1.2;
        bar.high *= 1.2;
        bar.low *= 1.2;
        bar.close *= 1.2;
    }
    for (source, symbol, market) in [
        ("binance", "BTCUSDT", ExecutionMarket::CryptoSpot),
        ("us_stock", "AAPL", ExecutionMarket::UsEquity),
        (
            "a_share",
            "600519",
            ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth),
        ),
    ] {
        for (original_kind, altered_kind) in strategies().into_iter().zip(strategies()) {
            let config = EngineConfig {
                symbol: symbol.into(),
                initial_capital: 100_000.0,
                commission_rate: 0.0,
                slippage_rate: 0.0,
            };
            let engine = BacktestEngine::new(
                config,
                RiskConfig {
                    stop_loss_pct: 0.0,
                    take_profit_pct: 0.0,
                    max_position_pct: 0.95,
                },
            )
            .with_execution_profile(ExecutionProfile::for_market(source, symbol));
            let original = engine.run(create_strategy(original_kind).as_mut(), &baseline);
            let altered = engine.run(create_strategy(altered_kind).as_mut(), &changed_future);
            assert_eq!(original.signals.len(), baseline.len(), "{source} {symbol}");
            assert_eq!(
                original.equity_curve.len(),
                baseline.len(),
                "{source} {symbol}"
            );
            for i in 0..60 {
                assert_eq!(
                    original.signals[i].side, altered.signals[i].side,
                    "future data changed signal {source} {i}"
                );
                assert_eq!(
                    original.signals[i].reason, altered.signals[i].reason,
                    "future data changed reason {source} {i}"
                );
                assert!(
                    (original.equity_curve[i].equity - altered.equity_curve[i].equity).abs() < 1e-7,
                    "future data changed equity {source} {i}"
                );
            }
            for point in &original.equity_curve {
                assert!(point.equity.is_finite());
                assert!(
                    (point.cash + point.position_value - point.equity).abs() < 1e-7,
                    "unreconciled equity {source}"
                );
                assert!(point.cash >= -1e-7, "negative cash {source}");
                assert!(point.position_value >= -1e-7, "short position {source}");
            }
            for fill in &original.fills {
                let index = baseline
                    .iter()
                    .position(|bar| bar.timestamp == fill.timestamp)
                    .unwrap();
                assert!(index > 0, "same-bar execution {source}");
                assert!(
                    (fill.price - baseline[index].open).abs() < 1e-7,
                    "not next-open execution {source}"
                );
                assert!(fill.size > 0.0 && fill.size.is_finite());
                if market == ExecutionMarket::UsEquity
                    || matches!(market, ExecutionMarket::ChinaA(_))
                {
                    assert_eq!(fill.size.fract(), 0.0, "fractional equity fill {source}");
                }
            }
        }
    }
}
