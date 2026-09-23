use axiom::practice;
use serde_json::json;
#[test]
fn editable_valuation_uses_company_inputs_not_market_candles() {
    let eps = practice::evaluate("eps", &[], &json!({"net_income":3000000.0,"preferred_dividends":300000.0,"weighted_average_shares":1000000.0})).unwrap();
    assert_eq!(eps["values"]["eps"], 2.7);
    let entry = axiom::knowledge::all_entries()
        .into_iter()
        .find(|entry| entry.id == "eps")
        .unwrap();
    assert!(
        entry.formula.contains("优先股"),
        "EPS lesson must disclose the preferred-dividend deduction used by its implementation"
    );
    let r = practice::evaluate("pe", &[], &json!({"price":60.0,"eps":3.0})).unwrap();
    assert_eq!(r["values"]["pe"], 20.0);
    assert_eq!(r["provenance"], "editable_teaching_inputs");
    let zero = practice::evaluate("pe", &[], &json!({"eps":0.0})).unwrap();
    assert!(zero["values"]["pe"].is_null());
    assert_eq!(zero["status"], "undefined");
}
#[test]
fn every_unique_knowledge_concept_has_editable_executable_practice() {
    use std::collections::BTreeSet;
    let expected: BTreeSet<_> = axiom::knowledge::base_entries()
        .into_iter()
        .map(|e| e.id)
        .collect();
    let cat = practice::base_catalog();
    let actual: BTreeSet<_> = cat.iter().map(|e| e.id.clone()).collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 168);
    let rolling = cat
        .iter()
        .find(|item| item.id == "rolling_correlation")
        .unwrap();
    assert_eq!(
        rolling
            .inputs
            .iter()
            .map(|input| input.key.as_str())
            .collect::<Vec<_>>(),
        vec!["period"]
    );
    let bars: Vec<_> = (0..240)
        .map(|i| axiom::types::Bar {
            timestamp: chrono::DateTime::from_timestamp(i * 3600, 0).unwrap(),
            open: 100.0 + i as f64 / 10.0,
            high: 103.0 + i as f64 / 10.0,
            low: 98.0 + i as f64 / 10.0,
            close: 101.0 + i as f64 / 10.0 + (i as f64).sin(),
            volume: 1000.0 + i as f64,
        })
        .collect();
    for item in cat
        .into_iter()
        .filter(|item| !matches!(item.id.as_str(), "inside_outside" | "cvd"))
    {
        let inputs = if item.id == "rolling_correlation" {
            json!({"series_x":[0.01,0.02,0.03],"series_y":[0.03,0.02,0.01],"period":2})
        } else {
            json!({})
        };
        let result = practice::evaluate(&item.id, &bars, &inputs)
            .unwrap_or_else(|e| panic!("{}: {}", item.id, e));
        assert!(
            result["values"].as_object().is_some_and(|v| !v.is_empty()),
            "{}",
            item.id
        );
        assert!(result["units"].is_object(), "{}", item.id);
    }
}
fn candles(n: usize) -> Vec<axiom::types::Bar> {
    (0..n)
        .map(|i| axiom::types::Bar {
            timestamp: chrono::DateTime::from_timestamp(i as i64 * 3600, 0).unwrap(),
            open: 100.0 + i as f64,
            high: 103.0 + i as f64,
            low: 98.0 + i as f64,
            close: 101.0 + i as f64,
            volume: 100.0,
        })
        .collect()
}
#[test]
fn market_practices_preserve_history_and_handle_empty_and_short_input() {
    let bars = candles(140);
    for concept in practice::base_catalog().into_iter().filter(|c| {
        c.input_kind == "market_bars"
            && !matches!(c.id.as_str(), "inside_outside" | "cvd" | "bid_ask_spread")
    }) {
        assert_eq!(
            practice::evaluate(&concept.id, &[], &json!({})).unwrap()["status"],
            "undefined"
        );
        for n in [1, 2, 5, 13, 30] {
            let _ = practice::evaluate(&concept.id, &bars[..n], &json!({})).unwrap();
        }
        let short = practice::evaluate(&concept.id, &bars[..100], &json!({})).unwrap();
        let full = practice::evaluate(&concept.id, &bars, &json!({})).unwrap();
        for (a, b) in short["series"]
            .as_array()
            .unwrap()
            .iter()
            .zip(full["series"].as_array().unwrap())
        {
            assert_eq!(
                a["values"],
                json!(b["values"].as_array().unwrap()[..100]),
                "future data changed {} / {}",
                concept.id,
                a["name"]
            );
        }
    }
}
#[test]
fn known_numerical_examples_and_inputs_are_not_placeholders() {
    let b = candles(6);
    assert_eq!(
        practice::evaluate("sma", &b, &json!({"period":3})).unwrap()["values"]["sma"],
        105.0
    );
    assert_eq!(
        practice::evaluate("donchian", &b, &json!({"period":3})).unwrap()["values"]
            ["prior_high_resistance"],
        107.0
    );
    assert_eq!(
        practice::evaluate("donchian", &b, &json!({"period":3})).unwrap()["values"]
            ["prior_low_support"],
        100.0
    );
    let full = practice::evaluate("piotroski", &[], &json!({})).unwrap();
    assert_eq!(full["values"]["piotroski"], 9.0);
    let issued = practice::evaluate("piotroski", &[], &json!({"shares_issued":true})).unwrap();
    assert_eq!(issued["values"]["piotroski"], 8.0);
    let theta = practice::evaluate("theta", &[], &json!({})).unwrap()["values"]["theta"]
        .as_f64()
        .unwrap();
    assert!((theta - (-0.0175726782)).abs() < 1e-7);
    let sortino = practice::evaluate(
        "sortino",
        &[],
        &json!({"returns":[0.1,-0.1],"periods_per_year":1.0,"risk_free_annual":0.1}),
    )
    .unwrap()["values"]["sortino"]
        .as_f64()
        .unwrap();
    assert!((sortino + std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
    assert_eq!(
        practice::evaluate("volume_ratio", &b, &json!({"period":3})).unwrap()["values"]
            ["volume_ratio"],
        1.0
    );
    let ichi = practice::evaluate(
        "ichimoku",
        &b,
        &json!({"tenkan":2,"kijun":2,"senkou_b":3,"displacement":1}),
    )
    .unwrap();
    assert_eq!(ichi["values"]["senkou_b"], 103.5);
}
#[test]
fn malformed_inputs_are_rejected_and_degenerate_math_is_undefined() {
    let b = candles(5);
    assert!(practice::evaluate("sma", &b, &json!({"period":0})).is_err());
    assert!(practice::evaluate("pitfall_rsi", &b, &json!({"overbought":"x"})).is_err());
    assert!(practice::evaluate("pe", &[], &json!({"eps":"x"})).is_err());
    assert_eq!(
        practice::evaluate("theta", &[], &json!({"volatility":0.0})).unwrap()["status"],
        "undefined"
    );
    assert_eq!(
        practice::evaluate("iv", &[], &json!({"time_years":0.0})).unwrap()["status"],
        "undefined"
    );
    assert_eq!(
        practice::evaluate("iv", &[], &json!({"option_price":200.0})).unwrap()["status"],
        "undefined"
    );
}

#[test]
fn practice_validation_rejects_bad_inputs_bars_and_unordered_history() {
    let mut bars = candles(3);
    assert!(practice::evaluate("sma", &bars, &json!([])).is_err());
    assert!(practice::evaluate("sma", &bars, &json!({"unknown": 3})).is_err());
    assert!(practice::evaluate("sma", &bars, &json!({"period": "3"})).is_err());

    bars[1].high = f64::NAN;
    assert!(practice::evaluate("sma", &bars, &json!({})).is_err());
    let mut duplicate = candles(3);
    duplicate[2].timestamp = duplicate[1].timestamp;
    assert!(practice::evaluate("sma", &duplicate, &json!({})).is_err());

    assert!(practice::evaluate("book_funding", &[], &json!([])).is_err());
    assert!(practice::evaluate("book_funding", &[], &json!({"unknown": 1})).is_err());
    assert!(practice::evaluate("book_funding", &[], &json!({"is_long": 1})).is_err());
    assert!(practice::evaluate(
        "book_advances_declines",
        &[],
        &json!({"current_prices": [101.0, 99.0], "previous_prices": [100.0]})
    )
    .is_err());
}

#[test]
fn consensus_with_empty_forecasts_reports_undefined_statistics() {
    let result = practice::evaluate(
        "consensus",
        &[],
        &json!({
            "eps_estimates": [],
            "revenue_estimates": [],
            "target_prices": [],
            "ratings": []
        }),
    )
    .unwrap();
    assert_eq!(result["status"], "computed");
    assert!(result["values"]["mean_eps"].is_null());
    assert!(result["values"]["eps_dispersion"].is_null());
}
#[test]
fn three_candle_star_requires_a_confirming_third_candle() {
    let mut b = candles(3);
    b[0].open = 110.0;
    b[0].close = 100.0;
    b[0].high = 111.0;
    b[0].low = 99.0;
    b[1].open = 99.0;
    b[1].close = 99.5;
    b[1].high = 100.0;
    b[1].low = 98.0;
    b[2].open = 100.0;
    b[2].close = 108.0;
    b[2].high = 109.0;
    b[2].low = 99.0;
    let result = practice::evaluate("k_pattern_star", &b, &json!({})).unwrap();
    assert_eq!(result["series"][0]["values"], json!([null, null, 1.0]));
    b[2].close = 103.0;
    assert_eq!(
        practice::evaluate("k_pattern_star", &b, &json!({})).unwrap()["values"]["k_pattern_star"],
        0.0
    );
}
#[test]
fn chained_indicators_do_not_treat_missing_warmup_as_zero() {
    let mut bars = candles(50);
    for b in &mut bars {
        b.open = 100.0;
        b.close = 100.0;
        b.high = 101.0;
        b.low = 99.0;
    }
    let macd = practice::evaluate("macd", &bars, &json!({})).unwrap();
    assert!(macd["series"][0]["values"][0].is_null());
    assert!(macd["series"][1]["values"][32].is_null());
    assert_eq!(macd["series"][1]["values"][33], 0.0);
    let dema = practice::evaluate("dema", &bars, &json!({"period":5})).unwrap();
    assert!(dema["series"][0]["values"][7].is_null());
    assert_eq!(dema["series"][0]["values"][8], 100.0);
    let stoch = practice::evaluate(
        "stochastic",
        &bars,
        &json!({"period":14,"smooth_k":3,"smooth_d":3}),
    )
    .unwrap();
    assert!(stoch["series"][0]["values"][14].is_null());
    assert_eq!(stoch["series"][0]["values"][15], 50.0);
    let hma = practice::evaluate("hma", &candles(10), &json!({"period":4})).unwrap();
    assert!((hma["values"]["hma"].as_f64().unwrap() - 110.0).abs() < 1e-10);
}
#[test]
fn adx_waits_for_its_own_wilder_seed() {
    let r = practice::evaluate("dmi_adx", &candles(40), &json!({"period":14})).unwrap();
    assert!(r["series"][2]["values"][26].is_null());
    assert_eq!(r["series"][2]["values"][27], 100.0);
}

#[test]
fn consensus_reports_forecast_distribution_separately_from_ratings() {
    let r=practice::evaluate("consensus",&[],&json!({"eps_estimates":[2,4,4,5,10],"revenue_estimates":[100,100,100,100,100],"target_prices":[100,110,120,130,140],"ratings":[1,1,2,3,3]})).unwrap();
    assert_eq!(r["values"]["mean_eps"], 5.0);
    assert_eq!(r["values"]["median_eps"], 4.0);
    assert_eq!(r["values"]["mean_revenue"], 100.0);
    assert_eq!(r["values"]["mean_target_price"], 120.0);
    assert_eq!(r["values"]["average_rating"], 2.0);
    assert!((r["values"]["eps_dispersion"].as_f64().unwrap() - 2.683281573).abs() < 1e-9);
}

#[test]
fn every_input_has_a_beginner_friendly_chinese_label() {
    let mut missing = Vec::new();
    for concept in axiom::practice::catalog() {
        for input in concept.inputs {
            if !input.label.chars().any(|c| ('一'..='鿿').contains(&c)) {
                missing.push(format!("{}:{}", concept.id, input.key));
            }
        }
    }
    assert!(missing.is_empty(), "missing labels: {}", missing.join(", "));
}
