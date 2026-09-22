//! AXIOM —— 面向小白的量化交易教学系统
//!
//! ## 架构
//!
//! 整个系统是事件驱动的,每个模块职责单一:
//!
//! ```text
//!     Bar (K线) ──┐
//!                 ▼
//!             Strategy ──► Signal
//!                          │
//!                          ▼
//!                  RiskManager (风控)
//!                          │
//!                          ▼
//!                  Portfolio (仓位管理)
//!                          │
//!                          ▼
//!                     Broker ──► Fill
//!                          │
//!                          ▼
//!                    Metrics + Equity 曲线
//! ```
//!
//! - 策略只产生 Signal,不直接下单
//! - Broker 是 seam:回测/模拟盘/实盘共用同一接口
//! - 任何模块都可以被替换,只要保持接口不变

pub mod broker;
pub mod config;
pub mod data;
pub mod engine;
pub mod indicators;
pub mod knowledge;
pub mod metrics;
pub mod paper;
pub mod portfolio;
pub mod practice;
pub mod risk;
pub mod strategy;
pub mod supplement;
pub mod symbols;
pub mod types;

pub mod api;
mod api_validation;
pub mod app_state;

pub use broker::{Broker, BrokerConfig, SimulatedBroker};
pub use engine::{BacktestEngine, EngineConfig, MultiStrategyResult};
pub use portfolio::{Portfolio, PortfolioConfig};
pub use risk::{RiskConfig, RiskManager};
pub use strategy::{
    BollingerBandsStrategy, BuyAndHoldStrategy, DonchianBreakoutStrategy, ElderRayStrategy,
    IchimokuStrategy, KdjStrategy, MacdStrategy, PpoStrategy, RandomStrategy, RsiStrategy,
    SmaCrossStrategy, Strategy, SupertrendStrategy, VortexStrategy, VwapReversionStrategy,
};
pub use types::{BacktestResult, Bar, EquityPoint, Fill, Order, Position, Side, Signal, Trade};
pub mod diagrams;

pub mod book;
pub mod book_sources;
pub mod book_technical;
pub mod code_links;
pub mod workflows;

pub mod book_charts;
