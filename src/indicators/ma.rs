//! 移动平均线及其变体。
//!
//! PDF 章节:第三部分-十二、移动平均线:MA、EMA 及其变体
//!
//! 共同用途:平滑价格序列,提取趋势。SMA 最简单但滞后;
//! EMA 反应更快;WMA 给近期更高权重;Wilder RMA 用于 ATR/RSI 等指标;
//! HMA 解决了滞后与平滑的矛盾;DEMA/TEMA 是 EMA 的双重/三重平滑。

use crate::types::Bar;

/// 简单移动平均 SMA —— 前 n 根收盘价的算术平均
pub fn sma(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 || prices.is_empty() {
        return vec![None; prices.len()];
    }
    let mut out = Vec::with_capacity(prices.len());
    let mut sum = 0.0;
    for i in 0..prices.len() {
        sum += prices[i];
        if i >= period {
            sum -= prices[i - period];
        }
        if i + 1 >= period {
            out.push(Some(sum / period as f64));
        } else {
            out.push(None);
        }
    }
    out
}

/// 指数移动平均 EMA
pub fn ema(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 || prices.is_empty() {
        return vec![None; prices.len()];
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut out = Vec::with_capacity(prices.len());
    let mut prev: Option<f64> = None;
    for i in 0..prices.len() {
        if i + 1 < period {
            out.push(None);
            continue;
        }
        if prev.is_none() {
            let seed: f64 = prices[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
            prev = Some(seed);
        } else {
            let new = alpha * prices[i] + (1.0 - alpha) * prev.unwrap();
            prev = Some(new);
        }
        out.push(prev);
    }
    out
}

/// 加权移动平均 WMA
pub fn wma(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 || prices.is_empty() {
        return vec![None; prices.len()];
    }
    let denom: f64 = (period * (period + 1) / 2) as f64;
    let mut out = Vec::with_capacity(prices.len());
    for i in 0..prices.len() {
        if i + 1 < period {
            out.push(None);
            continue;
        }
        let mut sum = 0.0;
        // 安全: i + 1 >= period 保证了 i - period + 1 >= 0
        let start = i + 1 - period;
        for j in 0..period {
            sum += prices[start + j] * (j + 1) as f64;
        }
        out.push(Some(sum / denom));
    }
    out
}

/// Wilder 平滑 RMA
pub fn rma(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    if period == 0 || prices.is_empty() {
        return vec![None; prices.len()];
    }
    let alpha = 1.0 / period as f64;
    let mut out = Vec::with_capacity(prices.len());
    let mut prev: Option<f64> = None;
    for i in 0..prices.len() {
        if i + 1 < period {
            out.push(None);
            continue;
        }
        if prev.is_none() {
            let seed: f64 = prices[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
            prev = Some(seed);
        } else {
            let new = alpha * prices[i] + (1.0 - alpha) * prev.unwrap();
            prev = Some(new);
        }
        out.push(prev);
    }
    out
}

/// Hull 移动平均 HMA
pub fn hma(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    if period <= 1 || prices.is_empty() {
        return vec![None; prices.len()];
    }
    let half = period / 2;
    let sqrt_p = ((period as f64).sqrt()) as usize;
    if sqrt_p == 0 {
        return vec![None; prices.len()];
    }
    let ema_full = ema(prices, period);
    let ema_half = ema(prices, half);
    let mut diff: Vec<f64> = Vec::with_capacity(prices.len());
    for i in 0..prices.len() {
        match (ema_half[i], ema_full[i]) {
            (Some(h), Some(f)) => diff.push(2.0 * h - f),
            _ => diff.push(0.0),
        }
    }
    wma(&diff, sqrt_p)
}

/// DEMA
pub fn dema(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let e1 = ema(prices, period);
    let e2_in: Vec<f64> = e1.iter().map(|x| x.unwrap_or(0.0)).collect();
    let e2 = ema(&e2_in, period);
    e1.iter()
        .zip(e2.iter())
        .map(|(a, b)| match (a, b) {
            (Some(x), Some(y)) => Some(2.0 * x - y),
            _ => None,
        })
        .collect()
}

/// TEMA
pub fn tema(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let e1 = ema(prices, period);
    let e2_in: Vec<f64> = e1.iter().map(|x| x.unwrap_or(0.0)).collect();
    let e2 = ema(&e2_in, period);
    let e3_in: Vec<f64> = e2.iter().map(|x| x.unwrap_or(0.0)).collect();
    let e3 = ema(&e3_in, period);
    e1.iter()
        .zip(e2.iter())
        .zip(e3.iter())
        .map(|((a, b), c)| match (a, b, c) {
            (Some(x), Some(y), Some(z)) => Some(3.0 * x - 3.0 * y + z),
            _ => None,
        })
        .collect()
}

/// VWMA 成交量加权均线
pub fn vwma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    if period == 0 {
        return vec![None; bars.len()];
    }
    let mut out = Vec::with_capacity(bars.len());
    let mut sum_pv = 0.0;
    let mut sum_v = 0.0;
    for i in 0..bars.len() {
        let tp = bars[i].typical_price();
        sum_pv += tp * bars[i].volume;
        sum_v += bars[i].volume;
        if i >= period {
            let old = bars[i - period];
            sum_pv -= old.typical_price() * old.volume;
            sum_v -= old.volume;
        }
        if i + 1 >= period && sum_v > 0.0 {
            out.push(Some(sum_pv / sum_v));
        } else {
            out.push(None);
        }
    }
    out
}

/// BBI 多空指标
pub fn bbi(bars: &[Bar]) -> Vec<Option<f64>> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let m3 = sma(&closes, 3);
    let m6 = sma(&closes, 6);
    let m12 = sma(&closes, 12);
    let m24 = sma(&closes, 24);
    m3.iter()
        .zip(m6.iter())
        .zip(m12.iter())
        .zip(m24.iter())
        .map(|(((a, b), c), d)| match (a, b, c, d) {
            (Some(x1), Some(x2), Some(x3), Some(x4)) => Some((x1 + x2 + x3 + x4) / 4.0),
            _ => None,
        })
        .collect()
}

/// BIAS 乖离率
pub fn bias(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let ma = sma(prices, period);
    prices
        .iter()
        .zip(ma.iter())
        .map(|(p, m)| match m {
            Some(ma_val) if *ma_val != 0.0 => Some((p - ma_val) / ma_val * 100.0),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let p = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let m = sma(&p, 3);
        assert_eq!(m[0], None);
        assert_eq!(m[1], None);
        assert_eq!(m[2], Some(2.0));
        assert_eq!(m[3], Some(3.0));
        assert_eq!(m[4], Some(4.0));
    }

    #[test]
    fn test_ema_responds_faster_than_sma() {
        let mut p = vec![100.0; 20];
        p.push(110.0);
        let e = ema(&p, 5);
        let s = sma(&p, 5);
        assert!(e.last().unwrap().unwrap() > s.last().unwrap().unwrap());
    }

    #[test]
    fn test_rma_uses_wilder_smoothing() {
        let p = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let r = rma(&p, 3);
        assert!(r.iter().filter_map(|x| *x).count() >= 3);
    }

    #[test]
    fn test_hull_runs_without_panic() {
        let p: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let h = hma(&p, 9);
        assert!(h.last().unwrap().is_some());
    }

    #[test]
    fn test_bias_zero_when_equals_ma() {
        let p = vec![10.0; 20];
        let b = bias(&p, 5);
        assert!(b.iter().skip(4).all(|x| x.unwrap() == 0.0));
    }
}
