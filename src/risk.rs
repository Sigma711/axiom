//! 风控模块:在策略信号和下单之间再插一道关卡。
//!
//! 内置规则:
//!   - 止损:持仓亏损达到 stop_loss_pct 时强制平仓
//!   - 止盈:持仓盈利达到 take_profit_pct 时强制平仓
//!   - 仓位上限:阻止把单标的仓位加到超过 max_position_pct

use crate::broker::Broker;
use crate::portfolio::Portfolio;
use crate::types::{Order, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub stop_loss_pct: f64, // 0 = 不启用
    pub take_profit_pct: f64,
    pub max_position_pct: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            stop_loss_pct: 0.0,
            take_profit_pct: 0.0,
            max_position_pct: 0.95,
        }
    }
}

pub struct RiskManager {
    pub config: RiskConfig,
    pub symbol: String,
}

impl RiskManager {
    pub fn new(config: RiskConfig, symbol: String) -> Self {
        Self { config, symbol }
    }

    /// 如果当前持仓触及止损/止盈,返回原因;否则返回 None。
    pub fn force_close_reason(&self, portfolio: &Portfolio, broker: &dyn Broker) -> Option<String> {
        let pos = portfolio.position();
        if pos.is_flat() {
            return None;
        }
        let price = broker.get_market_price(&self.symbol)?;
        if pos.avg_entry_price == 0.0 {
            return None;
        }
        let pnl_pct = (price - pos.avg_entry_price) / pos.avg_entry_price * pos.size.signum();
        if self.config.stop_loss_pct > 0.0 && pnl_pct <= -self.config.stop_loss_pct {
            return Some(format!(
                "止损 ({:.2}% <= -{:.2}%)",
                pnl_pct * 100.0,
                self.config.stop_loss_pct * 100.0
            ));
        }
        if self.config.take_profit_pct > 0.0 && pnl_pct >= self.config.take_profit_pct {
            return Some(format!(
                "止盈 ({:.2}% >= {:.2}%)",
                pnl_pct * 100.0,
                self.config.take_profit_pct * 100.0
            ));
        }
        None
    }

    /// 是否允许这个订单通过。
    pub fn allow_order(
        &self,
        order: &Order,
        portfolio: &Portfolio,
        broker: &dyn Broker,
    ) -> (bool, String) {
        if order.side != Side::Buy {
            return (true, "卖出不受仓位上限限制".to_string());
        }
        let equity = portfolio.equity();
        if equity <= 0.0 {
            return (false, "净值为 0,无法买入".to_string());
        }
        let price = match broker.get_market_price(&self.symbol) {
            Some(p) => p,
            None => return (false, "没有当前市场价".to_string()),
        };
        let pos = portfolio.position();
        let pos_value = pos.market_value(price);
        let new_pct = (pos_value + order.size * price) / equity;
        if new_pct > self.config.max_position_pct + 1e-9 {
            return (
                false,
                format!(
                    "超仓位上限 ({:.2}% > {:.2}%)",
                    new_pct * 100.0,
                    self.config.max_position_pct * 100.0
                ),
            );
        }
        (true, "通过".to_string())
    }
}
