//! 波动率指标 —— 衡量价格变动幅度。
//!
//! PDF 章节:第三部分-十七、波动率指标:价格会动多大

use crate::indicators::ma::sma;
use crate::indicators::trend::rma_wilder;
use crate::types::Bar;

/// True Range —— 单根 K 线的真实波动幅度
/// TR = max(H-L, |H-Cprev|, |L-Cprev|)
pub fn true_range(bars: &[Bar]) -> Vec<f64> {
    let n = bars.len();
    let mut tr = vec![0.0; n];
    if n == 0 { return tr; }
    tr[0] = bars[0].high - bars[0].low;
    for i in 1..n {
        let h = bars[i].high;
        let l = bars[i].low;
        let pc = bars[i - 1].close;
        tr[i] = (h - l).max((h - pc).abs()).max((l - pc).abs());
    }
    tr
}

/// ATR —— Average True Range (Wilder 平滑)
pub fn atr(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let tr = true_range(bars);
    rma_wilder(&tr, period)
}

/// ATR% —— ATR 占价格的百分比(用于跨品种比较)
pub fn atr_percent(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let a = atr(bars, period);
    a.iter().zip(bars.iter()).map(|(v, b)| match v {
        Some(x) if b.close != 0.0 => Some(x / b.close * 100.0),
        _ => None,
    }).collect()
}

/// 历史波动率 HV —— 收益率的标准差(年化)
/// 默认 periods_per_year = 365*24 (小时 K 线)
pub fn historical_volatility(prices: &[f64], period: usize,
                             periods_per_year: f64) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    for i in period..n {
        let rets: Vec<f64> = (i + 1 - period..=i)
            .map(|j| prices[j] / prices[j - 1] - 1.0)
            .collect();
        if rets.len() < 2 { continue; }
        let mean = rets.iter().sum::<f64>() / rets.len() as f64;
        let var: f64 = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
            / (rets.len() - 1) as f64;
        out[i] = Some(var.sqrt() * periods_per_year.sqrt());
    }
    out
}

/// Bollinger Bands —— SMA ± N × StdDev
pub struct BbandsOutput {
    pub upper: Vec<Option<f64>>,
    pub middle: Vec<Option<f64>>,
    pub lower: Vec<Option<f64>>,
}

pub fn bollinger_bands(prices: &[f64], period: usize, num_std: f64) -> BbandsOutput {
    let n = prices.len();
    let mut upper = vec![None; n];
    let mut middle = vec![None; n];
    let mut lower = vec![None; n];
    for i in period - 1..n {
        let window = &prices[i + 1 - period..=i];
        let mean: f64 = window.iter().sum::<f64>() / period as f64;
        let var: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
        let std = var.sqrt();
        middle[i] = Some(mean);
        upper[i] = Some(mean + num_std * std);
        lower[i] = Some(mean - num_std * std);
    }
    BbandsOutput { upper, middle, lower }
}

/// BBands-Keltner Squeeze —— 判断波动率收缩(可能即将爆发)
/// 当 BBands 落在 Keltner Channel 内部时 = 挤压状态
pub fn squeeze(bars: &[Bar], bb_period: usize, kc_period: usize,
               bb_std: f64, kc_mult: f64) -> Vec<Option<bool>> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let bb = bollinger_bands(&closes, bb_period, bb_std);
    let kc = super::trend::keltner(bars, kc_period, kc_mult);
    let n = bars.len();
    let mut out = vec![None; n];
    for i in 0..n {
        if let (Some(bu), Some(bl), Some(ku), Some(kl)) =
            (bb.upper[i], bb.lower[i], kc.upper[i], kc.lower[i]) {
            out[i] = Some(bu < ku && bl > kl);
        }
    }
    out
}

/// Chaikin Volatility —— EMA(H-L) 变化率
pub fn chaikin_volatility(bars: &[Bar], period: usize, ema_period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let hl_range: Vec<f64> = bars.iter().map(|b| b.high - b.low).collect();
    let ema_hl = crate::indicators::ma::ema(&hl_range, ema_period);
    let mut out = vec![None; n];
    for i in period..n {
        if let (Some(cur), Some(prev)) = (ema_hl[i], ema_hl[i - period]) {
            if prev != 0.0 {
                out[i] = Some((cur - prev) / prev * 100.0);
            }
        }
    }
    out
}

/// Mass Index —— 识别趋势反转
/// 基于 HL 范围与 EMA(HL) 的比率
pub fn mass_index(bars: &[Bar], ema_period: usize, sum_period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let hl_range: Vec<f64> = bars.iter().map(|b| b.high - b.low).collect();
    let single_ema = crate::indicators::ma::ema(&hl_range, ema_period);
    let ratio: Vec<f64> = hl_range.iter().zip(single_ema.iter())
        .map(|(r, e)| match e {
            Some(x) if *x != 0.0 => r / x,
            _ => 0.0,
        }).collect();
    let double_ema = crate::indicators::ma::ema(&ratio, ema_period);
    let mut out = vec![None; n];
    for i in sum_period..n {
        if i < double_ema.len() {
            let sum: f64 = (i + 1 - sum_period..=i)
                .filter_map(|j| double_ema[j])
                .sum();
            out[i] = Some(sum);
        }
    }
    out
}

/// Ulcer Index —— 综合下行深度和持续时间
pub fn ulcer_index(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    let ma = sma(prices, period);
    for i in period - 1..n {
        let window = &prices[i + 1 - period..=i];
        let ma_window = (i + 1 - period..=i)
            .filter_map(|j| ma[j])
            .collect::<Vec<f64>>();
        if ma_window.len() != period { continue; }
        let sum_sq: f64 = window.iter().zip(ma_window.iter())
            .map(|(p, m)| {
                let dd_pct = (p - m) / m * 100.0;
                if dd_pct > 0.0 { dd_pct * dd_pct } else { 0.0 }
            }).sum();
        out[i] = Some((sum_sq / period as f64).sqrt());
    }
    out
}

/// ADR —— Average Daily Range (类似 ATR,但用 High-Low 而不是 True Range)
pub fn adr(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let hl: Vec<f64> = bars.iter().map(|b| b.high - b.low).collect();
    sma(&hl, period).into_iter().enumerate().map(|(i, v)| {
        if i < n { v } else { None }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_bars(prices: &[f64]) -> Vec<Bar> {
        let t = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        prices.iter().enumerate().map(|(i, &p)| Bar {
            timestamp: t + chrono::Duration::hours(i as i64),
            open: p, high: p + 1.0, low: p - 1.0, close: p, volume: 100.0,
        }).collect()
    }

    #[test]
    fn test_atr_basic() {
        let bars = make_bars(&(0..30).map(|i| 100.0 + (i as f64).sin() * 5.0).collect::<Vec<_>>());
        let a = atr(&bars, 14);
        assert_eq!(a.len(), bars.len());
        for v in a.iter().flatten() {
            assert!(*v > 0.0);
        }
    }

    #[test]
    fn test_bollinger_bands_contains_price() {
        // 95% 的情况下价格应该在上下轨之间
        let prices: Vec<f64> = (0..100).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let bb = bollinger_bands(&prices, 20, 2.0);
        let mut count = 0;
        for i in 20..100 {
            if let (Some(u), Some(l)) = (bb.upper[i], bb.lower[i]) {
                if prices[i] <= u && prices[i] >= l {
                    count += 1;
                }
            }
        }
        assert!(count >= 75, "至少 95% 的价格在布林内内内");
    }

    #[test]
    fn test_true_range_first_bar() {
        let bars = make_bars(&[100.0]);
        let tr = true_range(&bars);
        assert_eq!(tr[0], 2.0); // high - low = 101 - 99
    }
}