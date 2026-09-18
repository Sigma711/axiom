//! 整个系统共用的基础数据类型。
//!
//! 每个类型对应量化交易里的一个核心概念:
//!   - `Bar`:     一根 K 线 (OHLCV)
//!   - `Signal`:  策略的"想法"
//!   - `Order`:   给券商的"指令"
//!   - `Fill`:    指令被实际执行的"成交回报"
//!   - `Position`: 你现在持有多少某个标的
//!   - `Trade`:   一笔从开到平的完整交易

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// 买卖方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
    Hold,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
            Side::Hold => write!(f, "HOLD"),
        }
    }
}

/// 订单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Market => write!(f, "MARKET"),
            OrderType::Limit => write!(f, "LIMIT"),
        }
    }
}

/// 一根 K 线 (OHLCV)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bar {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Bar {
    /// 典型价 = (H + L + C) / 3
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }
}

/// 策略信号 —— "想法",不是指令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub timestamp: DateTime<Utc>,
    pub side: Side,
    pub strength: f64,           // 0~1, 信号强度
    pub reason: String,           // 人类可读解释
    pub target_size: Option<f64>, // 可选:策略直接指定下单数量
}

/// 给券商的订单指令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub size: f64,
    pub price: Option<f64>, // 仅限价单需要
}

/// 订单的实际成交回报
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: String,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub side: Side,
    pub size: f64,
    pub price: f64,
    pub commission: f64,
}

impl Fill {
    /// 成交的总金额(不含手续费)
    pub fn value(&self) -> f64 {
        self.size * self.price
    }

    /// 对现金的影响 (正数=消耗现金, 负数=收到现金)
    pub fn cash_impact(&self) -> f64 {
        match self.side {
            Side::Buy => self.value() + self.commission,
            Side::Sell => -(self.value() - self.commission),
            Side::Hold => 0.0,
        }
    }
}

/// 单个标的的当前持仓
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub size: f64,             // 正=多头, 负=空头, 0=空仓
    pub avg_entry_price: f64,
    pub realized_pnl: f64,
}

impl Position {
    pub fn is_flat(&self) -> bool {
        self.size.abs() < 1e-9
    }
    pub fn is_long(&self) -> bool {
        self.size > 0.0
    }
    pub fn is_short(&self) -> bool {
        self.size < 0.0
    }
    pub fn market_value(&self, price: f64) -> f64 {
        self.size * price
    }
    pub fn unrealized_pnl(&self, price: f64) -> f64 {
        self.size * (price - self.avg_entry_price)
    }
}

/// 一笔完整的交易:从开仓到平仓
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: String,
    pub side: Side,
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub size: f64,
    pub entry_commission: f64,
    pub exit_commission: f64,
}

impl Trade {
    pub fn is_closed(&self) -> bool {
        self.exit_time.is_some()
    }
    pub fn total_commission(&self) -> f64 {
        self.entry_commission + self.exit_commission
    }
    pub fn pnl(&self) -> f64 {
        match (self.is_closed(), self.exit_price) {
            (true, Some(exit)) => {
                let gross = (exit - self.entry_price) * self.size;
                let gross = if self.side == Side::Sell { -gross } else { gross };
                gross - self.total_commission()
            }
            _ => 0.0,
        }
    }
    pub fn pnl_pct(&self) -> f64 {
        if !self.is_closed() {
            return 0.0;
        }
        let cost = self.entry_price * self.size;
        if cost == 0.0 {
            0.0
        } else {
            self.pnl() / cost
        }
    }
    pub fn duration_seconds(&self) -> Option<f64> {
        match self.exit_time {
            Some(t) => Some((t - self.entry_time).num_seconds() as f64),
            None => None,
        }
    }
}

/// 组合在某一时刻的快照 —— 用于画净值曲线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: DateTime<Utc>,
    pub cash: f64,
    pub position_value: f64,
    pub equity: f64,
}

/// 一次回测的完整产出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub config: serde_json::Value,
    pub equity_curve: Vec<EquityPoint>,
    pub trades: Vec<Trade>,
    pub fills: Vec<Fill>,
    pub signals: Vec<Signal>,
    pub metrics: serde_json::Value,
}

impl BacktestResult {
    pub fn initial_equity(&self) -> f64 {
        self.equity_curve
            .first()
            .map(|p| p.cash + p.position_value)
            .unwrap_or(0.0)
    }
    pub fn final_equity(&self) -> f64 {
        self.equity_curve
            .last()
            .map(|p| p.equity)
            .unwrap_or(0.0)
    }
}