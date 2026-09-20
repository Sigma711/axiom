use axiom::workflows;
use serde_json::json;

#[test]
fn book_worked_example_sizes_one_hundred_shares_without_predicting_a_trade() {
    let r = workflows::evaluate("book_combined_analysis", &[], &json!({})).unwrap();
    assert_eq!(r["values"]["risk_distance"], 8.0);
    assert_eq!(r["values"]["risk_budget_shares"], 100.0);
    assert_eq!(r["values"]["pe"], 20.0);
    assert_eq!(r["values"]["pb"], 2.5);
    assert_eq!(r["values"]["trend_up"], 1.0);
    assert!((r["values"]["value_spread"].as_f64().unwrap() - 0.06).abs() < 1e-12);
    let limited =
        workflows::evaluate("book_combined_analysis", &[], &json!({"cash_budget":4500})).unwrap();
    assert_eq!(limited["values"]["affordable_shares"], 45.0);
    assert_eq!(limited["values"]["position_shares"], 45.0);
}

#[test]
fn checklist_exposes_missing_execution_and_point_in_time_evidence() {
    let r = workflows::evaluate(
        "book_reading_order",
        &[],
        &json!({"execution_checked":false}),
    )
    .unwrap();
    assert_eq!(r["values"]["completed_steps"], 9.0);
    assert_eq!(r["values"]["review_complete"], 0.0);
    let r = workflows::evaluate(
        "book_data_conventions",
        &[],
        &json!({"published_before_signal":false}),
    )
    .unwrap();
    assert_eq!(r["values"]["reproducible"], 0.0);
    assert!(workflows::evaluate("book_combined_analysis", &[], &json!({"atr":0})).is_err());
    assert!(workflows::evaluate("book_reading_order", &[], &json!({"unknown":true})).is_err());
}

#[test]
fn regimes_do_not_count_correlated_indicators_as_independent_evidence() {
    let r = workflows::evaluate(
        "book_market_regimes",
        &[],
        &json!({"trend_indicators":3,"momentum_indicators":4}),
    )
    .unwrap();
    assert_eq!(r["values"]["redundant_indicators"], 4.0);
    assert_eq!(r["values"]["evidence_groups"], 5.0);
}
