use axiom::{
    metrics::{compute_metrics, metrics_summary, sortino_ratio, var_cvar},
    types::{BacktestResult, EquityPoint},
};
use chrono::{Duration, TimeZone, Utc};
use serde_json::json;

#[test]
fn sortino_uses_all_observations_in_downside_deviation() {
    // Mean = 1%, downside variance = .02² / 4, downside sigma = 1%.
    assert!((sortino_ratio(&[0.01, -0.02, 0.03, 0.02], 1.0) - 1.0).abs() < 1e-12);
}

#[test]
fn historical_cvar_includes_the_var_boundary_and_its_ties() {
    let (var, cvar) = var_cvar(&[-0.10, -0.05, -0.05, 0.01], 0.5);
    assert!((var - 0.05).abs() < 1e-12);
    assert!((cvar - 0.2 / 3.0).abs() < 1e-12);
}

#[test]
fn daily_equity_is_annualized_as_daily_observations() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let result = BacktestResult {
        config: json!({"initial_capital":100.0}),
        equity_curve: [100.0, 110.0, 99.0]
            .iter()
            .enumerate()
            .map(|(i, e)| EquityPoint {
                timestamp: start + Duration::days(i as i64),
                cash: *e,
                position_value: 0.0,
                equity: *e,
            })
            .collect(),
        trades: vec![],
        signals: vec![],
        fills: vec![],
        metrics: json!({}),
    };
    let metrics = compute_metrics(&result);
    // Sample standard deviation of +10%,-10% is sqrt(.02).
    let expected = (0.02_f64 * 365.0).sqrt();
    assert!((metrics["年化波动率"].as_f64().unwrap() - expected).abs() < 1e-10);
    assert_eq!(metrics["初始资金"], 100.0);
    assert!((metrics["最大回撤_pct"].as_f64().unwrap() - 0.10).abs() < 1e-12);
}

#[test]
fn ratios_are_not_formatted_as_percentages() {
    let text = metrics_summary(&json!({"夏普比率":1.25,"总收益率":0.125,"最大回撤_绝对":100.0}));
    assert_eq!(text["夏普比率"], "1.250");
    assert_eq!(text["总收益率"], "12.50%");
    assert_eq!(text["最大回撤_绝对"], "100.00");
}

#[test]
fn undefined_ratios_are_not_reported_as_zero_and_initial_loss_counts_as_drawdown() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let result = BacktestResult {
        config: json!({"initial_capital":100.0}),
        equity_curve: vec![EquityPoint {
            timestamp: start,
            cash: 99.0,
            position_value: 0.0,
            equity: 99.0,
        }],
        trades: vec![],
        signals: vec![],
        fills: vec![],
        metrics: json!({}),
    };
    let m = compute_metrics(&result);
    assert_eq!(m["最大回撤_绝对"], 1.0);
    assert_eq!(m["最大回撤_pct"], 0.01);
    for key in ["夏普比率", "索提诺比率", "年化收益率", "胜率", "盈亏比"] {
        assert!(m[key].is_null(), "{key}: {}", m[key]);
        assert!(m["指标说明"][key].is_string());
    }
    assert_eq!(metrics_summary(&m)["夏普比率"], "—");
}
