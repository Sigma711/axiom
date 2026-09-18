//! 趋势类指标 —— 判断趋势方向、强度和可能的反转。
//!
//! PDF 章节:第三部分-十三 MACD,十四 趋势方向与强度,十五 一目均衡表

use crate::indicators::ma::ema;
use crate::types::Bar;

/// MACD —— Moving Average Convergence Divergence
/// DIF = EMA(close, 12) - EMA(close, 26)
/// DEA = EMA(DIF, 9) (也叫信号线)
/// 柱体 = DIF - DEA
/// 用途:看零轴上下、柱体长短、金叉死叉、背离
pub struct MacdOutput {
    pub dif: Vec<Option<f64>>,
    pub dea: Vec<Option<f64>>,
    pub hist: Vec<Option<f64>>,
}

pub fn macd(prices: &[f64], fast: usize, slow: usize, signal: usize) -> MacdOutput {
    let ema_fast = ema(prices, fast);
    let ema_slow = ema(prices, slow);
    let dif: Vec<f64> = ema_fast
        .iter()
        .zip(ema_slow.iter())
        .map(|(f, s)| match (f, s) {
            (Some(x), Some(y)) => x - y,
            _ => 0.0,
        })
        .collect();
    let dea = ema(&dif, signal);
    let hist: Vec<Option<f64>> = dif
        .iter()
        .zip(dea.iter())
        .map(|(d, e)| match e {
            Some(x) => Some(d - x),
            None => None,
        })
        .collect();
    MacdOutput {
        dif: vec_to_option(&dif),
        dea,
        hist,
    }
}

fn vec_to_option(v: &[f64]) -> Vec<Option<f64>> {
    v.iter()
        .map(|x| if x.is_finite() { Some(*x) } else { None })
        .collect()
}

/// DMI / ADX —— Directional Movement Index
/// +DM / -DM:上升动向 / 下降动向
/// +DI / -DI:方向指标(归一化)
/// ADX:平均趋向指数,衡量趋势强度(>25 强趋势,<20 弱趋势)
pub struct DmiOutput {
    pub plus_di: Vec<Option<f64>>,
    pub minus_di: Vec<Option<f64>>,
    pub adx: Vec<Option<f64>>,
}

pub fn dmi(bars: &[Bar], period: usize) -> DmiOutput {
    let n = bars.len();
    let mut tr = vec![0.0; n];
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    for i in 1..n {
        let high = bars[i].high;
        let low = bars[i].low;
        let prev_close = bars[i - 1].close;
        let prev_high = bars[i - 1].high;
        let prev_low = bars[i - 1].low;
        tr[i] = (high - low)
            .max((high - prev_close).abs())
            .max((low - prev_close).abs());
        let up = high - prev_high;
        let down = prev_low - low;
        if up > down && up > 0.0 {
            plus_dm[i] = up;
        }
        if down > up && down > 0.0 {
            minus_dm[i] = down;
        }
    }
    // Wilder 平滑
    let tr_s = rma_wilder(&tr, period);
    let plus_dm_s = rma_wilder(&plus_dm, period);
    let minus_dm_s = rma_wilder(&minus_dm, period);
    let mut plus_di = vec![None; n];
    let mut minus_di = vec![None; n];
    let mut dx = vec![0.0; n];
    for i in 0..n {
        if let (Some(t), Some(p), Some(m)) = (tr_s[i], plus_dm_s[i], minus_dm_s[i]) {
            if t > 0.0 {
                plus_di[i] = Some(p / t * 100.0);
                minus_di[i] = Some(m / t * 100.0);
                let sum = plus_di[i].unwrap() + minus_di[i].unwrap();
                if sum > 0.0 {
                    dx[i] = ((plus_di[i].unwrap() - minus_di[i].unwrap()) / sum).abs() * 100.0;
                }
            }
        }
    }
    let adx = rma_wilder(&dx, period);
    DmiOutput {
        plus_di,
        minus_di,
        adx,
    }
}

pub fn rma_wilder(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    if n < period + 1 {
        return vec![None; n];
    }
    let mut out = vec![None; n];
    // 种子 = 前 period 个的和
    let seed: f64 = values[1..=period].iter().sum::<f64>() / period as f64;
    out[period] = Some(seed);
    let alpha = 1.0 / period as f64;
    for i in period + 1..n {
        let prev = out[i - 1].unwrap();
        out[i] = Some(alpha * values[i] + (1.0 - alpha) * prev);
    }
    out
}

/// Aroon —— (N 期内最高点距今天数, N 期内最低点距今天数)
/// Aroon Up = (N - periods_since_high) / N × 100
/// Aroon Down = (N - periods_since_low) / N × 100
pub struct AroonOutput {
    pub up: Vec<Option<f64>>,
    pub down: Vec<Option<f64>>,
}

pub fn aroon(bars: &[Bar], period: usize) -> AroonOutput {
    let n = bars.len();
    let mut up = vec![None; n];
    let mut down = vec![None; n];
    for i in period..n {
        let window = &bars[i + 1 - period..=i];
        let mut max_idx = 0;
        let mut min_idx = 0;
        for j in 1..window.len() {
            if window[j].high > window[max_idx].high {
                max_idx = j;
            }
            if window[j].low < window[min_idx].low {
                min_idx = j;
            }
        }
        let bars_since_high = window.len() - 1 - max_idx;
        let bars_since_low = window.len() - 1 - min_idx;
        up[i] = Some((period - bars_since_high) as f64 / period as f64 * 100.0);
        down[i] = Some((period - bars_since_low) as f64 / period as f64 * 100.0);
    }
    AroonOutput { up, down }
}

/// Parabolic SAR —— 抛物线止损转向
/// PDF 用途:趋势跟踪 + 止损位
/// 简化版:Welles 经典算法
pub struct SarOutput {
    pub sar: Vec<Option<f64>>,
    pub trend: Vec<Option<i8>>, // 1=多头, -1=空头
}

pub fn parabolic_sar(bars: &[Bar], af_start: f64, af_step: f64, af_max: f64) -> SarOutput {
    let n = bars.len();
    let mut sar = vec![None; n];
    let mut trend = vec![None; n];
    if n < 2 {
        return SarOutput { sar, trend };
    }
    // 初始:假设多头
    let mut is_long = true;
    let mut af = af_start;
    let mut ep = bars[0].high; // 极点
    let mut sar_val = bars[0].low;
    sar[0] = Some(sar_val);
    trend[0] = Some(1);
    for i in 1..n {
        if is_long {
            sar_val = sar_val + af * (ep - sar_val);
            sar_val = sar_val.min(bars[i - 1].low);
            if bars[i].low < sar_val {
                // 反转
                is_long = false;
                sar_val = ep;
                ep = bars[i].low;
                af = af_start;
                trend[i] = Some(-1);
            } else {
                if bars[i].high > ep {
                    ep = bars[i].high;
                    af = (af + af_step).min(af_max);
                }
                trend[i] = Some(1);
            }
        } else {
            sar_val = sar_val + af * (ep - sar_val);
            sar_val = sar_val.max(bars[i - 1].high);
            if bars[i].high > sar_val {
                is_long = true;
                sar_val = ep;
                ep = bars[i].high;
                af = af_start;
                trend[i] = Some(1);
            } else {
                if bars[i].low < ep {
                    ep = bars[i].low;
                    af = (af + af_step).min(af_max);
                }
                trend[i] = Some(-1);
            }
        }
        sar[i] = Some(sar_val);
    }
    SarOutput { sar, trend }
}

/// Supertrend —— 基于 ATR 的趋势线
/// 上轨 = (H+L)/2 + multiplier × ATR
/// 价格跌破下轨 =翻多;反之翻空
pub struct SupertrendOutput {
    pub trend: Vec<Option<f64>>,    // Supertrend 线本身
    pub direction: Vec<Option<i8>>, // 1=多, -1=空
}

pub fn supertrend(bars: &[Bar], period: usize, multiplier: f64) -> SupertrendOutput {
    let n = bars.len();
    let mut atr_vals = vec![0.0; n];
    // ATR
    let mut tr = vec![0.0; n];
    for i in 1..n {
        let high = bars[i].high;
        let low = bars[i].low;
        let prev_close = bars[i - 1].close;
        tr[i] = (high - low)
            .max((high - prev_close).abs())
            .max((low - prev_close).abs());
    }
    atr_vals[0] = tr[0];
    let alpha = 1.0 / period as f64;
    for i in 1..n {
        atr_vals[i] = alpha * tr[i] + (1.0 - alpha) * atr_vals[i - 1];
    }
    let mut trend = vec![None; n];
    let mut direction = vec![None; n];
    if n < 1 {
        return SupertrendOutput { trend, direction };
    }
    let mut dir = 1i8;
    let mut final_upper = 0.0;
    let mut final_lower = 0.0;
    let mut final_trend = 0.0;
    for i in 1..n {
        let hl2 = (bars[i].high + bars[i].low) / 2.0;
        let upper = hl2 + multiplier * atr_vals[i];
        let lower = hl2 - multiplier * atr_vals[i];
        // 调整 final bands
        if i == 1 {
            final_upper = upper;
            final_lower = lower;
        } else {
            if upper < final_upper || bars[i - 1].close > final_upper {
                final_upper = upper;
            }
            if lower > final_lower || bars[i - 1].close < final_lower {
                final_lower = lower;
            }
        }
        // 判断方向
        if i > 1 {
            if bars[i].close > final_upper {
                dir = 1;
            } else if bars[i].close < final_lower {
                dir = -1;
            }
        }
        final_trend = if dir == 1 { final_lower } else { final_upper };
        trend[i] = Some(final_trend);
        direction[i] = Some(dir);
    }
    SupertrendOutput { trend, direction }
}

/// Donchian Channel —— N 期最高/最低
pub struct DonchianOutput {
    pub upper: Vec<Option<f64>>,
    pub lower: Vec<Option<f64>>,
    pub middle: Vec<Option<f64>>,
}

pub fn donchian(bars: &[Bar], period: usize) -> DonchianOutput {
    let n = bars.len();
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    let mut middle = vec![None; n];
    for i in period - 1..n {
        // 窗口不包含当前 bar,只看前 N 根
        let window = &bars[i + 1 - period..i];
        if window.is_empty() {
            continue;
        }
        let mut hi = window[0].high;
        let mut lo = window[0].low;
        for b in window {
            if b.high > hi {
                hi = b.high;
            }
            if b.low < lo {
                lo = b.low;
            }
        }
        upper[i] = Some(hi);
        lower[i] = Some(lo);
        middle[i] = Some((hi + lo) / 2.0);
    }
    DonchianOutput {
        upper,
        lower,
        middle,
    }
}

/// Keltner Channel —— EMA ± multiplier × ATR
pub struct KeltnerOutput {
    pub upper: Vec<Option<f64>>,
    pub lower: Vec<Option<f64>>,
    pub middle: Vec<Option<f64>>,
}

pub fn keltner(bars: &[Bar], period: usize, multiplier: f64) -> KeltnerOutput {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let ema_mid = ema(&closes, period);
    // 计算 ATR
    let mut atr_vals = vec![0.0; bars.len()];
    for i in 1..bars.len() {
        let h = bars[i].high;
        let l = bars[i].low;
        let pc = bars[i - 1].close;
        atr_vals[i] = (h - l).max((h - pc).abs()).max((l - pc).abs());
    }
    let atr = rma_wilder(&atr_vals, period);
    let mut upper = vec![None; bars.len()];
    let mut lower = vec![None; bars.len()];
    let mut middle = vec![None; bars.len()];
    for i in 0..bars.len() {
        if let (Some(m), Some(a)) = (ema_mid[i], atr[i]) {
            middle[i] = Some(m);
            upper[i] = Some(m + multiplier * a);
            lower[i] = Some(m - multiplier * a);
        }
    }
    KeltnerOutput {
        upper,
        lower,
        middle,
    }
}

/// 一目均衡表 Ichimoku Cloud
/// 转换线 (9) + 基准线 (26) + 先行带 A/B + 迟行线
pub struct IchimokuOutput {
    pub tenkan: Vec<Option<f64>>,   // 转换线
    pub kijun: Vec<Option<f64>>,    // 基准线
    pub senkou_a: Vec<Option<f64>>, // 先行带 A
    pub senkou_b: Vec<Option<f64>>, // 先行带 B
    pub chikou: Vec<Option<f64>>,   // 迟行线
}

pub fn ichimoku(
    bars: &[Bar],
    tenkan_p: usize,
    kijun_p: usize,
    senkou_b_p: usize,
    displacement: usize,
) -> IchimokuOutput {
    let n = bars.len();
    let tenkan = midpoint(bars, tenkan_p);
    let kijun = midpoint(bars, kijun_p);
    let mut senkou_a = vec![None; n];
    let mut senkou_b = vec![None; n];
    for i in 0..n {
        if let (Some(t), Some(k)) = (tenkan[i], kijun[i]) {
            senkou_a[i] = Some((t + k) / 2.0);
        }
        senkou_b[i] = midpoint_at(bars, i.saturating_sub(displacement), senkou_b_p);
    }
    // 先行带 A/B 向右移 displacement 根
    let senkou_a_shift = shift_forward(&senkou_a, displacement);
    let senkou_b_shift = shift_forward(&senkou_b, displacement);
    let chikou = shift_forward_by_close(bars, displacement);
    IchimokuOutput {
        tenkan,
        kijun,
        senkou_a: senkou_a_shift,
        senkou_b: senkou_b_shift,
        chikou,
    }
}

fn midpoint(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period - 1..n {
        let window = &bars[i + 1 - period..=i];
        let hi = window
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let lo = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        out[i] = Some((hi + lo) / 2.0);
    }
    out
}

fn midpoint_at(bars: &[Bar], idx: usize, period: usize) -> Option<f64> {
    if idx + 1 < period {
        return None;
    }
    let start = idx + 1 - period;
    let window = &bars[start..=idx];
    let hi = window
        .iter()
        .map(|b| b.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let lo = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
    Some((hi + lo) / 2.0)
}

fn shift_forward(values: &[Option<f64>], k: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    for i in 0..n.saturating_sub(k) {
        out[i + k] = values[i];
    }
    out
}

fn shift_forward_by_close(bars: &[Bar], k: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in 0..n.saturating_sub(k) {
        out[i + k] = Some(bars[i].close);
    }
    out
}

/// ZigZag —— 标记显著转折点(忽略小于 threshold 的波动)
pub fn zigzag(prices: &[f64], threshold_pct: f64) -> Vec<Option<f64>> {
    let n = prices.len();
    if n == 0 {
        return vec![];
    }
    let mut out = vec![None; n];
    out[0] = Some(prices[0]);
    let mut last_pivot_idx = 0;
    let mut last_pivot_val = prices[0];
    let mut direction = 0i8; // 1=up, -1=down, 0=unknown
    for i in 1..n {
        let change = (prices[i] - last_pivot_val) / last_pivot_val;
        if direction >= 0 && change >= threshold_pct {
            // 新高点
            out[last_pivot_idx] = None;
            last_pivot_idx = i;
            last_pivot_val = prices[i];
            out[i] = Some(prices[i]);
            direction = 1;
        } else if direction <= 0 && change <= -threshold_pct {
            out[last_pivot_idx] = None;
            last_pivot_idx = i;
            last_pivot_val = prices[i];
            out[i] = Some(prices[i]);
            direction = -1;
        } else if direction == 1 && prices[i] > last_pivot_val {
            out[last_pivot_idx] = None;
            last_pivot_idx = i;
            last_pivot_val = prices[i];
            out[i] = Some(prices[i]);
        } else if direction == -1 && prices[i] < last_pivot_val {
            out[last_pivot_idx] = None;
            last_pivot_idx = i;
            last_pivot_val = prices[i];
            out[i] = Some(prices[i]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Bar;
    use chrono::{TimeZone, Utc};

    fn make_bars(prices: &[f64]) -> Vec<Bar> {
        let t = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        prices
            .iter()
            .enumerate()
            .map(|(i, &p)| Bar {
                timestamp: t + chrono::Duration::hours(i as i64),
                open: p,
                high: p + 0.5,
                low: p - 0.5,
                close: p,
                volume: 100.0,
            })
            .collect()
    }

    #[test]
    fn test_macd_basic() {
        let p: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let m = macd(&p, 12, 26, 9);
        assert_eq!(m.dif.len(), p.len());
        // 最后应该有值
        assert!(m.dif.last().unwrap().is_some());
    }

    #[test]
    fn test_aroon_detects_uptrend() {
        let bars = make_bars(&{
            let mut p: Vec<f64> = (0..50).map(|i| 100.0 - i as f64).collect();
            p.reverse(); // 单调上涨
            p
        });
        let a = aroon(&bars, 14);
        let last = a.up.last().unwrap().unwrap();
        assert!(last > 80.0);
    }

    #[test]
    fn test_ichimoku_has_five_lines() {
        let bars = make_bars(&(0..100).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
        let ich = ichimoku(&bars, 9, 26, 52, 26);
        assert_eq!(ich.tenkan.len(), bars.len());
        assert!(ich.tenkan.last().unwrap().is_some());
    }

    #[test]
    fn test_zigzag_runs() {
        let p = vec![100.0, 102.0, 105.0, 103.0, 101.0, 95.0, 97.0, 110.0];
        let z = zigzag(&p, 0.05);
        assert_eq!(z.len(), p.len());
        assert!(z.iter().any(|x| x.is_some()));
    }
}

// -----------------------------------------------------------------------------
// VortexIndicator —— VI+ / VI-
// VI+ = sum( |H_i - L_{i-1}| ) / N
// VI- = sum( |L_i - H_{i-1}| ) / N
// -----------------------------------------------------------------------------

pub struct VortexOutput {
    pub plus: Vec<Option<f64>>,
    pub minus: Vec<Option<f64>>,
}

pub fn vortex(bars: &[Bar], period: usize) -> VortexOutput {
    let n = bars.len();
    let mut plus = vec![None; n];
    let mut minus = vec![None; n];
    if n <= period {
        return VortexOutput { plus, minus };
    }
    let mut sum_p = 0.0;
    let mut sum_m = 0.0;
    for i in 1..=period {
        let dp = (bars[i].high - bars[i - 1].low).abs();
        let dm = (bars[i].low - bars[i - 1].high).abs();
        sum_p += dp;
        sum_m += dm;
    }
    if period < n {
        plus[period] = Some(sum_p / period as f64);
        minus[period] = Some(sum_m / period as f64);
    }
    for i in (period + 1)..n {
        let dp = (bars[i].high - bars[i - 1].low).abs();
        let dm = (bars[i].low - bars[i - 1].high).abs();
        sum_p += dp - (bars[i - period + 1].high - bars[i - period].low).abs();
        sum_m += dm - (bars[i - period + 1].low - bars[i - period].high).abs();
        plus[i] = Some(sum_p / period as f64);
        minus[i] = Some(sum_m / period as f64);
    }
    VortexOutput { plus, minus }
}
