//! 统计学工具 —— 回归、相关系数、Z-Score、百分位排名。

/// 简单线性回归 —— 返回 (slope, intercept)
pub fn linear_regression(y: &[f64]) -> Option<(f64, f64)> {
    let n = y.len();
    if n < 2 { return None; }
    let xs: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let x_mean = xs.iter().sum::<f64>() / n as f64;
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        let dx = xs[i] - x_mean;
        num += dx * (y[i] - y_mean);
        den += dx * dx;
    }
    if den == 0.0 { return None; }
    let slope = num / den;
    let intercept = y_mean - slope * x_mean;
    Some((slope, intercept))
}

/// 皮尔逊相关系数
pub fn correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len());
    if n < 2 { return None; }
    let x_mean = x.iter().take(n).sum::<f64>() / n as f64;
    let y_mean = y.iter().take(n).sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    for i in 0..n {
        let a = x[i] - x_mean;
        let b = y[i] - y_mean;
        num += a * b;
        dx2 += a * a;
        dy2 += b * b;
    }
    if dx2 == 0.0 || dy2 == 0.0 { return None; }
    Some(num / (dx2 * dy2).sqrt())
}

/// 滚动相关系数
pub fn rolling_correlation(x: &[f64], y: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = x.len().min(y.len());
    let mut out = vec![None; n];
    for i in period..n {
        let xw = &x[i + 1 - period..=i];
        let yw = &y[i + 1 - period..=i];
        out[i] = correlation(xw, yw);
    }
    out
}

/// Z-Score —— (x - mean) / std
pub fn zscore(series: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = series.len();
    let mut out = vec![None; n];
    for i in period - 1..n {
        let window = &series[i + 1 - period..=i];
        let mean = window.iter().sum::<f64>() / period as f64;
        let var = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
        let std = var.sqrt();
        if std > 0.0 {
            out[i] = Some((series[i] - mean) / std);
        }
    }
    out
}

/// Beta = Cov(策略, 基准) / Var(基准)
pub fn beta(strategy_returns: &[f64], benchmark_returns: &[f64]) -> Option<f64> {
    let n = strategy_returns.len().min(benchmark_returns.len());
    if n < 2 { return None; }
    let s_mean = strategy_returns.iter().take(n).sum::<f64>() / n as f64;
    let b_mean = benchmark_returns.iter().take(n).sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var = 0.0;
    for i in 0..n {
        cov += (strategy_returns[i] - s_mean) * (benchmark_returns[i] - b_mean);
        var += (benchmark_returns[i] - b_mean).powi(2);
    }
    if var == 0.0 { None } else { Some(cov / var) }
}

/// Alpha = 策略收益 - (无风险 + Beta × (基准收益 - 无风险))
pub fn alpha(strategy_returns: &[f64], benchmark_returns: &[f64],
             risk_free: f64) -> Option<f64> {
    let b = beta(strategy_returns, benchmark_returns)?;
    let n = strategy_returns.len().min(benchmark_returns.len());
    let s_mean = strategy_returns.iter().take(n).sum::<f64>() / n as f64;
    let b_mean = benchmark_returns.iter().take(n).sum::<f64>() / n as f64;
    let rf_per_period = risk_free / (365.0 * 24.0); // 假设小时 K
    Some(s_mean - (rf_per_period + b * (b_mean - rf_per_period)))
}

/// 百分位排名 —— 当前值在 N 期内的百分位
pub fn percentile_rank(series: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = series.len();
    let mut out = vec![None; n];
    for i in period - 1..n {
        let window = &series[i + 1 - period..=i];
        let cur = series[i];
        let count_below = window.iter().filter(|&&x| x < cur).count();
        out[i] = Some(count_below as f64 / (period - 1) as f64 * 100.0);
    }
    out
}

/// 偏度 Skewness
pub fn skewness(returns: &[f64]) -> Option<f64> {
    let n = returns.len();
    if n < 3 { return None; }
    let mean = returns.iter().sum::<f64>() / n as f64;
    let mut m2 = 0.0;
    let mut m3 = 0.0;
    for &r in returns {
        let d = r - mean;
        m2 += d * d;
        m3 += d * d * d;
    }
    m2 /= n as f64;
    m3 /= n as f64;
    if m2 == 0.0 { return None; }
    Some(m3 / m2.powf(1.5))
}

/// 峰度 Kurtosis (excess)
pub fn kurtosis(returns: &[f64]) -> Option<f64> {
    let n = returns.len();
    if n < 4 { return None; }
    let mean = returns.iter().sum::<f64>() / n as f64;
    let mut m2 = 0.0;
    let mut m4 = 0.0;
    for &r in returns {
        let d = r - mean;
        m2 += d * d;
        m4 += d.powi(4);
    }
    m2 /= n as f64;
    m4 /= n as f64;
    if m2 == 0.0 { return None; }
    Some(m4 / (m2 * m2) - 3.0) // excess kurtosis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((correlation(&x, &y).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_correlation_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        assert!((correlation(&x, &y).unwrap() - -1.0).abs() < 1e-9);
    }

    #[test]
    fn test_linear_regression_perfect_line() {
        let y: Vec<f64> = (0..10).map(|i| (i as f64) * 2.0 + 5.0).collect();
        let (slope, intercept) = linear_regression(&y).unwrap();
        assert!((slope - 2.0).abs() < 1e-9);
        assert!((intercept - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_zscore_zero_at_mean() {
        let s = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let z = zscore(&s, 5);
        for v in z.iter().flatten() {
            assert!(v.abs() < 1e-9);
        }
    }
}
/// Hurst 指数: H > 0.5 = 持续(趋势), H < 0.5 = 反转(均值回归), H = 0.5 = 随机游走
/// R/S 分析法: H = log(R/S) / log(n)
pub fn hurst(prices: &[f64]) -> Option<f64> {
    let n = prices.len();
    if n < 16 { return None; }
    let mean = prices.iter().sum::<f64>() / n as f64;
    let mut dev: Vec<f64> = prices.iter().map(|p| p - mean).collect();
    let mut cum_dev: Vec<f64> = Vec::with_capacity(n);
    let mut acc = 0.0;
    for &d in &dev {
        acc += d;
        cum_dev.push(acc);
    }
    let r = cum_dev.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - cum_dev.iter().cloned().fold(f64::INFINITY, f64::min);
    let s = {
        let var: f64 = dev.iter().map(|x| x.powi(2)).sum::<f64>() / n as f64;
        var.sqrt()
    };
    if s == 0.0 { return None; }
    Some(((r / s).ln()) / (n as f64).ln())
}
