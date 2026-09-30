//! Centralized market-execution constraints shared by backtest and paper trading.
//!
//! Static profiles cover rules selected from the requested market and security
//! board. Dynamic symbol filters, reference-data exceptions, order-book
//! liquidity and intraday halt state remain outside daily OHLCV.

use crate::types::{Bar, Side};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChinaBoard {
    MainOrGrowth,
    Growth,
    Star,
    Bse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "market", content = "board", rename_all = "snake_case")]
pub enum ExecutionMarket {
    /// Compatibility profile for callers that do not identify a market.
    Unrestricted,
    CryptoSpot,
    UsEquity,
    ChinaA(ChinaBoard),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAssumption {
    pub id: String,
    pub description_zh: String,
    pub simulated: bool,
    pub limitation_zh: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionProfile {
    #[serde(flatten)]
    pub market: ExecutionMarket,
}

impl Default for ExecutionProfile {
    fn default() -> Self {
        Self::new(ExecutionMarket::Unrestricted)
    }
}

impl ExecutionProfile {
    pub fn new(market: ExecutionMarket) -> Self {
        Self { market }
    }

    /// Selects only profiles that are unambiguous from the market source and
    /// six-digit A-share code. Unknown sources retain legacy behavior.
    pub fn for_market(source: &str, symbol: &str) -> Self {
        let market = match source {
            "real" | "binance" => ExecutionMarket::CryptoSpot,
            "us_stock" => ExecutionMarket::UsEquity,
            "a_share" if symbol.starts_with("688") => ExecutionMarket::ChinaA(ChinaBoard::Star),
            "a_share" if symbol.starts_with("300") || symbol.starts_with("301") => {
                ExecutionMarket::ChinaA(ChinaBoard::Growth)
            }
            "a_share"
                if symbol.starts_with("920")
                    || symbol.starts_with('4')
                    || symbol.starts_with('8') =>
            {
                ExecutionMarket::ChinaA(ChinaBoard::Bse)
            }
            "a_share"
                if ["000", "001", "002", "003", "600", "601", "603", "605"]
                    .iter()
                    .any(|prefix| symbol.starts_with(prefix)) =>
            {
                ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)
            }
            // The provider label alone does not prove that a six-digit code is
            // an A-share ordinary stock (for example, 200/900 codes are B shares).
            "a_share" => ExecutionMarket::Unrestricted,
            _ => ExecutionMarket::Unrestricted,
        };
        Self::new(market)
    }

    /// Returns a cash-safe buy quantity at or below the requested quantity.
    pub fn buy_quantity(&self, requested: f64) -> Option<f64> {
        if !requested.is_finite() || requested <= 0.0 {
            return None;
        }
        let quantity = match self.market {
            ExecutionMarket::Unrestricted | ExecutionMarket::CryptoSpot => requested,
            ExecutionMarket::UsEquity => requested.floor(),
            ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth) => {
                (requested / 100.0).floor() * 100.0
            }
            ExecutionMarket::ChinaA(ChinaBoard::Growth) => (requested / 100.0).floor() * 100.0,
            ExecutionMarket::ChinaA(ChinaBoard::Star) => requested.floor().max(0.0),
            ExecutionMarket::ChinaA(ChinaBoard::Bse) => requested.floor().max(0.0),
        };
        let minimum = match self.market {
            ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)
            | ExecutionMarket::ChinaA(ChinaBoard::Growth)
            | ExecutionMarket::ChinaA(ChinaBoard::Bse) => 100.0,
            ExecutionMarket::ChinaA(ChinaBoard::Star) => 200.0,
            ExecutionMarket::UsEquity => 1.0,
            ExecutionMarket::Unrestricted | ExecutionMarket::CryptoSpot => f64::MIN_POSITIVE,
        };
        (quantity + EPSILON >= minimum).then_some(quantity)
    }

    pub fn valid_buy_quantity(&self, quantity: f64) -> bool {
        self.buy_quantity(quantity)
            .is_some_and(|normalized| (normalized - quantity).abs() < EPSILON)
    }

    /// Sell rules differ from buy rules. Main/Growth board lots normally use
    /// 100 shares, while a complete integer residual may be exited once.
    pub fn valid_sell_quantity(
        &self,
        quantity: f64,
        total_position: f64,
        sellable_position: f64,
    ) -> bool {
        if !quantity.is_finite()
            || quantity <= 0.0
            || quantity > sellable_position + EPSILON
            || quantity > total_position + EPSILON
        {
            return false;
        }
        match self.market {
            ExecutionMarket::Unrestricted | ExecutionMarket::CryptoSpot => true,
            ExecutionMarket::UsEquity
            | ExecutionMarket::ChinaA(ChinaBoard::Star)
            | ExecutionMarket::ChinaA(ChinaBoard::Bse) => is_integer(quantity),
            ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth)
            | ExecutionMarket::ChinaA(ChinaBoard::Growth) => {
                is_integer(quantity)
                    && (is_multiple(quantity, 100.0)
                        || (quantity - sellable_position).abs() < EPSILON)
            }
        }
    }

    pub fn uses_t_plus_one(&self) -> bool {
        matches!(self.market, ExecutionMarket::ChinaA(_))
    }

    pub fn shanghai_trade_date(timestamp: DateTime<Utc>) -> Option<NaiveDate> {
        timestamp
            .checked_add_signed(Duration::hours(8))
            .map(|local| local.date_naive())
    }

    /// Returns a conservative execution rejection supported by the two daily
    /// OHLCV rows. It does not infer an exchange halt from a missing row, and
    /// it does not claim that every observed limit-price trade was fillable.
    pub fn daily_bar_execution_block(
        &self,
        previous: Option<&Bar>,
        current: &Bar,
        side: Side,
    ) -> Option<&'static str> {
        if current.volume <= EPSILON {
            return Some("日线 OHLCV 报告无成交量，不模拟成交");
        }
        let previous = previous?;
        if current.timestamp - previous.timestamp < Duration::hours(12) {
            return None;
        }
        let limit = match self.market {
            ExecutionMarket::ChinaA(ChinaBoard::MainOrGrowth) => 0.10,
            ExecutionMarket::ChinaA(ChinaBoard::Growth) => {
                let trade_date = Self::shanghai_trade_date(current.timestamp)?;
                if trade_date < NaiveDate::from_ymd_opt(2020, 8, 24)? {
                    0.10
                } else {
                    0.20
                }
            }
            ExecutionMarket::ChinaA(ChinaBoard::Star) => 0.20,
            ExecutionMarket::ChinaA(ChinaBoard::Bse) => 0.30,
            _ => return None,
        };
        let expected = match side {
            Side::Buy => round_price(previous.close * (1.0 + limit)),
            Side::Sell => round_price(previous.close * (1.0 - limit)),
            Side::Hold => return None,
        };
        let one_price = [current.open, current.high, current.low, current.close]
            .into_iter()
            .all(|price| (price - expected).abs() <= 0.005 + EPSILON);
        if !one_price {
            return None;
        }
        match side {
            Side::Buy => Some("一字涨停日线按保守队列假设拒绝买入"),
            Side::Sell => Some("一字跌停日线按保守队列假设拒绝卖出"),
            Side::Hold => None,
        }
    }

    /// Known A-share transfer stamp-tax schedule. Dates before the verified
    /// single-sided regime deliberately return `None` instead of inventing a
    /// historical rate.
    pub fn transaction_tax_rate(&self, side: Side, timestamp: DateTime<Utc>) -> Option<f64> {
        if !matches!(self.market, ExecutionMarket::ChinaA(_)) || side == Side::Buy {
            return Some(0.0);
        }
        if side != Side::Sell {
            return Some(0.0);
        }
        Self::shanghai_trade_date(timestamp)?;
        let half_rate = Utc.with_ymd_and_hms(2023, 8, 27, 16, 0, 0).single()?;
        let single_sided = Utc.with_ymd_and_hms(2008, 9, 18, 16, 0, 0).single()?;
        if timestamp >= half_rate {
            Some(0.0005)
        } else if timestamp >= single_sided {
            Some(0.001)
        } else {
            None
        }
    }

    pub fn assumptions(&self) -> Vec<ExecutionAssumption> {
        let mut assumptions = match self.market {
            ExecutionMarket::Unrestricted => vec![assumption(
                "legacy_quantity_rules",
                "未指定市场时保留原有任意正数量行为。",
                true,
                Some("调用方未提供市场，结果不代表任何交易所数量规则。"),
                None,
            )],
            ExecutionMarket::CryptoSpot => vec![assumption(
                "crypto_dynamic_filters_unavailable",
                "当前仅计算正数量，未取得交易对实时 LOT_SIZE、MARKET_LOT_SIZE 与 NOTIONAL 过滤器。",
                false,
                Some("结果不代表订单已通过交易所动态过滤器。"),
                Some("https://developers.binance.com/docs/binance-spot-api-docs/filters"),
            )],
            ExecutionMarket::UsEquity => vec![assumption(
                "us_whole_share_product_default",
                "本产品默认按整股模拟美股订单。",
                true,
                Some("碎股能力取决于券商与证券，不是全市场统一禁令。"),
                Some("https://www.investor.gov/introduction-investing/general-resources/news-alerts/alerts-bulletins/investor-bulletins/fractional-share-investing-buying-slice-instead-whole-share"),
            )],
            ExecutionMarket::ChinaA(board) => vec![
                assumption(
                    "china_a_board_quantity_rule",
                    match board {
                        ChinaBoard::MainOrGrowth => "主板买入按100股整数倍，整数余股可一次卖出。",
                        ChinaBoard::Growth => "创业板买入按100股整数倍，整数余股可一次卖出。",
                        ChinaBoard::Star => "科创板普通股票买入最低200股，超过200股后可逐股递增。",
                        ChinaBoard::Bse => "北交所买入最低100股，超过100股后可逐股递增。",
                    },
                    true,
                    Some("仅覆盖普通股票竞价数量，不覆盖基金、债券、存托凭证的特殊规则；689前缀存托凭证不会自动套用本profile。"),
                    Some(match board {
                        ChinaBoard::Bse => "https://www.bse.cn/jygl_list/200028217.html",
                        _ => "https://www.sse.com.cn/lawandrules/sselawsrules2025/stocks/exchange/c/c_20260424_10816482.shtml",
                    }),
                ),
                assumption(
                    "china_a_stamp_tax_2008",
                    "A股卖出印花税自2008-09-19起按成交额的0.1%计算。",
                    true,
                    Some("2008-09-19之前的卖单因缺少已验证税率而拒绝。"),
                    Some("https://www.mof.gov.cn/zhengwuxinxi/caizhengxinwen/200809/t20080919_76432.htm"),
                ),
                assumption(
                    "china_a_stamp_tax_2023",
                    "A股卖出印花税自2023-08-28起减半为0.05%。",
                    true,
                    Some("未模拟券商最低佣金与其他动态费用。"),
                    Some("https://shanxi.chinatax.gov.cn/web/detail/sx-11400-545-1780448"),
                ),
                assumption(
                    "china_a_daily_ohlcv_execution_guards",
                    "日线无成交量时拒绝买卖；普通股一字涨停拒绝买入、一字跌停拒绝卖出。",
                    true,
                    Some("这是基于OHLCV的保守队列假设；无法识别ST、新股前五日、退市整理、临时停牌或实际排队可成交量。"),
                    Some("https://www.sse.com.cn/lawandrules/sselawsrules2025/stocks/exchange/c/c_20260424_10816482.shtml"),
                ),
                assumption(
                    "china_a_t_plus_one_sellable_inventory",
                    "按上海时区交易日期记录买入批次，当日新增普通股票不可卖出，早先库存仍可卖。",
                    true,
                    Some("未模拟节假日交收日历；跨日判断使用行情日期。"),
                    Some("https://www.sse.com.cn/lawandrules/sselawsrules2025/stocks/exchange/c/c_20260424_10816482.shtml"),
                ),
            ],
        };
        if matches!(self.market, ExecutionMarket::ChinaA(ChinaBoard::Growth)) {
            assumptions.push(assumption(
                "china_growth_2020_limit_change",
                "创业板普通股一字涨跌停判断：2020-08-24 前按 10%，此后按 20%。",
                true,
                Some("新股上市前五日、风险警示和退市整理等例外仍需独立证券状态资料。"),
                Some("https://www.szse.cn/aboutus/trends/news/t20200821_580924.html"),
            ));
        }
        assumptions.extend([
            assumption(
                "pending_signal_executes_next_open",
                "收盘后形成的策略信号只会在下一根K线开盘尝试执行。",
                true,
                Some("最后一根K线产生的信号没有下一根开盘，因此保持未成交。"),
                None,
            ),
            assumption(
                "open_price_risk_checks_only",
                "止盈止损只在下一根K线开盘执行检查。",
                true,
                Some("不会用同一根K线的盘中最高价或最低价回填成交，因此也不模拟盘中首次触发顺序。"),
                None,
            ),
            assumption(
                "configured_commission_and_slippage",
                "现金与盈亏包含配置的佣金率和滑点。",
                true,
                Some("费率是教学配置，不代表真实券商账单。"),
                None,
            ),
        ]);
        if !matches!(self.market, ExecutionMarket::ChinaA(_)) {
            assumptions.extend([
                assumption(
                    "taxes_not_simulated",
                    "市场税费与动态监管费未模拟。",
                    false,
                    Some("真实净收益可能更低。"),
                    None,
                ),
                assumption(
                    "price_limits_and_halts_not_simulated",
                    "除日线报告零成交量时拒绝成交外，停牌、订单簿流动性及部分成交未模拟。",
                    false,
                    Some("K线开盘价不保证真实订单可成交。"),
                    None,
                ),
            ]);
        }
        assumptions
    }
}

fn round_price(price: f64) -> f64 {
    (price * 100.0).round() / 100.0
}

fn is_integer(value: f64) -> bool {
    (value - value.round()).abs() < EPSILON
}

fn is_multiple(value: f64, step: f64) -> bool {
    (value / step - (value / step).round()).abs() < EPSILON
}

fn assumption(
    id: &str,
    description_zh: &str,
    simulated: bool,
    limitation_zh: Option<&str>,
    source_url: Option<&str>,
) -> ExecutionAssumption {
    ExecutionAssumption {
        id: id.into(),
        description_zh: description_zh.into(),
        simulated,
        limitation_zh: limitation_zh.map(str::to_owned),
        source_url: source_url.map(str::to_owned),
    }
}
