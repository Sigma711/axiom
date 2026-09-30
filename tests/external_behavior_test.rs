use axiom::book_technical;
use serde_json::json;

#[test]
fn rrg_coordinates_classify_each_relative_strength_quadrant() {
    let cases = [
        (101.0, 101.0, 1.0),
        (101.0, 99.0, 2.0),
        (99.0, 99.0, 3.0),
        (99.0, 101.0, 4.0),
    ];

    for (rs_ratio, rs_momentum, expected_quadrant) in cases {
        let result = book_technical::evaluate(
            "book_rrg_coordinates",
            &[],
            &json!({"rs_ratio": rs_ratio, "rs_momentum": rs_momentum}),
        )
        .unwrap();

        assert_eq!(result["values"]["quadrant"], expected_quadrant);
    }
}

#[test]
fn option_value_and_moneyness_respect_put_direction_and_atm_tolerance() {
    let put = book_technical::evaluate(
        "book_option_value_components",
        &[],
        &json!({"spot": 90.0, "strike": 100.0, "premium": 12.0, "is_call": false}),
    )
    .unwrap();
    assert_eq!(put["values"]["intrinsic_value"], 10.0);
    assert_eq!(put["values"]["premium_minus_intrinsic"], 2.0);

    let atm = book_technical::evaluate(
        "book_option_moneyness",
        &[],
        &json!({
            "spot": 100.05,
            "strike": 100.0,
            "is_call": true,
            "atm_tolerance_fraction": 0.001
        }),
    )
    .unwrap();
    assert_eq!(atm["values"]["classification"], 0.0);

    let out_of_the_money = book_technical::evaluate(
        "book_option_moneyness",
        &[],
        &json!({
            "spot": 90.0,
            "strike": 100.0,
            "is_call": true,
            "atm_tolerance_fraction": 0.001
        }),
    )
    .unwrap();
    assert_eq!(out_of_the_money["values"]["classification"], -1.0);
}
