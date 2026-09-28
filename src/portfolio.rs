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
use crate::execution::ExecutionProfile;
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
    execution_profile: ExecutionProfile,
}

impl Portfolio {
    pub fn new(broker: Box<dyn Broker>, config: PortfolioConfig) -> Self {
        Self::new_with_execution_profile(broker, config, ExecutionProfile::default())
    }

    pub fn new_with_execution_profile(
        broker: Box<dyn Broker>,
        config: PortfolioConfig,
        execution_profile: ExecutionProfile,
    ) -> Self {
        Self {
            broker,
            config,
            open_trade: None,
            closed_trades: Vec::new(),
            execution_profile,
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
                let size = self
                    .broker
                    .sellable_size(&self.config.symbol, ts)
                    .min(pos.size.abs());
                if size < self.config.min_trade_size {
                    return None;
                }
                if !self
                    .execution_profile
                    .valid_sell_quantity(size, pos.size.abs(), size)
                {
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
        if !fill.size.is_finite()
            || fill.size <= 0.0
            || !fill.price.is_finite()
            || fill.price <= 0.0
            || !fill.commission.is_finite()
            || fill.commission < 0.0
            || fill.symbol != self.config.symbol
        {
            return;
        }
        match fill.side {
            Side::Buy => match self.open_trade.as_mut() {
                Some(open) if open.side == Side::Buy && open.symbol == fill.symbol => {
                    let combined_size = open.size + fill.size;
                    if !combined_size.is_finite() {
                        return;
                    }
                    open.entry_price =
                        (open.entry_price * open.size + fill.price * fill.size) / combined_size;
                    open.size = combined_size;
                    open.entry_commission += fill.commission;
                }
                None => {
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
                Some(_) => {}
            },
            Side::Sell => {
                let Some(mut open) = self.open_trade.take() else {
                    return;
                };
                if open.side != Side::Buy || open.symbol != fill.symbol {
                    self.open_trade = Some(open);
                    return;
                }
                let closed_size = fill.size.min(open.size);
                let fully_closed = closed_size >= open.size - 1e-9;
                let allocated_entry_commission = if fully_closed {
                    open.entry_commission
                } else {
                    open.entry_commission * closed_size / open.size
                };
                let allocated_exit_commission = fill.commission * closed_size / fill.size;
                self.closed_trades.push(Trade {
                    symbol: fill.symbol.clone(),
                    side: Side::Buy,
                    entry_time: open.entry_time,
                    exit_time: Some(fill.timestamp),
                    entry_price: open.entry_price,
                    exit_price: Some(fill.price),
                    size: closed_size,
                    entry_commission: allocated_entry_commission,
                    exit_commission: allocated_exit_commission,
                });
                if !fully_closed {
                    open.size -= closed_size;
                    open.entry_commission -= allocated_entry_commission;
                    self.open_trade = Some(open);
                }
            }
            Side::Hold => {}
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
            self.execution_profile
                .buy_quantity(max_money / cost)
                .unwrap_or(0.0)
        }
    }
}
