//! 成交量与资金流指标。
//!
//! PDF 章节:第三部分-十八、成交量与资金指标,十九、成交量分布与筹码结构

use crate::indicators::ma::rma;
use crate::types::Bar;

/// OBV —— On Balance Volume
/// 收盘价上日高于昨收 → +vol;反之 -vol;相等 → 不变
pub fn obv(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n == 0 {
        return out;
    }
    out[0] = Some(bars[0].volume);
    for i in 1..n {
        let prev = out[i - 1].unwrap();
        let diff = bars[i].close - bars[i - 1].close;
        out[i] = Some(if diff > 0.0 {
            prev + bars[i].volume
        } else if diff < 0.0 {
            prev - bars[i].volume
        } else {
            prev
        });
    }
    out
}

/// ADL —— Accumulation/Distribution Line
/// 衡量资金流入/流出
pub fn adl(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n == 0 {
        return out;
    }
    let mut adl_val = 0.0;
    for i in 0..n {
        let h = bars[i].high;
        let l = bars[i].low;
        let c = bars[i].close;
        let range = h - l;
        let mfm = if range == 0.0 {
            0.0
        } else {
            ((c - l) - (h - c)) / range
        };
        adl_val += mfm * bars[i].volume;
        out[i] = Some(adl_val);
    }
    out
}

/// CMF —— Chaikin Money Flow
/// N 期 MFM × Volume 的和 / N 期 Volume 总和
pub fn cmf(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period - 1..n {
        let window = &bars[i + 1 - period..=i];
        let mut mfv_sum = 0.0;
        let mut vol_sum = 0.0;
        for b in window {
            let range = b.high - b.low;
            let mfm = if range == 0.0 {
                0.0
            } else {
                ((b.close - b.low) - (b.high - b.close)) / range
            };
            mfv_sum += mfm * b.volume;
            vol_sum += b.volume;
        }
        if vol_sum > 0.0 {
            out[i] = Some(mfv_sum / vol_sum);
        }
    }
    out
}

/// MFI —— Money Flow Index (RSI 的量价版)
/// 典型价上升日的资金流 = 上涨资金流;下降日的 = 下降资金流
/// MFI = 100 - 100 / (1 + 上涨流 / 下降流)
pub fn mfi(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    if n < period + 1 {
        return vec![None; n];
    }
    let mut out = vec![None; n];
    for i in period..n {
        let mut pos_flow = 0.0;
        let mut neg_flow = 0.0;
        for j in (i + 1 - period)..=i {
            let tp = bars[j].typical_price();
            let prev_tp = bars[j - 1].typical_price();
            let flow = tp * bars[j].volume;
            if tp > prev_tp {
                pos_flow += flow;
            } else if tp < prev_tp {
                neg_flow += flow;
            }
        }
        if neg_flow == 0.0 {
            out[i] = Some(100.0);
        } else {
            let ratio = pos_flow / neg_flow;
            out[i] = Some(100.0 - 100.0 / (1.0 + ratio));
        }
    }
    out
}

/// VWAP —— Volume Weighted Average Price (日内)
/// 当根 K 线 VWAP = sum(TP × vol) / sum(vol)
/// 注意:真正的 VWAP 应该日内重置,这里简化用 running total
pub fn vwap(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    let mut cum_pv = 0.0;
    let mut cum_v = 0.0;
    for i in 0..n {
        let tp = bars[i].typical_price();
        cum_pv += tp * bars[i].volume;
        cum_v += bars[i].volume;
        if cum_v > 0.0 {
            out[i] = Some(cum_pv / cum_v);
        }
    }
    out
}

/// Chaikin Oscillator —— ADL 的 EMA(快) - EMA(慢)
pub fn chaikin_oscillator(bars: &[Bar], fast: usize, slow: usize) -> Vec<Option<f64>> {
    let adl_vals = adl(bars);
    let adl_floats: Vec<f64> = adl_vals.iter().map(|x| x.unwrap_or(0.0)).collect();
    let ema_fast = crate::indicators::ma::ema(&adl_floats, fast);
    let ema_slow = crate::indicators::ma::ema(&adl_floats, slow);
    ema_fast
        .iter()
        .zip(ema_slow.iter())
        .map(|(f, s)| match (f, s) {
            (Some(x), Some(y)) => Some(x - y),
            _ => None,
        })
        .collect()
}

/// PVT —— Price Volume Trend
/// PVT = Prev + ((Close - PrevClose) / PrevClose) × Volume
pub fn pvt(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n == 0 {
        return out;
    }
    out[0] = Some(0.0);
    for i in 1..n {
        let prev = out[i - 1].unwrap();
        let pc = bars[i - 1].close;
        if pc > 0.0 {
            out[i] = Some(prev + (bars[i].close - pc) / pc * bars[i].volume);
        } else {
            out[i] = Some(prev);
        }
    }
    out
}

/// Force Index —— (Close - PrevClose) × Volume
pub fn force_index(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n == 0 {
        return out;
    }
    out[0] = Some(0.0);
    for i in 1..n {
        out[i] = Some((bars[i].close - bars[i - 1].close) * bars[i].volume);
    }
    out
}

/// EMV / EOM —— Ease of Movement
/// Distance Moved = (H+L)/2 - (prevH+prevL)/2
/// EMV = Distance / (V / (H-L))
pub fn emv(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 2 {
        return out;
    }
    for i in 1..n {
        let dm = (bars[i].high + bars[i].low) / 2.0 - (bars[i - 1].high + bars[i - 1].low) / 2.0;
        let br = if bars[i].high == bars[i].low {
            1.0
        } else {
            bars[i].volume / (bars[i].high - bars[i].low)
        };
        out[i] = Some(dm / br);
    }
    out
}

/// NVI —— Negative Volume Index
/// 量缩日累加价格变化;量增日不动
/// 用于"聪明的钱"指标(认为缩量日的交易更聪明)
pub fn nvi(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n == 0 {
        return out;
    }
    out[0] = Some(1000.0);
    for i in 1..n {
        let prev = out[i - 1].unwrap();
        if bars[i].volume < bars[i - 1].volume {
            let change = (bars[i].close - bars[i - 1].close) / bars[i - 1].close;
            out[i] = Some(prev + prev * change);
        } else {
            out[i] = Some(prev);
        }
    }
    out
}

/// PVI —— Positive Volume Index (与 NVI 相反)
pub fn pvi(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n == 0 {
        return out;
    }
    out[0] = Some(1000.0);
    for i in 1..n {
        let prev = out[i - 1].unwrap();
        if bars[i].volume > bars[i - 1].volume {
            let change = (bars[i].close - bars[i - 1].close) / bars[i - 1].close;
            out[i] = Some(prev + prev * change);
        } else {
            out[i] = Some(prev);
        }
    }
    out
}

/// Volume Oscillator —— (ShortMA - LongMA) / LongMA × 100
pub fn volume_oscillator(bars: &[Bar], short: usize, long: usize) -> Vec<Option<f64>> {
    let vols: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let s = crate::indicators::ma::sma(&vols, short);
    let l = crate::indicators::ma::sma(&vols, long);
    s.iter()
        .zip(l.iter())
        .map(|(a, b)| match (a, b) {
            (Some(x), Some(z)) if *z != 0.0 => Some((x - z) / z * 100.0),
            _ => None,
        })
        .collect()
}

/// VROC —— Volume Rate of Change
pub fn vroc(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period..n {
        if bars[i - period].volume != 0.0 {
            out[i] =
                Some((bars[i].volume - bars[i - period].volume) / bars[i - period].volume * 100.0);
        }
    }
    out
}

/// VR —— Volume Ratio (容量比率)
/// N 日内上涨日成交量和 / 下跌日成交量和 × 100
pub fn vr(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period..n {
        let mut up_v = 0.0;
        let mut down_v = 0.0;
        for j in (i + 1 - period..=i).rev() {
            let change = bars[j].close - bars[j - 1].close;
            if change > 0.0 {
                up_v += bars[j].volume;
            } else if change < 0.0 {
                down_v += bars[j].volume;
            }
        }
        if down_v > 0.0 {
            out[i] = Some(up_v / down_v * 100.0);
        } else if up_v > 0.0 {
            out[i] = Some(f64::INFINITY);
        }
    }
    out
}

/// WVAD —— Williams Variable Accumulation/Distribution
/// 简化版:(Close - Open) / (High - Low) × Volume
pub fn wvad(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    let mut cum = 0.0;
    for i in 0..n {
        let range = bars[i].high - bars[i].low;
        let value = if range == 0.0 {
            0.0
        } else {
            (bars[i].close - bars[i].open) / range * bars[i].volume
        };
        cum += value;
        out[i] = Some(cum);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_bars(prices: &[f64], volumes: &[f64]) -> Vec<Bar> {
        assert_eq!(prices.len(), volumes.len());
        let t = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        prices
            .iter()
            .zip(volumes.iter())
            .enumerate()
            .map(|(i, (&p, &v))| Bar {
                timestamp: t + chrono::Duration::hours(i as i64),
                open: p,
                high: p + 1.0,
                low: p - 1.0,
                close: p,
                volume: v,
            })
            .collect()
    }

    #[test]
    fn test_obv_uptrend() {
        // 单调上涨 → OBV 应该一直涨
        let prices: Vec<f64> = (0..10).map(|i| 100.0 + i as f64).collect();
        let vols: Vec<f64> = vec![100.0; 10];
        let bars = make_bars(&prices, &vols);
        let o = obv(&bars);
        let mut prev = 0.0;
        for v in o.iter().flatten() {
            assert!(*v >= prev);
            prev = *v;
        }
    }

    #[test]
    fn test_mfi_range() {
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
        let vols: Vec<f64> = vec![100.0; 30];
        let bars = make_bars(&prices, &vols);
        let m = mfi(&bars, 14);
        for v in m.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0);
        }
    }

    #[test]
    fn test_vwap_basic() {
        let prices: Vec<f64> = vec![100.0, 105.0, 95.0, 110.0, 90.0];
        let vols: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let bars = make_bars(&prices, &vols);
        let v = vwap(&bars);
        assert_eq!(v.len(), prices.len());
        assert!(v[0].is_some());
        let last = v.last().unwrap().unwrap();
        assert!(last > 0.0);
    }
}
/// Klinger Volume Oscillator: 看多资金 vs 看空资金的力度差
/// HLC 三值判断多空: 累加成交量 (高于前日 close 视为多)
/// 然后做快慢 EMA 之差
pub fn klinger(bars: &[Bar], fast: usize, slow: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < slow + 1 {
        return out;
    }
    let mut sv: Vec<f64> = Vec::with_capacity(n);
    sv.push(0.0);
    for i in 1..n {
        let prev_hlc = (bars[i - 1].high + bars[i - 1].low + bars[i - 1].close) / 3.0;
        let cur_hlc = (bars[i].high + bars[i].low + bars[i].close) / 3.0;
        let trend = if cur_hlc > prev_hlc {
            1.0
        } else if cur_hlc < prev_hlc {
            -1.0
        } else {
            0.0
        };
        sv.push(trend * bars[i].volume);
    }
    let fast_ema = ema_warm(&sv, fast);
    let slow_ema = ema_warm(&sv, slow);
    for i in 0..n {
        if let (Some(f), Some(s)) = (
            fast_ema.get(i).copied().flatten(),
            slow_ema.get(i).copied().flatten(),
        ) {
            out[i] = Some(f - s);
        }
    }
    out
}

fn ema_warm(vals: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut prev = vals[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(prev);
    for i in period..n {
        prev = alpha * vals[i] + (1.0 - alpha) * prev;
        out[i] = Some(prev);
    }
    out
}

/// 量比: 当日每分钟均量 / 过去 5 日同时段均量
/// 简化为: 当前 period 成交量均值 / 上一 period 成交量均值
pub fn volume_ratio(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period * 2 {
        return out;
    }
    for i in period * 2..n {
        let recent: f64 = bars[i + 1 - period..=i]
            .iter()
            .map(|b| b.volume)
            .sum::<f64>();
        let prev: f64 = bars[i + 1 - period * 2..=i - period]
            .iter()
            .map(|b| b.volume)
            .sum::<f64>();
        if prev > 0.0 {
            out[i] = Some(recent / prev);
        }
    }
    out
}
