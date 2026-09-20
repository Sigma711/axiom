//! 组合 = 现金 + 持仓。它的核心职责:
//!   1. 跟踪当前净值
//!   2. 把策略的 Signal 转成具体的 Order (负责仓位大小)
//!   3. 记录已完成的 Trade
//!
//! 仓位大小计算(简化):
//!   - 买入:最多动用 max_position_pct 的可用现金
//!   - 卖出:清掉当前所有持仓
//!
//! 注意:Portfolio 拥有 broker 的所有权(Box<dyn Broker>),
//! 这样下单时可以直接拿可变引用,不需要 Arc/Mutex。
//! 如果要跨线程共享,把整个 Portfolio 用 Arc<Mutex<>> 包起来。

use crate::broker::{new_order, Broker};
use crate::types::{Fill, Order, OrderType, Position, Side, Trade};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    pub max_position_pct: f64,
    pub min_trade_size: f64,
    pub symbol: String,
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            max_position_pct: 0.95,
            min_trade_size: 1e-6,
            symbol: "BTC/USDT".to_string(),
        }
    }
}

pub struct Portfolio {
    pub broker: Box<dyn Broker>,
    pub config: PortfolioConfig,
    open_trade: Option<Trade>,
    closed_trades: Vec<Trade>,
}

impl Portfolio {
    pub fn new(broker: Box<dyn Broker>, config: PortfolioConfig) -> Self {
        Self {
            broker,
            config,
            open_trade: None,
            closed_trades: Vec::new(),
        }
    }

    /// 当前净值 = 现金 + 持仓按最新价估值
    pub fn equity(&self) -> f64 {
        let mut total = self.broker.get_cash();
        let pos = self.position();
        if let Some(price) = self.broker.get_market_price(&self.config.symbol) {
            total += pos.market_value(price);
        }
        total
    }

    pub fn cash(&self) -> f64 {
        self.broker.get_cash()
    }

    pub fn position(&self) -> Position {
        self.broker.get_position(&self.config.symbol)
    }

    pub fn is_flat(&self) -> bool {
        self.position().is_flat()
    }

    /// 把策略信号翻译成具体的订单,带仓位大小。
    pub fn on_signal(
        &self,
        signal: Side,
        current_price: f64,
        ts: DateTime<Utc>,
        strength: f64,
    ) -> Option<Order> {
        let pos = self.position();
        match signal {
            Side::Buy => {
                if !pos.is_flat() {
                    return None; // 简化:已持仓时忽略重复买入
                }
                let size = self.calc_buy_size(current_price, strength);
                if size < self.config.min_trade_size {
                    return None;
                }
                Some(new_order(
                    &self.config.symbol,
                    Side::Buy,
                    size,
                    ts,
                    OrderType::Market,
                    None,
                ))
            }
            Side::Sell => {
                if pos.is_flat() {
                    return None;
                }
                let size = pos.size.abs();
                if size < self.config.min_trade_size {
                    return None;
                }
                Some(new_order(
                    &self.config.symbol,
                    Side::Sell,
                    size,
                    ts,
                    OrderType::Market,
                    None,
                ))
            }
            Side::Hold => None,
        }
    }

    /// 成交回报回调:同步更新内部 Trade 记录
    pub fn on_fill(&mut self, fill: &Fill) {
        if fill.size == 0.0 {
            return;
        }
        match (fill.side, self.open_trade.as_ref()) {
            (Side::Buy, None) => {
                self.open_trade = Some(Trade {
                    symbol: fill.symbol.clone(),
                    side: Side::Buy,
                    entry_time: fill.timestamp,
                    exit_time: None,
                    entry_price: fill.price,
                    exit_price: None,
                    size: fill.size,
                    entry_commission: fill.commission,
                    exit_commission: 0.0,
                });
            }
            (Side::Sell, Some(t)) => {
                let mut closed = t.clone();
                closed.exit_time = Some(fill.timestamp);
                closed.exit_price = Some(fill.price);
                closed.exit_commission = fill.commission;
                self.closed_trades.push(closed);
                self.open_trade = None;
            }
            _ => {}
        }
    }

    pub fn closed_trades(&self) -> &[Trade] {
        &self.closed_trades
    }

    pub fn open_trade(&self) -> Option<&Trade> {
        self.open_trade.as_ref()
    }

    fn calc_buy_size(&self, price: f64, strength: f64) -> f64 {
        let cash = self.broker.get_cash();
        let strength = strength.clamp(0.0, 1.0);
        let max_money = cash * self.config.max_position_pct * strength;
        let cost = self.broker.buy_cost_per_unit(price);
        if !cost.is_finite() || cost <= 0.0 || !max_money.is_finite() {
            0.0
        } else {
            max_money / cost
        }
    }
}
