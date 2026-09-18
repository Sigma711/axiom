//! "模拟券商" —— 接收订单,模拟真实成交。
//!
//! 关键概念:
//!   - 手续费 (commission):每笔成交按成交额的一定比例扣除
//!   - 滑点 (slippage):实际成交价和市场中间价的偏离,市价单尤其明显
//!   - 现金与持仓:买消耗现金 + 增加持仓,卖反向
//!
//! 这是 demo 里最关键的 seam。回测、模拟盘、实盘都共用这个接口,
//! 只是背后实现不同:
//!   - `SimulatedBroker` (本文件):用于回测和模拟盘
//!   - 实盘用 ccxt 的真实下单 API(另写 adapter 复用同一 trait)

use crate::types::{Fill, Order, OrderType, Position, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 券商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerConfig {
    pub commission_rate: f64, // 单边手续费率
    pub slippage_rate: f64,   // 滑点率(对中间价的偏离)
    pub allow_short: bool,    // 是否允许做空
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            commission_rate: 0.001,
            slippage_rate: 0.0005,
            allow_short: false,
        }
    }
}

/// 券商的对外接口。回测/模拟盘/实盘共用。
pub trait Broker: Send + Sync {
    fn place_order(&mut self, order: Order) -> Fill;
    fn get_position(&self, symbol: &str) -> Position;
    fn get_cash(&self) -> f64;
    fn set_market_price(&mut self, symbol: &str, price: f64);
    fn get_market_price(&self, symbol: &str) -> Option<f64>;
    fn trade_log(&self) -> Vec<(DateTime<Utc>, String, f64)>;
}

/// 模拟券商 —— 跑回测和模拟盘都用这个
///
/// 设计原则:
///   - 无 I/O、无网络、纯函数式 —— 测试友好
///   - 维护 last_price,用于限价单判断和滑点计算
///   - 订单要么全部成交,要么全部不成交(简化模型)
#[derive(Debug, Clone)]
pub struct SimulatedBroker {
    pub config: BrokerConfig,
    pub initial_cash: f64,
    pub positions: HashMap<String, Position>,
    pub last_price: HashMap<String, f64>,
    cash: f64,
    trade_log: Vec<(DateTime<Utc>, String, f64)>,
}

impl SimulatedBroker {
    pub fn new(config: BrokerConfig, initial_cash: f64) -> Self {
        Self {
            config,
            initial_cash,
            positions: HashMap::new(),
            last_price: HashMap::new(),
            cash: initial_cash,
            trade_log: Vec::new(),
        }
    }

    /// 计算成交价:市价单按中间价 ± 滑点成交;限价单仅在价格触及限价时成交。
    fn resolve_fill_price(&self, order: &Order) -> Option<f64> {
        let mid = *self.last_price.get(&order.symbol)?;
        match order.order_type {
            OrderType::Market => {
                let slip = mid * self.config.slippage_rate;
                Some(match order.side {
                    Side::Buy => mid + slip,  // 买得更贵一点
                    Side::Sell => mid - slip, // 卖得更便宜一点
                    Side::Hold => return None,
                })
            }
            OrderType::Limit => {
                let limit = order.price?;
                match order.side {
                    Side::Buy if mid <= limit => Some(limit),
                    Side::Sell if mid >= limit => Some(limit),
                    _ => None,
                }
            }
        }
    }

    /// 把成交应用到账户上(更新现金和持仓)
    fn apply_fill(&mut self, symbol: &str, side: Side, size: f64, price: f64, commission: f64) {
        let pos = self
            .positions
            .entry(symbol.to_string())
            .or_insert_with(|| Position {
                symbol: symbol.to_string(),
                ..Default::default()
            });
        match side {
            Side::Buy => {
                self.cash -= price * size + commission;
                let new_size = pos.size + size;
                if new_size.abs() < 1e-9 {
                    pos.size = 0.0;
                    pos.avg_entry_price = 0.0;
                } else {
                    pos.avg_entry_price =
                        (pos.avg_entry_price * pos.size + price * size) / new_size;
                    pos.size = new_size;
                }
            }
            Side::Sell => {
                self.cash += price * size - commission;
                let realized = (price - pos.avg_entry_price) * size - commission;
                pos.realized_pnl += realized;
                let new_size = pos.size - size;
                if new_size.abs() < 1e-9 {
                    pos.size = 0.0;
                    pos.avg_entry_price = 0.0;
                } else {
                    pos.size = new_size;
                    // 如果从多头翻空头,avg price 重置
                    if (pos.size > 0.0) != (new_size > 0.0) {
                        pos.avg_entry_price = price;
                    }
                }
            }
            Side::Hold => {}
        }
    }

    fn log(&mut self, ts: DateTime<Utc>, msg: String) {
        self.trade_log.push((ts, msg, self.cash));
    }
}

impl Broker for SimulatedBroker {
    fn place_order(&mut self, order: Order) -> Fill {
        let fill_price = match self.resolve_fill_price(&order) {
            Some(p) => p,
            None => {
                return Fill {
                    order_id: order.id,
                    timestamp: order.timestamp,
                    symbol: order.symbol.clone(),
                    side: order.side,
                    size: 0.0,
                    price: 0.0,
                    commission: 0.0,
                };
            }
        };

        // 检查现金 / 持仓是否够
        match order.side {
            Side::Buy => {
                let cost = fill_price * order.size * (1.0 + self.config.commission_rate);
                if cost > self.cash + 1e-9 {
                    self.log(order.timestamp, format!("资金不足,订单 {} 失败", order.id));
                    return Fill {
                        order_id: order.id,
                        timestamp: order.timestamp,
                        symbol: order.symbol.clone(),
                        side: order.side,
                        size: 0.0,
                        price: fill_price,
                        commission: 0.0,
                    };
                }
            }
            Side::Sell => {
                let pos = self.get_position(&order.symbol);
                if pos.size < order.size && !self.config.allow_short {
                    self.log(
                        order.timestamp,
                        format!(
                            "持仓不足,订单 {} 失败(想要卖 {},只有 {})",
                            order.id, order.size, pos.size
                        ),
                    );
                    return Fill {
                        order_id: order.id,
                        timestamp: order.timestamp,
                        symbol: order.symbol.clone(),
                        side: order.side,
                        size: 0.0,
                        price: fill_price,
                        commission: 0.0,
                    };
                }
            }
            Side::Hold => {
                return Fill {
                    order_id: order.id,
                    timestamp: order.timestamp,
                    symbol: order.symbol.clone(),
                    side: order.side,
                    size: 0.0,
                    price: 0.0,
                    commission: 0.0,
                };
            }
        }

        let commission = fill_price * order.size * self.config.commission_rate;
        self.apply_fill(
            &order.symbol,
            order.side,
            order.size,
            fill_price,
            commission,
        );

        Fill {
            order_id: order.id,
            timestamp: order.timestamp,
            symbol: order.symbol.clone(),
            side: order.side,
            size: order.size,
            price: fill_price,
            commission,
        }
    }

    fn get_position(&self, symbol: &str) -> Position {
        self.positions
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| Position {
                symbol: symbol.to_string(),
                size: 0.0,
                avg_entry_price: 0.0,
                realized_pnl: 0.0,
            })
    }

    fn get_cash(&self) -> f64 {
        self.cash
    }

    fn set_market_price(&mut self, symbol: &str, price: f64) {
        self.last_price.insert(symbol.to_string(), price);
    }

    fn get_market_price(&self, symbol: &str) -> Option<f64> {
        self.last_price.get(symbol).copied()
    }

    fn trade_log(&self) -> Vec<(DateTime<Utc>, String, f64)> {
        self.trade_log.clone()
    }
}

/// 构造订单的小工具
pub fn new_order(
    symbol: &str,
    side: Side,
    size: f64,
    ts: DateTime<Utc>,
    order_type: OrderType,
    price: Option<f64>,
) -> Order {
    Order {
        id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
        timestamp: ts,
        symbol: symbol.to_string(),
        side,
        order_type,
        size,
        price,
    }
}

#[allow(dead_code)]
pub fn err(msg: &str) -> anyhow::Error {
    anyhow::anyhow!("{}", msg)
}
