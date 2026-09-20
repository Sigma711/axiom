//! 动量类指标 —— 衡量涨跌的速度和力度,识别超买超卖。
//!
//! PDF 章节:第三部分-十六、动量指标

use crate::indicators::ma::{ema, sma};
use crate::types::Bar;

/// RSI —— Relative Strength Index (Wilder 版)
/// 用途:超买超卖(>70 超买, <30 超卖)
/// 注意:RSI 超买不等于卖出信号,见 PDF 误区章节
pub fn rsi(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = prices.len();
    if period == 0 || n < period + 1 {
        return vec![None; n];
    }
    let mut gains = 0.0;
    let mut losses = 0.0;
    for i in 1..=period {
        let diff = prices[i] - prices[i - 1];
        if diff > 0.0 {
            gains += diff;
        } else {
            losses -= diff;
        }
    }
    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    let mut out = vec![None; n];
    out[period] = Some(if avg_loss == 0.0 {
        100.0
    } else {
        100.0 - (100.0 / (1.0 + avg_gain / avg_loss))
    });
    for i in period + 1..n {
        let diff = prices[i] - prices[i - 1];
        let g = if diff > 0.0 { diff } else { 0.0 };
        let l = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (avg_gain * (period - 1) as f64 + g) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + l) / period as f64;
        out[i] = Some(if avg_loss == 0.0 {
            100.0
        } else {
            100.0 - (100.0 / (1.0 + avg_gain / avg_loss))
        });
    }
    out
}

/// Stochastic 随机指标
/// %K = (C - LowN) / (HighN - LowN) × 100
/// %D = SMA(%K, smoothK)
pub struct StochasticOutput {
    pub k: Vec<Option<f64>>,
    pub d: Vec<Option<f64>>,
}

pub fn stochastic(
    bars: &[Bar],
    k_period: usize,
    d_period: usize,
    smooth_k: usize,
) -> StochasticOutput {
    let n = bars.len();
    let mut raw_k = vec![None; n];
    for i in k_period - 1..n {
        let window = &bars[i + 1 - k_period..=i];
        let hi = window
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let lo = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        let range = hi - lo;
        raw_k[i] = Some(if range == 0.0 {
            50.0
        } else {
            (bars[i].close - lo) / range * 100.0
        });
    }
    // 平滑 K (SMA of raw K)
    let k = crate::indicators::ma::optional_smooth(&raw_k, smooth_k, sma);
    let d = crate::indicators::ma::optional_smooth(&k, d_period, sma);
    StochasticOutput { k, d }
}

/// KDJ —— 中国市场流行的随机指标变种
/// RSV = (C - LowN) / (HighN - LowN) × 100
/// K = SMA(RSV, M1) × 1 (相当于 2/3 上期 K + 1/3 RSV)
/// D = SMA(K, M2)
/// J = 3K - 2D
pub struct KdjOutput {
    pub k: Vec<Option<f64>>,
    pub d: Vec<Option<f64>>,
    pub j: Vec<Option<f64>>,
}

pub fn kdj(bars: &[Bar], n: usize, m1: usize, m2: usize) -> KdjOutput {
    let len = bars.len();
    let mut rsv = vec![None; len];
    for i in n - 1..len {
        let window = &bars[i + 1 - n..=i];
        let hi = window
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let lo = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        let range = hi - lo;
        rsv[i] = Some(if range == 0.0 {
            50.0
        } else {
            (bars[i].close - lo) / range * 100.0
        });
    }
    // K, D 使用 Wilder 平滑
    let mut k_vals: Vec<f64> = vec![50.0; len];
    let mut d_vals: Vec<f64> = vec![50.0; len];
    let mut j_vals: Vec<f64> = vec![50.0; len];
    let alpha_k = 1.0 / m1 as f64;
    let alpha_d = 1.0 / m2 as f64;
    for i in 1..len {
        if let Some(r) = rsv[i] {
            k_vals[i] = alpha_k * r + (1.0 - alpha_k) * k_vals[i - 1];
            d_vals[i] = alpha_d * k_vals[i] + (1.0 - alpha_d) * d_vals[i - 1];
            j_vals[i] = 3.0 * k_vals[i] - 2.0 * d_vals[i];
        } else {
            k_vals[i] = k_vals[i - 1];
            d_vals[i] = d_vals[i - 1];
            j_vals[i] = j_vals[i - 1];
        }
    }
    let k_out: Vec<Option<f64>> = k_vals
        .iter()
        .enumerate()
        .map(|(i, v)| if rsv[i].is_some() { Some(*v) } else { None })
        .collect();
    let d_out: Vec<Option<f64>> = d_vals
        .iter()
        .enumerate()
        .map(|(i, v)| if rsv[i].is_some() { Some(*v) } else { None })
        .collect();
    let j_out: Vec<Option<f64>> = j_vals
        .iter()
        .enumerate()
        .map(|(i, v)| if rsv[i].is_some() { Some(*v) } else { None })
        .collect();
    KdjOutput {
        k: k_out,
        d: d_out,
        j: j_out,
    }
}

/// CCI —— Commodity Channel Index
/// 典型价 TP = (H+L+C) / 3
/// CCI = (TP - SMA(TP, N)) / (0.015 × MeanDeviation)
pub fn cci(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let tps: Vec<f64> = bars.iter().map(|b| b.typical_price()).collect();
    let ma_tp = sma(&tps, period);
    let mut out = vec![None; n];
    for i in period - 1..n {
        let window = &tps[i + 1 - period..=i];
        let mean = ma_tp[i].unwrap();
        let mean_dev: f64 = window.iter().map(|x| (x - mean).abs()).sum::<f64>() / period as f64;
        if mean_dev > 0.0 {
            out[i] = Some((tps[i] - mean) / (0.015 * mean_dev));
        }
    }
    out
}

/// Williams %R
/// %R = (HighN - Close) / (HighN - LowN) × -100
pub fn williams_r(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period - 1..n {
        let window = &bars[i + 1 - period..=i];
        let hi = window
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let lo = window.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        let range = hi - lo;
        if range > 0.0 {
            out[i] = Some((hi - bars[i].close) / range * -100.0);
        }
    }
    out
}

/// ROC —— Rate of Change
/// ROC = (Close - Close[N]) / Close[N] × 100
pub fn roc(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    for (i, value) in out.iter_mut().enumerate().skip(period) {
        let prev = prices[i - period];
        if prev != 0.0 {
            *value = Some((prices[i] - prev) / prev * 100.0);
        }
    }
    out
}

/// Momentum
/// MOM = Close - Close[N]
pub fn momentum(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    for (i, value) in out.iter_mut().enumerate().skip(period) {
        *value = Some(prices[i] - prices[i - period]);
    }
    out
}

/// CMO —— Chande Momentum Oscillator
pub fn cmo(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = prices.len();
    if n < period + 1 {
        return vec![None; n];
    }
    let mut out = vec![None; n];
    for (i, value) in out.iter_mut().enumerate().skip(period) {
        let mut up_sum = 0.0;
        let mut down_sum = 0.0;
        for j in (i + 1 - period)..=i {
            let diff = prices[j] - prices[j - 1];
            if diff > 0.0 {
                up_sum += diff;
            } else {
                down_sum -= diff;
            }
        }
        let total = up_sum + down_sum;
        if total > 0.0 {
            *value = Some((up_sum - down_sum) / total * 100.0);
        }
    }
    out
}

/// TSI —— True Strength Index
/// 双重 EMA 平滑价格变化
pub fn tsi(prices: &[f64], long_period: usize, short_period: usize) -> Vec<Option<f64>> {
    let diffs: Vec<_> = prices.windows(2).map(|w| w[1] - w[0]).collect();
    let abs: Vec<_> = diffs.iter().map(|x| x.abs()).collect();
    let a = ema(&diffs, long_period);
    let b = ema(&abs, long_period);
    let valid_a: Vec<_> = a.iter().filter_map(|x| *x).collect();
    let valid_b: Vec<_> = b.iter().filter_map(|x| *x).collect();
    let aa = ema(&valid_a, short_period);
    let bb = ema(&valid_b, short_period);
    let mut out = vec![None; prices.len()];
    for (i, (a, b)) in aa.iter().zip(bb.iter()).enumerate() {
        if let (Some(a), Some(b)) = (a, b) {
            if *b != 0.0 && i + long_period < out.len() {
                out[i + long_period] = Some(100.0 * a / b);
            }
        }
    }
    out
}

/// Stochastic RSI
pub struct StochRsiOutput {
    pub k: Vec<Option<f64>>,
    pub d: Vec<Option<f64>>,
}

pub fn stochastic_rsi(
    prices: &[f64],
    rsi_period: usize,
    stoch_period: usize,
    k_smooth: usize,
    d_smooth: usize,
) -> StochRsiOutput {
    let rsi_vals = rsi(prices, rsi_period);
    let n = prices.len();
    let mut raw_k = vec![None; n];
    for i in rsi_period..n {
        if rsi_vals[i].is_none() {
            continue;
        }
        if i + 1 < stoch_period {
            continue;
        }
        let start = i + 1 - stoch_period;
        if start < rsi_period {
            continue;
        }
        let window: Vec<f64> = (start..=i).filter_map(|j| rsi_vals[j]).collect();
        if window.len() < stoch_period {
            continue;
        }
        let hi = window.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b));
        let lo = window.iter().fold(f64::INFINITY, |a, b| a.min(*b));
        let range = hi - lo;
        let cur = rsi_vals[i].unwrap();
        raw_k[i] = Some(if range == 0.0 {
            50.0
        } else {
            (cur - lo) / range * 100.0
        });
    }
    let k = crate::indicators::ma::optional_smooth(&raw_k, k_smooth, sma);
    let d = crate::indicators::ma::optional_smooth(&k, d_smooth, sma);
    StochRsiOutput { k, d }
}

/// Ultimate Oscillator —— 综合三个周期的动量
pub fn ultimate_oscillator(bars: &[Bar], p1: usize, p2: usize, p3: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut bp = vec![0.0; n]; // buying pressure
    let mut tr = vec![0.0; n]; // true range
    for i in 1..n {
        let close = bars[i].close;
        let low = bars[i].low;
        let prev_close = bars[i - 1].close;
        bp[i] = close - low.min(prev_close);
        tr[i] = (bars[i].high - low)
            .max((bars[i].high - prev_close).abs())
            .max((low - prev_close).abs());
    }
    let avg = |period: usize, idx: usize| -> Option<f64> {
        if idx < period {
            return None;
        }
        let bp_sum: f64 = (idx + 1 - period..=idx).map(|i| bp[i]).sum();
        let tr_sum: f64 = (idx + 1 - period..=idx).map(|i| tr[i]).sum();
        if tr_sum == 0.0 {
            None
        } else {
            Some(100.0 * bp_sum / tr_sum)
        }
    };
    let mut out = vec![None; n];
    for (i, value) in out.iter_mut().enumerate().skip(3) {
        let a1 = avg(p1, i);
        let a2 = avg(p2, i);
        let a3 = avg(p3, i);
        if let (Some(x1), Some(x2), Some(x3)) = (a1, a2, a3) {
            *value = Some(
                100.0 * (p1 as f64 * x1 + p2 as f64 * x2 + p3 as f64 * x3)
                    / (p1 as f64 + p2 as f64 + p3 as f64),
            );
        }
    }
    out
}

/// Awesome Oscillator —— (SMA5_HL2 - SMA34_HL2) 的差
pub fn awesome_oscillator(bars: &[Bar], fast: usize, slow: usize) -> Vec<Option<f64>> {
    let hl2: Vec<f64> = bars.iter().map(|b| (b.high + b.low) / 2.0).collect();
    let sma_fast = sma(&hl2, fast);
    let sma_slow = sma(&hl2, slow);
    sma_fast
        .iter()
        .zip(sma_slow.iter())
        .map(|(f, s)| match (f, s) {
            (Some(x), Some(y)) => Some(x - y),
            _ => None,
        })
        .collect()
}

/// DPO —— Detrended Price Oscillator
/// DPO = Close - SMA(Close, N) shifted N/2 + 1 bars ago
pub fn dpo(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let ma = sma(prices, period);
    let shift = period / 2 + 1;
    let mut out = vec![None; prices.len()];
    for i in shift..prices.len() {
        out[i] = ma[i].map(|m| prices[i - shift] - m);
    }
    out
}

/// Elder Ray: 牛力 (Bull Power) = High - EMA(N); 熊力 (Bear Power) = Low - EMA(N)
/// 用于识别价格相对均线的多空力量
pub fn elder_ray(bars: &[Bar], period: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let ema_vals = ema(&closes, period);
    let mut bull = Vec::with_capacity(bars.len());
    let mut bear = Vec::with_capacity(bars.len());
    for (i, bar) in bars.iter().enumerate() {
        match ema_vals[i] {
            Some(e) => {
                bull.push(Some(bar.high - e));
                bear.push(Some(bar.low - e));
            }
            None => {
                bull.push(None);
                bear.push(None);
            }
        }
    }
    (bull, bear)
}

/// Balance of Power (BOP) = (Close - Open) / (High - Low)
/// 衡量多空力量平衡: 1 = 纯多头, -1 = 纯空头, 0 = 平衡
pub fn bop(bars: &[Bar]) -> Vec<Option<f64>> {
    bars.iter()
        .map(|b| {
            let range = b.high - b.low;
            if range == 0.0 {
                Some(0.0)
            } else {
                Some((b.close - b.open) / range)
            }
        })
        .collect()
}

/// Fisher Transform: 把价格正态化到接近正态分布
/// Y = 0.5 * ln((1+X) / (1-X)), X 限制在 [-0.999, 0.999]
pub fn fisher_transform(prices: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }
    for i in period - 1..n {
        let window = &prices[i + 1 - period..=i];
        let min_p = window.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_p = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_p - min_p;
        if range == 0.0 {
            continue;
        }
        let x = ((prices[i] - min_p) / range) * 2.0 - 1.0;
        let x = x.clamp(-0.999, 0.999);
        out[i] = Some(0.5 * ((1.0 + x) / (1.0 - x)).ln());
    }
    out
}

/// Relative Vigor Index (RVI): 用收盘-开盘对比衡量能量
/// RVI = SMA(close - open, n) / SMA(high - low, n)
pub fn rvi(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    for i in period - 1..n {
        let w = &bars[i + 1 - period..=i];
        let num: f64 = w.iter().map(|b| b.close - b.open).sum::<f64>() / period as f64;
        let den: f64 = w.iter().map(|b| b.high - b.low).sum::<f64>() / period as f64;
        if den != 0.0 {
            out[i] = Some(num / den);
        }
    }
    out
}

/// DeMarker: 比较当前周期最高/最低与前一周期, 识别超买超卖
/// DeMax = max(0, high[i] - high[i-1]), DeMin = max(0, low[i-1] - low[i])
/// DMark = SMA(DeMax, n) / (SMA(DeMax, n) + SMA(DeMin, n))
pub fn demarker(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n < period + 1 {
        return out;
    }
    for (i, value) in out.iter_mut().enumerate().skip(period) {
        let mut de_max = 0.0;
        let mut de_min = 0.0;
        for j in (i + 1 - period)..=i {
            let prev = bars[j - 1];
            let cur = bars[j];
            de_max += (cur.high - prev.high).max(0.0);
            de_min += (prev.low - cur.low).max(0.0);
        }
        let denom = de_max + de_min;
        if denom > 0.0 {
            *value = Some(de_max / denom);
        } else {
            *value = Some(0.5);
        }
    }
    out
}

/// Know Sure Thing (KST): 4 个不同周期 ROC 的加权移动平均
/// KST = SMA(ROC(10) * 1, 10) + SMA(ROC(15) * 2, 10) + SMA(ROC(20) * 3, 10) + SMA(ROC(30) * 4, 15)
pub fn kst(prices: &[f64]) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    if n < 50 {
        return out;
    }
    let r10 = roc(prices, 10);
    let r15 = roc(prices, 15);
    let r20 = roc(prices, 20);
    let r30 = roc(prices, 30);
    let s1 = sma_of_opts(&r10, 10);
    let s2 = sma_of_opts(&r15, 10);
    let s3 = sma_of_opts(&r20, 10);
    let s4 = sma_of_opts(&r30, 15);
    for i in 0..n {
        if let (Some(a), Some(b), Some(c), Some(d)) = (s1[i], s2[i], s3[i], s4[i]) {
            out[i] = Some(a + 2.0 * b + 3.0 * c + 4.0 * d);
        }
    }
    out
}

fn sma_of_opts(vals: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    let mut sum = 0.0;
    let mut count = 0;
    for i in 0..n {
        if let Some(v) = vals[i] {
            sum += v;
            count += 1;
        }
        if i >= period {
            if let Some(v) = vals[i - period] {
                sum -= v;
                count -= 1;
            }
        }
        if count == period {
            out[i] = Some(sum / period as f64);
        }
    }
    out
}

/// Coppock Curve: (WMA(ROC(14), 10) + WMA(ROC(11), 10)) — 长周期买卖指标
/// WMA(ROC, 10) = 加权移动平均, 权重 1..10
pub fn coppock(prices: &[f64]) -> Vec<Option<f64>> {
    let n = prices.len();
    let mut out = vec![None; n];
    if n < 25 {
        return out;
    }
    let r14 = roc(prices, 14);
    let r11 = roc(prices, 11);
    for (i, value) in out.iter_mut().enumerate().skip(24) {
        let w14 = wma_of_opts(&r14, i, 10);
        let w11 = wma_of_opts(&r11, i, 10);
        if let (Some(a), Some(b)) = (w14, w11) {
            *value = Some(a + b);
        }
    }
    out
}

fn wma_of_opts(vals: &[Option<f64>], end: usize, period: usize) -> Option<f64> {
    if end < period - 1 {
        return None;
    }
    let start = end + 1 - period;
    let mut sum = 0.0;
    let mut weight_sum = 0.0;
    for (i, idx) in (start..=end).enumerate() {
        if let Some(v) = vals[idx] {
            let w = (i + 1) as f64;
            sum += w * v;
            weight_sum += w;
        }
    }
    if weight_sum > 0.0 {
        Some(sum / weight_sum)
    } else {
        None
    }
}

/// 心理线 PSY: N 日内上涨天数占比
/// PSY = (count(close > prev_close) over N) / N * 100
/// > 75 超买, < 25 超卖
pub fn psy(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = closes.len();
    let mut out = vec![None; n];
    if n < period + 1 {
        return out;
    }
    for (i, value) in out.iter_mut().enumerate().skip(period) {
        let window = &closes[i - period..=i];
        let up_count = window.windows(2).filter(|w| w[1] > w[0]).count();
        *value = Some(up_count as f64 / period as f64 * 100.0);
    }
    out
}

/// AR 指标: N 日内 sum(high - open) / sum(open - low) × 100
pub fn ar(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    for i in period - 1..n {
        let window = &bars[i + 1 - period..=i];
        let up_sum: f64 = window.iter().map(|b| b.high - b.open).sum();
        let down_sum: f64 = window.iter().map(|b| b.open - b.low).sum();
        if down_sum > 0.0 {
            out[i] = Some(up_sum / down_sum * 100.0);
        }
    }
    out
}

/// BR 指标: N 日内 (high - prev_close) 与 (prev_close - low) 的累加
pub fn br(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period + 1 {
        return out;
    }
    for (i, value) in out.iter_mut().enumerate().skip(period) {
        let mut up_sum = 0.0;
        let mut dn_sum = 0.0;
        for j in (i + 1 - period)..=i {
            let prev_close = bars[j - 1].close;
            let cur = &bars[j];
            up_sum += (cur.high - prev_close).max(0.0);
            dn_sum += (prev_close - cur.low).max(0.0);
        }
        if dn_sum > 0.0 {
            *value = Some(up_sum / dn_sum * 100.0);
        } else {
            *value = Some(100.0);
        }
    }
    out
}

/// CR 指标: 中价 (H+L)/2 的带状能量
/// CR = sum((mid - prev_mid) for up days) / sum((prev_mid - mid) for down days)
pub fn cr(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period + 1 {
        return out;
    }
    for i in period..n {
        let window = &bars[i + 1 - period..=i];
        let mids: Vec<f64> = window.iter().map(|b| (b.high + b.low) / 2.0).collect();
        let up_sum: f64 = mids.windows(2).map(|w| (w[1] - w[0]).max(0.0)).sum();
        let dn_sum: f64 = mids.windows(2).map(|w| (w[0] - w[1]).max(0.0)).sum();
        if dn_sum > 0.0 {
            out[i] = Some(up_sum / dn_sum * 100.0);
        } else {
            out[i] = Some(100.0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_bars(prices: &[f64]) -> Vec<Bar> {
        let t = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        prices
            .iter()
            .enumerate()
            .map(|(i, &p)| Bar {
                timestamp: t + chrono::Duration::hours(i as i64),
                open: p,
                high: p + 1.0,
                low: p - 1.0,
                close: p,
                volume: 100.0,
            })
            .collect()
    }

    #[test]
    fn test_rsi_is_between_0_and_100() {
        let p: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
        let r = rsi(&p, 14);
        for v in r.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0);
        }
    }

    #[test]
    fn test_stochastic_basic() {
        let bars = make_bars(
            &(0..50)
                .map(|i| 100.0 + (i as f64).sin() * 5.0)
                .collect::<Vec<_>>(),
        );
        let s = stochastic(&bars, 14, 3, 3);
        assert_eq!(s.k.len(), bars.len());
    }

    #[test]
    fn test_kdj_j_can_exceed_range() {
        let bars = make_bars(&(0..50).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
        let k = kdj(&bars, 9, 3, 3);
        // J 可以超出 0-100,但 K/D 不应超出
        for v in k.k.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0);
        }
        for v in k.d.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0);
        }
    }

    #[test]
    fn test_williams_r_range() {
        let bars = make_bars(
            &(0..30)
                .map(|i| 100.0 + (i as f64).sin() * 5.0)
                .collect::<Vec<_>>(),
        );
        let r = williams_r(&bars, 14);
        for v in r.iter().flatten() {
            assert!(*v <= 0.0 && *v >= -100.0);
        }
    }

    #[test]
    fn test_cci_extreme_values() {
        let bars = make_bars(&(0..50).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
        let c = cci(&bars, 20);
        for v in c.iter().flatten() {
            assert!(v.abs() < 1000.0);
        }
    }
}
