//! 模拟盘引擎 —— 长时间运行,实时拉行情,跑策略,管理组合。
//!
//! 设计要点:
//!   - 一个后台 tokio 任务负责定时拉行情 + 调用 process_bar(同步)
//!   - 状态变更通过 axum 的 State 共享
//!   - 模拟盘的"实时" = 每 N 秒拉一次最新 K 线收盘价

use crate::broker::{Broker, BrokerConfig, SimulatedBroker};
use crate::data::AsyncDataFeed;
use crate::portfolio::{Portfolio, PortfolioConfig};
use crate::risk::{RiskConfig, RiskManager};
use crate::strategy::Strategy;
use crate::types::{Bar, EquityPoint, Fill, Signal};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperConfigP {
    pub symbol: String,
    pub initial_capital: f64,
    pub commission_rate: f64,
    pub slippage_rate: f64,
    pub poll_interval_seconds: u64,
    pub risk: RiskConfig,
}

impl Default for PaperConfigP {
    fn default() -> Self {
        Self {
            symbol: "BTCUSDT".to_string(),
            initial_capital: 10_000.0,
            commission_rate: 0.001,
            slippage_rate: 0.0005,
            poll_interval_seconds: 5,
            risk: RiskConfig::default(),
        }
    }
}

/// 模拟盘的对外状态快照(给前端展示)
#[derive(Debug, Clone, Serialize)]
pub struct PaperSnapshot {
    pub is_running: bool,
    pub current_bar: Option<Bar>,
    pub cash: f64,
    pub position_size: f64,
    pub position_value: f64,
    pub equity: f64,
    pub last_signal: Option<Signal>,
    pub last_fill: Option<Fill>,
    pub equity_curve: Vec<EquityPoint>,
    pub trades_count: usize,
    pub log: Vec<PaperLogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

/// 模拟盘的内部状态
pub struct PaperState {
    pub config: PaperConfigP,
    pub broker: SimulatedBroker,
    pub portfolio: Portfolio,
    pub risk: RiskManager,
    pub strategy: Box<dyn Strategy>,
    pub is_running: bool,
    pub current_bar: Option<Bar>,
    pub last_signal: Option<Signal>,
    pub last_fill: Option<Fill>,
    pub equity_curve: Vec<EquityPoint>,
    pub log: Vec<PaperLogEntry>,
}

impl PaperState {
    pub fn new(config: PaperConfigP, strategy: Box<dyn Strategy>) -> Self {
        let broker = SimulatedBroker::new(
            BrokerConfig {
                commission_rate: config.commission_rate,
                slippage_rate: config.slippage_rate,
                allow_short: false,
            },
            config.initial_capital,
        );
        let portfolio = Portfolio::new(
            Box::new(SimulatedBroker::new(
                BrokerConfig {
                    commission_rate: config.commission_rate,
                    slippage_rate: config.slippage_rate,
                    allow_short: false,
                },
                config.initial_capital,
            )),
            PortfolioConfig {
                max_position_pct: config.risk.max_position_pct,
                min_trade_size: 1e-6,
                symbol: config.symbol.clone(),
            },
        );
        let risk = RiskManager::new(config.risk.clone(), config.symbol.clone());

        Self {
            config,
            broker,
            portfolio,
            risk,
            strategy,
            is_running: false,
            current_bar: None,
            last_signal: None,
            last_fill: None,
            equity_curve: Vec::new(),
            log: Vec::new(),
        }
    }

    /// 处理一根新的 K 线:更新价格 → 策略判断 → 风控 → 下单 → 快照
    pub fn process_bar(&mut self, bar: Bar) -> anyhow::Result<()> {
        self.current_bar = Some(bar);
        self.portfolio
            .broker
            .set_market_price(&self.config.symbol, bar.close);
        self.broker.set_market_price(&self.config.symbol, bar.close);

        let signal = self.strategy.on_bar(&bar);
        self.last_signal = Some(signal.clone());
        self.log(
            PaperLogLevel::Info,
            format!("信号: {} ({})", signal.side, signal.reason),
        );

        let mut order = None;

        if let Some(reason) = self
            .risk
            .force_close_reason(&self.portfolio, &*self.portfolio.broker)
        {
            if !self.portfolio.is_flat() {
                order = self.portfolio.on_signal(
                    crate::types::Side::Sell,
                    bar.close,
                    bar.timestamp,
                    1.0,
                );
            }
            self.log(PaperLogLevel::Warn, format!("[风控] {}", reason));
        } else if signal.side != crate::types::Side::Hold {
            order =
                self.portfolio
                    .on_signal(signal.side, bar.close, bar.timestamp, signal.strength);
        }

        if let Some(ref o) = order {
            let (allowed, why) = self
                .risk
                .allow_order(o, &self.portfolio, &*self.portfolio.broker);
            if !allowed {
                self.log(PaperLogLevel::Warn, format!("风控拒绝: {}", why));
                order = None;
            }
        }

        if let Some(o) = order {
            let fill = self.portfolio.broker.place_order(o);
            if fill.size > 0.0 {
                self.last_fill = Some(fill.clone());
                self.portfolio.on_fill(&fill);
                self.log(
                    PaperLogLevel::Fill,
                    format!(
                        "成交: {} {} @ {:.2} (手续费 {:.2})",
                        fill.side, fill.size, fill.price, fill.commission
                    ),
                );
            }
        }

        let price = self
            .portfolio
            .broker
            .get_market_price(&self.config.symbol)
            .unwrap_or(bar.close);
        let pos = self.portfolio.position();
        self.equity_curve.push(EquityPoint {
            timestamp: bar.timestamp,
            cash: self.portfolio.broker.get_cash(),
            position_value: pos.market_value(price),
            equity: self.portfolio.equity(),
        });

        Ok(())
    }

    pub fn snapshot(&self) -> PaperSnapshot {
        let pos = self.portfolio.position();
        let price = self
            .portfolio
            .broker
            .get_market_price(&self.config.symbol)
            .or_else(|| self.current_bar.map(|b| b.close))
            .unwrap_or(0.0);
        PaperSnapshot {
            is_running: self.is_running,
            current_bar: self.current_bar,
            cash: self.portfolio.broker.get_cash(),
            position_size: pos.size,
            position_value: pos.market_value(price),
            equity: self.portfolio.equity(),
            last_signal: self.last_signal.clone(),
            last_fill: self.last_fill.clone(),
            equity_curve: self.equity_curve.clone(),
            trades_count: self.portfolio.closed_trades().len(),
            log: self.log.iter().rev().take(100).cloned().collect(),
        }
    }

    pub fn log(&mut self, level: PaperLogLevel, msg: String) {
        self.log.push(PaperLogEntry {
            timestamp: chrono::Utc::now(),
            level: level.as_str().to_string(),
            message: msg,
        });
        if self.log.len() > 1000 {
            let drop = self.log.len() - 1000;
            self.log.drain(0..drop);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PaperLogLevel {
    Info,
    Warn,
    Error,
    Fill,
}

impl PaperLogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaperLogLevel::Info => "INFO",
            PaperLogLevel::Warn => "WARN",
            PaperLogLevel::Error => "ERROR",
            PaperLogLevel::Fill => "FILL",
        }
    }
}

/// 后台任务:定时拉数据 + 调用 process_bar
pub async fn run_paper_loop<F: AsyncDataFeed>(feed: Arc<F>, state: Arc<RwLock<PaperState>>) {
    loop {
        let (is_running, symbol, poll_secs) = {
            let s = state.read().await;
            (
                s.is_running,
                s.config.symbol.clone(),
                s.config.poll_interval_seconds,
            )
        };

        if !is_running {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }

        match feed.stream_live_async(&symbol).await {
            Ok(bars) if !bars.is_empty() => {
                let mut s = state.write().await;
                for bar in bars {
                    if let Err(e) = s.process_bar(bar) {
                        s.log(PaperLogLevel::Error, format!("处理出错: {}", e));
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                let mut s = state.write().await;
                s.log(PaperLogLevel::Error, format!("拉取数据失败: {}", e));
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
    }
}
