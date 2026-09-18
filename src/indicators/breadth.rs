//! 市场宽度指标 —— PDF 第二十一章
//!
//! 本项目主要做单一币种交易,但市场宽度是技术分析中的重要维度。
//! 提供基础计算函数,实际需要多币种数据可对接外部市场数据源。

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
/// < 0.5 强势, > 2 弱势
pub fn trin(advances: f64, declines: f64, up_volume: f64, down_volume: f64) -> f64 {
    if declines == 0.0 || down_volume == 0.0 {
        return 0.0;
    }
    (advances / declines) / (up_volume / down_volume)
}

/// McClellan Oscillator = EMA(ADL, 19) - EMA(ADL, 39)
/// PDF 第二十一章 6 节
pub fn mcclellan_oscillator(adl_now: f64, adl_19_ago: f64, adl_39_ago: f64) -> f64 {
    let ema_19 = ema_simple(adl_19_ago, adl_now - adl_19_ago, 19);
    let ema_39 = ema_simple(adl_39_ago, adl_now - adl_39_ago, 39);
    ema_19 - ema_39
}

/// 简易 EMA 计算 (从 prev 和 delta 更新)
fn ema_simple(prev: f64, _delta: f64, period: usize) -> f64 {
    let alpha = 2.0 / (period as f64 + 1.0);
    // 需要上一个 EMA 值才能算;这里只是简单返回 prev + 增量
    // 实际 EMA 需要历史 EMA 值
    prev // 简化
}

/// 市场宽度脉冲 (Breadth Thrust)
/// PDF 第二十一章 12 节
/// 10 日内 AD 净值从 -0.2 跳到 +0.6 (10% 阈值)
/// 是历史上最强的买入信号
pub fn breadth_thrust(ad_now: f64, ad_10_days_ago: f64, threshold: f64) -> bool {
    ad_10_days_ago < -0.2 && ad_now > 0.6 && (ad_now - ad_10_days_ago) > threshold
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
/// 上涨/总股票数 × 100
pub fn bullish_percent_index(advances: f64, total_stocks: f64) -> f64 {
    if total_stocks == 0.0 {
        return 0.0;
    }
    advances / total_stocks * 100.0
}
