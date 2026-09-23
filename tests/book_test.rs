use axiom::{book, types::Bar};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

fn closed_bars(closes: &[f64]) -> Vec<Bar> {
    let start = DateTime::from_timestamp(0, 0).unwrap();
    closes
        .iter()
        .enumerate()
        .map(|(i, &close)| Bar {
            timestamp: start + Duration::hours(i as i64),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 100.0,
        })
        .collect()
}

#[test]
fn industry_metrics_use_explicit_chinese_labeled_inputs_and_reject_invalid_values() {
    let concept = book::catalog()
        .into_iter()
        .find(|x| x.id == "book_bank_nim")
        .expect("bank NIM practice");
    assert_eq!(concept.inputs[0].label, "净利息收入（元）");
    let r = book::evaluate(
        "book_bank_nim",
        &[],
        &json!({"net_interest_income":2.0,"average_earning_assets":100.0}),
    )
    .unwrap();
    assert_eq!(r["values"]["book_bank_nim"].as_f64(), Some(0.02));
    assert!(book::evaluate("book_bank_nim", &[], &json!({"unknown":1.0})).is_err());
    assert!(book::evaluate("book_bank_nim", &[], &json!({"average_earning_assets":0.0})).is_err());
}

#[test]
fn every_industry_metric_has_a_worked_literal_answer() {
    let cases: &[(&str, f64)] = &[
        ("book_bank_nim", 0.02),
        ("book_bank_npl_ratio", 0.01),
        ("book_bank_provision_coverage", 3.0),
        ("book_bank_cet1_ratio", 0.12),
        ("book_bank_cost_income", 0.4),
        ("book_insurance_combined_ratio", 0.9),
        ("book_insurance_solvency_ratio", 1.8),
        ("book_insurance_nbv", 15.0),
        ("book_reit_ffo", 65.0),
        ("book_reit_affo", 50.0),
        ("book_reit_occupancy", 0.9),
        ("book_reit_cap_rate", 0.06),
        ("book_saas_arr", 120_000_000.0),
        ("book_saas_nrr", 1.1),
        ("book_saas_churn", 0.05),
        ("book_saas_rule_of_40", 0.42),
        ("book_saas_cac_payback", 15.0),
        ("book_platform_gmv", 10_000_000_000.0),
        ("book_platform_take_rate", 0.05),
        ("book_internet_dau_mau", 0.3),
        ("book_internet_arpu", 10.0),
        ("book_retail_same_store_sales_growth", 0.1),
        ("book_semiconductor_utilization", 0.8),
        ("book_semiconductor_asp", 10.0),
        ("book_industrial_book_to_bill", 1.2),
        ("book_industrial_backlog", 3_000_000.0),
        ("book_energy_reserve_life", 10.0),
        ("book_energy_lifting_cost", 2.0),
        ("book_gold_aisc", 1200.0),
        ("book_airline_load_factor", 0.85),
        ("book_airline_rasm", 0.08),
        ("book_airline_casm", 0.07),
        ("book_telecom_arpu", 5.0),
        ("book_telecom_churn", 0.02),
        ("book_biopharma_cash_runway", 12.0),
        ("book_shipping_tce", 10.0),
    ];
    for (id, expected) in cases {
        let result = book::evaluate(id, &[], &json!({})).unwrap_or_else(|e| panic!("{id}: {e}"));
        let actual = result["values"][id].as_f64().expect("numeric result");
        assert!(
            (actual - expected).abs() < 1e-12,
            "{id}: expected {expected}, got {actual}"
        );
    }
}

#[test]
fn every_book_catalog_default_is_executable_and_all_inputs_are_described() {
    for concept in book::catalog() {
        assert!(
            !concept.name.trim().is_empty(),
            "{} has no name",
            concept.id
        );
        for input in &concept.inputs {
            assert!(
                !input.key.trim().is_empty(),
                "{} has an empty key",
                concept.id
            );
            assert!(
                !input.label.trim().is_empty(),
                "{}:{} has no label",
                concept.id,
                input.key
            );
        }
        let bars = if matches!(
            concept.id.as_str(),
            "book_log_return" | "book_nonstandard_bar"
        ) {
            closed_bars(&[100.0, 110.0])
        } else {
            Vec::new()
        };
        book::evaluate(&concept.id, &bars, &json!({}))
            .unwrap_or_else(|e| panic!("{} defaults: {e}", concept.id));
    }
}

#[test]
fn log_return_uses_the_last_two_closed_market_bars() {
    let bars = closed_bars(&[90.0, 100.0, 110.0]);
    let out = book::evaluate("book_log_return", &bars, &json!({})).unwrap();
    assert_eq!(out["input_kind"], "market_bars");
    assert_eq!(out["provenance"], "provided_market_bars");
    assert!((out["values"]["book_log_return"].as_f64().unwrap() - (1.1_f64).ln()).abs() < 1e-12);
    let concept = book::catalog()
        .into_iter()
        .find(|concept| concept.id == "book_log_return")
        .unwrap();
    assert_eq!(concept.input_kind, "market_bars");
    assert!(concept.inputs.is_empty());
}

#[test]
fn log_return_rejects_price_fixtures_short_unordered_and_future_bars() {
    let bars = closed_bars(&[100.0, 110.0]);
    assert!(book::evaluate(
        "book_log_return",
        &bars,
        &json!({"start_price": 100.0, "end_price": 110.0})
    )
    .is_err());
    assert!(book::evaluate("book_log_return", &bars[..1], &json!({})).is_err());

    let mut duplicate_time = bars.clone();
    duplicate_time[1].timestamp = duplicate_time[0].timestamp;
    assert!(book::evaluate("book_log_return", &duplicate_time, &json!({})).is_err());

    let mut future = bars;
    future[1].timestamp = Utc::now() + Duration::hours(1);
    assert!(book::evaluate("book_log_return", &future, &json!({})).is_err());
}

#[test]
fn industry_ratio_denominators_reject_zero() {
    let cases: &[(&str, &str)] = &[
        ("book_bank_nim", "average_earning_assets"),
        ("book_bank_npl_ratio", "gross_loans"),
        ("book_bank_provision_coverage", "nonperforming_loans"),
        ("book_bank_cet1_ratio", "risk_weighted_assets"),
        ("book_bank_cost_income", "operating_income"),
        ("book_insurance_combined_ratio", "net_earned_premiums"),
        ("book_insurance_solvency_ratio", "required_capital"),
        ("book_reit_occupancy", "lettable_area"),
        ("book_reit_cap_rate", "property_value"),
        ("book_saas_nrr", "beginning_cohort_revenue"),
        ("book_saas_churn", "beginning_customers"),
        ("book_saas_cac_payback", "monthly_arpu"),
        ("book_platform_take_rate", "gross_merchandise_value"),
        ("book_internet_dau_mau", "monthly_active_users"),
        ("book_internet_arpu", "active_users"),
        (
            "book_retail_same_store_sales_growth",
            "prior_same_store_sales",
        ),
        ("book_semiconductor_utilization", "maximum_capacity"),
        ("book_semiconductor_asp", "units_sold"),
        ("book_industrial_book_to_bill", "recognized_revenue"),
        ("book_energy_reserve_life", "annual_production"),
        ("book_energy_lifting_cost", "production_volume"),
        ("book_gold_aisc", "gold_ounces_produced"),
        ("book_airline_load_factor", "available_seat_kilometers"),
        ("book_airline_rasm", "available_seat_kilometers"),
        ("book_airline_casm", "available_seat_kilometers"),
        ("book_telecom_arpu", "average_subscribers"),
        ("book_telecom_churn", "beginning_subscribers"),
        ("book_biopharma_cash_runway", "monthly_cash_burn"),
        ("book_shipping_tce", "available_operating_days"),
    ];
    for (id, denominator) in cases {
        let mut inputs = serde_json::Map::new();
        inputs.insert((*denominator).into(), json!(0.0));
        assert!(
            book::evaluate(id, &[], &serde_json::Value::Object(inputs)).is_err(),
            "{id} accepts zero {denominator}"
        );
    }
}
#[test]
fn book_ratios_have_known_answers() {
    let r = book::evaluate(
        "book_current_ratio",
        &[],
        &json!({"current_assets":150.0,"current_liabilities":100.0}),
    )
    .unwrap();
    assert_eq!(r["values"]["book_current_ratio"].as_f64(), Some(1.5));
}

#[test]
fn valuation_formulae_have_worked_answers() {
    let x = book::evaluate(
        "book_ptbv",
        &[],
        &json!({"market_cap":1000.0,"equity":800.0,"goodwill":100.0,"intangibles":200.0}),
    )
    .unwrap();
    assert_eq!(x["values"]["book_ptbv"].as_f64(), Some(2.0));
    let x=book::evaluate("book_ev",&[],&json!({"market_cap":1000.0,"debt":300.0,"preferred_equity":20.0,"minority_interest":10.0,"cash":50.0})).unwrap();
    assert_eq!(x["values"]["book_ev"].as_f64(), Some(1280.0));
    let x = book::evaluate(
        "book_gross_margin",
        &[],
        &json!({"revenue":500.0,"cost_of_sales":300.0}),
    )
    .unwrap();
    assert_eq!(x["values"]["gross_margin"].as_f64(), Some(0.4));
}

#[test]
fn insurance_new_business_value_discounts_the_future_profit_cash_flows() {
    let r = book::evaluate(
        "book_insurance_nbv",
        &[],
        &json!({"year1_profit":110.0,"year2_profit":121.0,"discount_rate":0.1}),
    )
    .unwrap();
    assert!((r["values"]["book_insurance_nbv"].as_f64().unwrap() - 200.0).abs() < 1e-10);
    assert!(book::evaluate("book_insurance_nbv", &[], &json!({"discount_rate":-1.0})).is_err());
}

#[test]
fn all_foundation_lessons_have_specific_examples_and_matching_catalog_entries() {
    let lessons: serde_json::Value =
        serde_json::from_str(include_str!("../docs/book/foundation-lessons.json")).unwrap();
    let entries = book::entries();
    for (id, lesson) in lessons.as_object().unwrap() {
        let entry = entries.iter().find(|e| &e.id == id).unwrap();
        assert_eq!(entry.meaning, lesson["meaning"].as_str().unwrap());
        assert_eq!(entry.example, lesson["example"].as_str().unwrap());
        assert_eq!(entry.pitfalls, lesson["pitfalls"].as_str().unwrap());
        assert!(
            entry.example.chars().any(|c| c.is_ascii_digit()),
            "{id} needs a concrete example"
        );
    }
    for entry in entries.iter().filter(|e| e.category != "原书行业专属指标") {
        assert!(
            lessons.get(&entry.id).is_some(),
            "missing prose: {}",
            entry.id
        );
    }
}

#[test]
fn nonstandard_bar_ohlc4_is_a_synthetic_display_price_not_an_execution_price() {
    // Worked independently: (100 + 109 + 99 + 106) / 4 = 103.5.
    let synthetic = book::nonstandard_bar_ohlc4(100.0, 109.0, 99.0, 106.0).unwrap();
    assert!((synthetic - 103.5).abs() < 1e-12);
    assert_ne!(
        synthetic, 106.0,
        "OHLC4 must not be presented as the actual close"
    );
}

#[test]
fn nonstandard_bar_ohlc4_rejects_invalid_ohlc_before_computing() {
    for (open, high, low, close) in [
        (100.0, 99.0, 98.0, 101.0),   // high is below close
        (100.0, 110.0, 101.0, 105.0), // low is above open
        (0.0, 10.0, 1.0, 5.0),        // non-positive open
        (100.0, f64::NAN, 99.0, 105.0),
        (100.0, f64::INFINITY, 99.0, 105.0),
        (f64::MAX, f64::MAX, f64::MAX, f64::MAX), // would overflow the sum
    ] {
        assert!(
            book::nonstandard_bar_ohlc4(open, high, low, close).is_err(),
            "invalid OHLC was accepted: {open}, {high}, {low}, {close}"
        );
    }
}

#[test]
fn nonstandard_bar_practice_reuses_the_validated_ohlc4_calculation() {
    let mut bars = closed_bars(&[90.0, 106.0]);
    bars[1].open = 100.0;
    bars[1].high = 109.0;
    bars[1].low = 99.0;
    let result = book::evaluate("book_nonstandard_bar", &bars, &json!({})).unwrap();
    assert_eq!(result["input_kind"], "market_bars");
    assert_eq!(result["provenance"], "provided_market_bars");
    assert_eq!(
        result["values"]["book_nonstandard_bar"].as_f64(),
        Some(103.5)
    );
    assert_eq!(result["values"]["actual_close"].as_f64(), Some(106.0));
    assert_eq!(
        result["values"]["synthetic_minus_close"].as_f64(),
        Some(-2.5)
    );
    assert!(book::evaluate("book_nonstandard_bar", &[], &json!({})).is_err());
    assert!(book::evaluate("book_nonstandard_bar", &bars, &json!({"open":100.0})).is_err());
    bars[1].high = 101.0;
    assert!(book::evaluate("book_nonstandard_bar", &bars, &json!({})).is_err());
}
