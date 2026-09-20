//! 期权 Greeks —— Black-Scholes 模型
//!
//! PDF 第四部分-二十四、期权页面指标
//! 1-5 节 (Delta, Gamma, Theta, Vega, Rho) + 6-15 节 (IV, IV Rank, Skew 等)
//!
//! 本项目侧重技术分析,本模块实现完整的 Black-Scholes 模型,
//! 任何标的的欧式期权都可以计算 Greeks。

/// 标准正态分布 PDF
fn norm_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// 标准正态分布 CDF (Abramowitz & Stegun 近似)
fn norm_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989422804014327;
    let mut p = d * (-x * x / 2.0).exp() *
        (t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 +
            t * (-1.821255978 + t * 1.330274429)))));
    if x > 0.0 { 1.0 - p } else { p }
}

/// Black-Scholes 期权定价 (欧式)
pub fn bs_price(spot: f64, strike: f64, time: f64, rate: f64,
              volatility: f64, is_call: bool) -> f64 {
    if time <= 0.0 || volatility <= 0.0 {
        return if is_call { (spot - strike).max(0.0) } else { (strike - spot).max(0.0) };
    }
    let d1 = ((spot / strike).ln() + (rate + volatility * volatility / 2.0) * time)
        / (volatility * time.sqrt());
    let d2 = d1 - volatility * time.sqrt();
    if is_call {
        spot * norm_cdf(d1) - strike * (-rate * time).exp() * norm_cdf(d2)
    } else {
        strike * (-rate * time).exp() * norm_cdf(-d2) - spot * norm_cdf(-d1)
    }
}

/// Delta = 期权价格对标的价格的偏导
/// Call: N(d1); Put: N(d1) - 1
pub fn delta(spot: f64, strike: f64, time: f64, rate: f64,
            volatility: f64, is_call: bool) -> f64 {
    if time <= 0.0 || volatility <= 0.0 { return 0.0; }
    let d1 = ((spot / strike).ln() + (rate + volatility * volatility / 2.0) * time)
        / (volatility * time.sqrt());
    if is_call { norm_cdf(d1) } else { norm_cdf(d1) - 1.0 }
}

/// Gamma = Delta 对标的价格的偏导
/// 买入跨式 = 最大;平值附近最大
pub fn gamma(spot: f64, _strike: f64, time: f64, _rate: f64, volatility: f64) -> f64 {
    if time <= 0.0 || volatility <= 0.0 { return 0.0; }
    let d1 = ((spot / _strike).ln() + (_rate + volatility * volatility / 2.0) * time)
        / (volatility * time.sqrt());
    norm_pdf(d1) / (spot * volatility * time.sqrt())
}

/// Theta = 期权价格对时间的偏导 (每天)
/// Call: -(S × N'(d1) × σ) / (2 × √t) - r × K × e^(-rt) × N(d2)
/// Put: 同 + r × K × e^(-rt) × N(-d2)
pub fn theta(spot: f64, strike: f64, time: f64, rate: f64,
            volatility: f64, is_call: bool) -> f64 {
    if time <= 0.0 || volatility <= 0.0 { return 0.0; }
    let d1 = ((spot / strike).ln() + (rate + volatility * volatility / 2.0) * time)
        / (volatility * time.sqrt());
    let d2 = d1 - volatility * time.sqrt();
    let common = -(spot * norm_pdf(d1) * volatility) / (2.0 * time.sqrt());
    if is_call {
        common - rate * strike * (-rate * time).exp() * norm_cdf(d2)
    } else {
        common + rate * strike * (-rate * time).exp() * norm_cdf(-d2)
    }
}

/// Vega = 期权价格对 IV 的偏导 (每 1% 变化)
pub fn vega(spot: f64, strike: f64, time: f64, _rate: f64, volatility: f64) -> f64 {
    if time <= 0.0 { return 0.0; }
    let d1 = ((spot / strike).ln() + (_rate + volatility * volatility / 2.0) * time)
        / (volatility * time.sqrt());
    spot * norm_pdf(d1) * time.sqrt() / 100.0  // 转换为每 1% 变化
}

/// Rho = 期权价格对无风险利率的偏导 (每 1% 变化)
pub fn rho(spot: f64, strike: f64, time: f64, rate: f64,
          volatility: f64, is_call: bool) -> f64 {
    if time <= 0.0 { return 0.0; }
    let d1 = ((spot / strike).ln() + (rate + volatility * volatility / 2.0) * time)
        / (volatility * time.sqrt());
    let d2 = d1 - volatility * time.sqrt();
    if is_call {
        strike * time * (-rate * time).exp() * norm_cdf(d2) / 100.0
    } else {
        -strike * time * (-rate * time).exp() * norm_cdf(-d2) / 100.0
    }
}

/// IV Rank = (IV - 52周最低IV) / (52周最高IV - 最低IV) × 100
/// PDF 第二十四章 6 节
pub fn iv_rank(current_iv: f64, min_iv: f64, max_iv: f64) -> f64 {
    if max_iv <= min_iv { return 0.0; }
    (current_iv - min_iv) / (max_iv - min_iv) * 100.0
}

/// IV Percentile = 低于当前 IV 的天数 / 总天数 × 100
/// PDF 第二十四章 7 节
pub fn iv_percentile(days_below: f64, total_days: f64) -> f64 {
    if total_days == 0.0 { return 0.0; }
    days_below / total_days * 100.0
}

/// Put/Call Ratio = Put 成交量 / Call 成交量
/// PDF 第二十四章 18 节
/// > 1.2 极度悲观(反向看多), < 0.7 极度乐观(反向看空)
pub fn put_call_ratio(put_volume: f64, call_volume: f64) -> f64 {
    if call_volume == 0.0 { return 0.0; }
    put_volume / call_volume
}
