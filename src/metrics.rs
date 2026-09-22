//! 回测结果的"成绩单"。

use crate::types::{BacktestResult, EquityPoint};
use serde_json::{json, Value};
use std::collections::HashMap;

/// 从回测结果计算全套指标,返回 JSON 友好的 HashMap。
pub fn compute_metrics(result: &BacktestResult) -> Value {
    let eq = &result.equity_curve;
    if eq.is_empty() {
        return json!({"error": "没有数据"});
    }

    // The first equity point is recorded after the first order can have
    // filled, so it may already include commission and slippage. Prefer the
    // configured starting capital when it is available.
    let initial = result
        .config
        .get("initial_capital")
        .and_then(|value| value.as_f64())
        .filter(|value| *value > 0.0)
        .unwrap_or_else(|| eq[0].cash + eq[0].position_value);
    let final_equity = eq.last().unwrap().equity;
    let total_return = if initial > 0.0 {
        final_equity / initial - 1.0
    } else {
        0.0
    };

    let days = (eq.last().unwrap().timestamp - eq[0].timestamp).num_seconds() as f64 / 86_400.0;
    let annualized = if initial > 0.0 && days > 0.0 && final_equity >= 0.0 {
        (final_equity / initial).powf(365.0 / days) - 1.0
    } else {
        f64::NAN
    };

    let (max_dd, max_dd_pct) = max_drawdown(eq, initial);
    let periods_per_year = if eq.len() > 1 && days > 0.0 {
        (eq.len() - 1) as f64 * 365.0 / days
    } else {
        f64::NAN
    };
    let sharpe = sharpe_ratio(eq, periods_per_year, 0.0);
    let volatility = annualized_volatility(eq, periods_per_year);

    let closed: Vec<&crate::types::Trade> =
        result.trades.iter().filter(|t| t.is_closed()).collect();
    let n_trades = closed.len();

    let (win_rate, avg_win, avg_loss, profit_factor, avg_pnl_pct) = if n_trades > 0 {
        let wins: Vec<f64> = closed
            .iter()
            .map(|t| t.pnl())
            .filter(|p| *p > 0.0)
            .collect();
        let losses: Vec<f64> = closed
            .iter()
            .map(|t| t.pnl())
            .filter(|p| *p <= 0.0)
            .collect();
        let win_rate = if n_trades > 0 {
            wins.len() as f64 / n_trades as f64
        } else {
            0.0
        };
        let avg_win = if !wins.is_empty() {
            wins.iter().sum::<f64>() / wins.len() as f64
        } else {
            0.0
        };
        let avg_loss = if !losses.is_empty() {
            losses.iter().sum::<f64>() / losses.len() as f64
        } else {
            0.0
        };
        let profit_factor = if losses.is_empty() {
            if wins.is_empty() {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            let loss_sum = losses.iter().sum::<f64>();
            if loss_sum == 0.0 {
                f64::INFINITY
            } else {
                wins.iter().sum::<f64>() / loss_sum.abs()
            }
        };
        let avg_pnl_pct = closed.iter().map(|t| t.pnl_pct()).sum::<f64>() / n_trades as f64;
        (win_rate, avg_win, avg_loss, profit_factor, avg_pnl_pct)
    } else {
        (f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN)
    };

    // 进阶指标
    let rets = returns(eq);
    let sortino = sortino_ratio(&rets, periods_per_year);
    let calmar = calmar_ratio(annualized, max_dd_pct);
    let (var_95, cvar_95) = var_cvar(&rets, 0.95);
    let skew = crate::indicators::statistics::skewness(&rets).unwrap_or(f64::NAN);
    let kurt = crate::indicators::statistics::kurtosis(&rets).unwrap_or(f64::NAN);

    let mut metrics = json!({
        "初始资金": initial,
        "最终净值": final_equity,
        "总收益率": total_return,
        "年化收益率": annualized,
        "最大回撤_绝对": max_dd,
        "最大回撤_pct": max_dd_pct,
        "夏普比率": sharpe,
        "索提诺比率": sortino,
        "Calmar比率": calmar,
        "年化波动率": volatility,
        "VaR_95": var_95,
        "CVaR_95": cvar_95,
        "偏度": skew,
        "峰度": kurt,
        "交易笔数": n_trades,
        "胜率": win_rate,
        "平均盈利": avg_win,
        "平均亏损": avg_loss,
        "盈亏比": profit_factor,
        "平均收益率": avg_pnl_pct,
        "总手续费": result.fills.iter().map(|f| f.commission).sum::<f64>(),
    });
    let explanations: serde_json::Map<String, Value> = metrics
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, v)| v.is_null())
        .map(|(k, _)| {
            (
                k.clone(),
                json!("样本不足、没有已完成交易、零分母或年化超出有限范围；无定义不等于零。"),
            )
        })
        .collect();
    metrics["指标说明"] = json!(explanations);
    metrics
}

/// 简单格式化显示(供 GUI / CLI 用)
pub fn metrics_summary(metrics: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(obj) = metrics.as_object() {
        for (k, v) in obj {
            let s = match v {
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        if k.contains("比率") || k == "盈亏比" {
                            format!("{:.3}", f)
                        } else if k.contains("收益率")
                            || k.ends_with("_pct")
                            || k == "胜率"
                            || k == "年化波动率"
                        {
                            format!("{:.2}%", f * 100.0)
                        } else {
                            format!("{:.2}", f)
                        }
                    } else {
                        v.to_string()
                    }
                }
                Value::Null => "—".into(),
                _ => v.to_string(),
            };
            out.insert(k.clone(), s);
        }
    }
    out
}

fn max_drawdown(eq: &[EquityPoint], initial: f64) -> (f64, f64) {
    let mut peak = initial;
    let mut max_dd = 0.0;
    let mut max_dd_pct = 0.0;
    for p in eq {
        if p.equity > peak {
            peak = p.equity;
        }
        if peak > 0.0 {
            let dd = peak - p.equity;
            let dd_pct = dd / peak;
            if dd > max_dd {
                max_dd = dd;
            }
            if dd_pct > max_dd_pct {
                max_dd_pct = dd_pct;
            }
        }
    }
    (max_dd, max_dd_pct)
}

fn returns(eq: &[EquityPoint]) -> Vec<f64> {
    let mut out = Vec::with_capacity(eq.len());
    for i in 1..eq.len() {
        let prev = eq[i - 1].equity;
        if prev > 0.0 {
            out.push(eq[i].equity / prev - 1.0);
        }
    }
    out
}

fn annualized_volatility(eq: &[EquityPoint], periods_per_year: f64) -> f64 {
    let rets = returns(eq);
    if rets.len() < 2 || !periods_per_year.is_finite() || periods_per_year <= 0.0 {
        return f64::NAN;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (rets.len() - 1) as f64;
    var.sqrt() * periods_per_year.sqrt()
}

fn sharpe_ratio(eq: &[EquityPoint], periods_per_year: f64, risk_free: f64) -> f64 {
    let rets = returns(eq);
    if rets.len() < 2 || !periods_per_year.is_finite() || periods_per_year <= 0.0 {
        return f64::NAN;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (rets.len() - 1) as f64;
    let std = var.sqrt();
    if std == 0.0 {
        return f64::NAN;
    }
    let rf_per_period = risk_free / periods_per_year;
    (mean - rf_per_period) / std * periods_per_year.sqrt()
}

/// Sortino Ratio —— 只用下行波动率的分母
pub fn sortino_ratio(rets: &[f64], periods_per_year: f64) -> f64 {
    if rets.len() < 2 || !periods_per_year.is_finite() || periods_per_year <= 0.0 {
        return f64::NAN;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let downside: Vec<f64> = rets.iter().filter(|&&r| r < 0.0).copied().collect();
    if downside.is_empty() {
        return f64::INFINITY;
    }
    let down_var = downside.iter().map(|r| r.powi(2)).sum::<f64>() / rets.len() as f64;
    let down_std = down_var.sqrt();
    if down_std == 0.0 {
        return 0.0;
    }
    mean / down_std * periods_per_year.sqrt()
}

/// Calmar Ratio = CAGR / MaxDD
pub fn calmar_ratio(cagr: f64, max_dd_pct: f64) -> f64 {
    if max_dd_pct == 0.0 {
        return f64::NAN;
    }
    cagr / max_dd_pct
}

/// VaR (Value at Risk) 和 CVaR (Conditional VaR / Expected Shortfall)
/// rets 是收益率序列,confidence 是置信度(如 0.95)
/// 返回以损失为正方向的收益率分位；全盈利样本的历史损失可以为负。
pub fn var_cvar(rets: &[f64], confidence: f64) -> (f64, f64) {
    if rets.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    if !(0.0 < confidence && confidence < 1.0) || rets.iter().any(|r| !r.is_finite()) {
        return (f64::NAN, f64::NAN);
    }
    let mut sorted: Vec<f64> = rets.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Empirical nearest-rank loss quantile, including all ties in the tail.
    let idx = sorted.len() - (confidence * sorted.len() as f64).ceil() as usize;
    let var = -sorted[idx];
    let tail: Vec<f64> = sorted
        .iter()
        .filter(|r| **r <= sorted[idx])
        .copied()
        .collect();
    let cvar = if !tail.is_empty() {
        -tail.iter().sum::<f64>() / tail.len() as f64
    } else {
        var
    };
    (var, cvar)
}
