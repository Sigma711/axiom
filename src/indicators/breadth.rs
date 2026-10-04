//! 市场宽度指标 —— PDF 第二十一章
//!
//! 这些纯函数服务于固定成分股截面的真实市场宽度案例；数据来源、
//! 成分范围、交易日对齐和缺失规则由 `market_breadth` 的公开边界负责。

/// 上涨下跌家数 (Advance/Decline) — 单根 K 线级别
/// 返回 (+1, -1) 表示 涨/跌
pub fn advance_decline_one_bar(close: f64, prev_close: f64) -> i32 {
    if close > prev_close {
        1
    } else if close < prev_close {
        -1
    } else {
        0
    }
}

/// Market advance/decline line: cumulative cross-sectional advances minus declines.
///
/// `advances[i]` and `declines[i]` must describe the same constituent universe and
/// session. A length mismatch is rejected with an empty result rather than silently
/// truncating two different calendars.
pub fn market_advance_decline_line(
    advances: &[f64],
    declines: &[f64],
    initial_value: f64,
) -> Vec<f64> {
    if advances.len() != declines.len() {
        return Vec::new();
    }
    let mut value = initial_value;
    advances
        .iter()
        .zip(declines)
        .map(|(advances, declines)| {
            value += advances - declines;
            value
        })
        .collect()
}

/// 累积派发线 ADL (Accumulation/Distribution Line)
/// PDF 第二十一章 2 节
/// CLV = ((C - L) - (H - C)) / (H - L)  ∈ [-1, 1]
/// ADL += CLV × Volume
pub fn adl_one_bar(prev_adl: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
    let range = high - low;
    let clv = if range == 0.0 {
        0.0
    } else {
        ((close - low) - (high - close)) / range
    };
    prev_adl + clv * volume
}

/// 上涨下跌比 (AD Ratio)
/// PDF 第二十一章 3 节
pub fn ad_ratio(advances: f64, declines: f64) -> f64 {
    if declines == 0.0 {
        return 0.0;
    }
    advances / declines
}

/// 新高新低比 (New High/New Low)
/// PDF 第二十一章 5 节
pub fn new_high_low_ratio(new_highs: f64, new_lows: f64) -> f64 {
    if new_lows == 0.0 {
        return 0.0;
    }
    new_highs / new_lows
}

/// TRIN (Arms Index) = (上涨股数/下跌股数) / (上涨量/下跌量)
/// PDF 第二十一章 8 节
/// 数值需结合市场、时段与历史分布解释，不在公式层硬编码经验阈值。
pub fn trin(advances: f64, declines: f64, up_volume: f64, down_volume: f64) -> f64 {
    if declines == 0.0 || down_volume == 0.0 {
        return 0.0;
    }
    (advances / declines) / (up_volume / down_volume)
}

/// McClellan Oscillator = EMA(net advances, 19) - EMA(net advances, 39).
///
/// The input is the daily advances-minus-declines series, not a cumulative A/D line.
/// Values stay undefined until both EMAs have completed their warmup.
pub fn mcclellan_oscillator(net_advances: &[f64]) -> Vec<Option<f64>> {
    let fast = crate::indicators::ma::ema(net_advances, 19);
    let slow = crate::indicators::ma::ema(net_advances, 39);
    fast.into_iter()
        .zip(slow)
        .map(|(fast, slow)| Some(fast? - slow?))
        .collect()
}

/// 市场宽度脉冲 (Breadth Thrust)
/// PDF 第二十一章 12 节
/// Zweig Breadth Thrust: the 10-day EMA of `advances / (advances + declines)`
/// moves from below 0.40 to above 0.615 within ten observations.
pub fn breadth_thrust(advance_fractions: &[f64]) -> bool {
    let ema = crate::indicators::ma::ema(advance_fractions, 10);
    let Some(current) = ema.last().copied().flatten() else {
        return false;
    };
    ema.iter()
        .rev()
        .take(10)
        .flatten()
        .any(|value| *value < 0.40)
        && current > 0.615
}

/// 上涨成交量比 (Up Down Volume)
/// PDF 第二十一章 4 节
pub fn up_down_volume_ratio(up_vol: f64, down_vol: f64) -> f64 {
    if down_vol == 0.0 {
        return 0.0;
    }
    up_vol / down_vol
}

/// 涨跌家数 (Advances/Declines)
/// PDF 第二十一章 1 节
pub fn advances_declines(closes: &[f64]) -> (usize, usize) {
    let mut adv = 0;
    let mut dec = 0;
    for w in closes.windows(2) {
        if w[1] > w[0] {
            adv += 1;
        } else if w[1] < w[0] {
            dec += 1;
        }
    }
    (adv, dec)
}

/// 牛/熊比例 (Bullish Percent Index, BPI)
/// PDF 第二十一章 10 节
/// Point-and-figure buy signals / total issues × 100.
pub fn bullish_percent_index(point_figure_buy_signals: f64, total_stocks: f64) -> f64 {
    if total_stocks == 0.0 {
        return 0.0;
    }
    point_figure_buy_signals / total_stocks * 100.0
}

/// Persistent Point & Figure signal using logarithmic percentage boxes.
///
/// A buy signal is set when an X column exceeds the previous X-column high;
/// a sell signal is set when an O column falls below the previous O-column low.
/// The returned state persists through reversals until the opposite breakout.
pub fn point_and_figure_buy_signal(
    closes: &[f64],
    box_fraction: f64,
    reversal_boxes: i64,
) -> Option<bool> {
    if closes.len() < 3
        || !(0.0..1.0).contains(&box_fraction)
        || reversal_boxes < 1
        || closes
            .iter()
            .any(|price| !price.is_finite() || *price <= 0.0)
    {
        return None;
    }
    let step = (1.0 + box_fraction).ln();
    let boxes: Vec<i64> = closes
        .iter()
        .map(|price| (price.ln() / step).floor() as i64)
        .collect();
    let mut direction = 0_i8;
    let mut column_high = boxes[0];
    let mut column_low = boxes[0];
    let mut previous_x_high = None;
    let mut previous_o_low = None;
    let mut signal = None;
    for &price_box in &boxes[1..] {
        match direction {
            0 if price_box > column_high => {
                direction = 1;
                column_high = price_box;
            }
            0 if price_box < column_low => {
                direction = -1;
                column_low = price_box;
            }
            1 if price_box > column_high => {
                column_high = price_box;
                if previous_x_high.is_some_and(|prior| column_high > prior) {
                    signal = Some(true);
                }
            }
            1 if price_box <= column_high - reversal_boxes => {
                previous_x_high = Some(column_high);
                direction = -1;
                column_low = price_box;
                if previous_o_low.is_some_and(|prior| column_low < prior) {
                    signal = Some(false);
                }
            }
            -1 if price_box < column_low => {
                column_low = price_box;
                if previous_o_low.is_some_and(|prior| column_low < prior) {
                    signal = Some(false);
                }
            }
            -1 if price_box >= column_low + reversal_boxes => {
                previous_o_low = Some(column_low);
                direction = 1;
                column_high = price_box;
                if previous_x_high.is_some_and(|prior| column_high > prior) {
                    signal = Some(true);
                }
            }
            _ => {}
        }
    }
    signal
}

/// Tick Index: contemporaneous upticks minus downticks for each observation.
pub fn tick_index(up_ticks: &[u64], down_ticks: &[u64]) -> Vec<i64> {
    up_ticks
        .iter()
        .zip(down_ticks)
        .map(|(&up, &down)| up as i64 - down as i64)
        .collect()
}
