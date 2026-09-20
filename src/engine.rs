//! 回测引擎 —— 把所有零件拼起来,按"一根 K 线一根 K 线"地驱动整个系统。
//!
//! 事件循环(每根 K 线):
//!   1. 更新券商的当前价
//!   2. 让策略看到这根 K 线 → 得到 Signal
//!   3. 风控检查:要不要覆写信号(止损/止盈)
//!   4. 组合把信号转成具体订单
//!   5. 券商执行订单 → Fill
//!   6. 组合处理 Fill,记录 Trade
//!   7. 快照净值

use crate::broker::{BrokerConfig, SimulatedBroker};
use crate::metrics::compute_metrics;
use crate::portfolio::{Portfolio, PortfolioConfig};
use crate::risk::{RiskConfig, RiskManager};
use crate::strategy::Strategy;
use crate::types::{BacktestResult, Bar, EquityPoint, Fill, Side, Signal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub symbol: String,
    pub initial_capital: f64,
    pub commission_rate: f64,
    pub slippage_rate: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            symbol: "BTC/USDT".to_string(),
            initial_capital: 10_000.0,
            commission_rate: 0.001,
            slippage_rate: 0.0005,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestProgress {
    pub current: usize,
    pub total: usize,
    pub latest_bar: Option<Bar>,
    pub latest_signal: Option<Signal>,
    pub latest_fill: Option<Fill>,
    pub equity: f64,
}

pub struct BacktestEngine {
    pub config: EngineConfig,
    pub risk_config: RiskConfig,
}

impl BacktestEngine {
    pub fn new(config: EngineConfig, risk_config: RiskConfig) -> Self {
        Self {
            config,
            risk_config,
        }
    }

    /// 在给定的 K 线序列上跑策略,产出 BacktestResult。
    pub fn run(&self, strategy: &mut dyn Strategy, bars: &[Bar]) -> BacktestResult {
        strategy.reset();

        let broker = SimulatedBroker::new(
            BrokerConfig {
                commission_rate: self.config.commission_rate,
                slippage_rate: self.config.slippage_rate,
                allow_short: false,
            },
            self.config.initial_capital,
        );

        let mut portfolio = Portfolio::new(
            Box::new(broker),
            PortfolioConfig {
                max_position_pct: self.risk_config.max_position_pct,
                min_trade_size: 1e-6,
                symbol: self.config.symbol.clone(),
            },
        );
        let risk = RiskManager::new(self.risk_config.clone(), self.config.symbol.clone());

        let mut equity_curve: Vec<EquityPoint> = Vec::with_capacity(bars.len());
        let mut fills: Vec<Fill> = Vec::new();
        let mut signals: Vec<Signal> = Vec::with_capacity(bars.len());

        let total = bars.len();

        let mut pending_signal: Option<Signal> = None;
        for bar in bars {
            // A signal is known only after its source candle closes. Execute
            // the previous signal at this candle's open, never at that close.
            portfolio
                .broker
                .set_market_price(&self.config.symbol, bar.open);
            let previous_signal = pending_signal.take();
            let execution_signal =
                if let Some(reason) = risk.force_close_reason(&portfolio, &*portfolio.broker) {
                    Some(Signal {
                        timestamp: bar.timestamp,
                        side: Side::Sell,
                        strength: 1.0,
                        reason: format!("[风控] {}", reason),
                        target_size: None,
                    })
                } else {
                    previous_signal
                };
            if let Some(signal) = execution_signal {
                if signal.side != Side::Hold {
                    if let Some(order) =
                        portfolio.on_signal(signal.side, bar.open, bar.timestamp, signal.strength)
                    {
                        let (allowed, _) = risk.allow_order(&order, &portfolio, &*portfolio.broker);
                        if allowed {
                            let fill = portfolio.broker.place_order(order);
                            if fill.size > 0.0 {
                                fills.push(fill.clone());
                                portfolio.on_fill(&fill);
                            }
                        }
                    }
                }
            }

            // Mark to the completed candle close, then create an intention for
            // the following open. The final intention deliberately remains unfilled.
            portfolio
                .broker
                .set_market_price(&self.config.symbol, bar.close);
            let signal = strategy.on_bar(bar);
            signals.push(signal.clone());
            if signal.side != Side::Hold {
                pending_signal = Some(signal);
            }

            let pos = portfolio.position();
            equity_curve.push(EquityPoint {
                timestamp: bar.timestamp,
                cash: portfolio.broker.get_cash(),
                position_value: pos.market_value(bar.close),
                equity: portfolio.equity(),
            });
        }

        BacktestResult {
            config: serde_json::json!({
                "symbol": self.config.symbol,
                "initial_capital": self.config.initial_capital,
                "commission_rate": self.config.commission_rate,
                "slippage_rate": self.config.slippage_rate,
                "strategy": strategy.name(),
                "params": strategy.params(),
                "n_bars": total,
            }),
            equity_curve,
            trades: portfolio.closed_trades().to_vec(),
            fills,
            signals,
            metrics: serde_json::Value::Null,
        }
    }
}

// -----------------------------------------------------------------------------
// 多策略对比
// -----------------------------------------------------------------------------

pub struct MultiStrategyResult {
    pub results: HashMap<String, BacktestResult>,
}

impl MultiStrategyResult {
    pub fn best_by(&self, metric: &str) -> Option<(String, &BacktestResult)> {
        let mut best: Option<(String, f64, &BacktestResult)> = None;
        for (name, result) in &self.results {
            let v = result
                .metrics
                .get(metric)
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::NEG_INFINITY);
            match &best {
                None => best = Some((name.clone(), v, result)),
                Some((_, bv, _)) if v > *bv => best = Some((name.clone(), v, result)),
                _ => {}
            }
        }
        best.map(|(n, _, r)| (n, r))
    }
}

pub fn compare_strategies(
    strategies: Vec<(String, Box<dyn Strategy>)>,
    bars: &[Bar],
    engine_config: EngineConfig,
    risk_config: RiskConfig,
) -> MultiStrategyResult {
    let mut out = MultiStrategyResult {
        results: HashMap::new(),
    };
    let engine = BacktestEngine::new(engine_config, risk_config);
    for (name, mut strat) in strategies {
        let mut result = engine.run(strat.as_mut(), bars);
        result.metrics = compute_metrics(&result);
        out.results.insert(name, result);
    }
    out
}
