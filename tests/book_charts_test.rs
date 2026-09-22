use axiom::{book_charts as charts, types::Bar};
use chrono::DateTime;
use serde_json::json;

fn close_bars(closes: &[f64]) -> Vec<Bar> {
    closes
        .iter()
        .enumerate()
        .map(|(i, &close)| Bar {
            timestamp: DateTime::from_timestamp(i as i64 * 60, 0).unwrap(),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 100.0,
        })
        .collect()
}

#[test]
fn market_chart_concepts_use_provided_complete_ohlcv_bars() {
    let ha_source = vec![Bar {
        timestamp: DateTime::from_timestamp(0, 0).unwrap(),
        open: 10.0,
        high: 12.0,
        low: 9.0,
        close: 11.0,
        volume: 50.0,
    }];
    type ChartCase = (
        &'static str,
        Vec<Bar>,
        serde_json::Value,
        &'static str,
        f64,
        &'static str,
    );
    let cases: Vec<ChartCase> = vec![
        (
            "book_chart_heikin_ashi",
            ha_source,
            json!({}),
            "ha_close",
            10.5,
            "ohlc",
        ),
        (
            "book_chart_renko",
            close_bars(&[10.0, 12.1]),
            json!({"brick_size": 1.0}),
            "renko_close",
            12.0,
            "close",
        ),
        (
            "book_chart_point_figure",
            close_bars(&[10.0, 12.0]),
            json!({"box_size": 1.0, "reversal_boxes": 3.0}),
            "point_figure_box",
            12.0,
            "close",
        ),
        (
            "book_chart_kagi",
            close_bars(&[10.0, 12.0, 10.8]),
            json!({"reversal_size": 1.0}),
            "kagi_turn",
            10.8,
            "close",
        ),
        (
            "book_chart_three_line_break",
            close_bars(&[10.0, 9.0, 8.0, 7.0, 11.0]),
            json!({"line_count": 3.0}),
            "three_line_close",
            11.0,
            "close",
        ),
    ];

    for (id, source, input, name, expected, source_price) in cases {
        let result = charts::evaluate(id, &source, &input).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(result["input_kind"], "market_bars", "{id}");
        assert_eq!(result["provenance"], "provided_market_bars", "{id}");
        assert_eq!(result["chart"]["input"], "provided_ohlcv_bars", "{id}");
        assert_eq!(result["chart"]["source_price"], source_price, "{id}");
        assert_eq!(result["chart"]["source_bar_count"], source.len(), "{id}");
        assert_eq!(result["values"][name].as_f64(), Some(expected), "{id}");
    }
}

#[test]
fn market_chart_outputs_are_causal_and_keep_thresholds_editable() {
    let cases = [
        ("book_chart_heikin_ashi", json!({})),
        ("book_chart_renko", json!({"brick_size": 1.0})),
        (
            "book_chart_point_figure",
            json!({"box_size": 1.0, "reversal_boxes": 3.0}),
        ),
        ("book_chart_kagi", json!({"reversal_size": 1.0})),
        ("book_chart_three_line_break", json!({"line_count": 3.0})),
    ];
    let prefix = close_bars(&[10.0, 12.0, 10.0, 13.0]);
    let mut full = prefix.clone();
    full.push(Bar {
        timestamp: DateTime::from_timestamp(4 * 60, 0).unwrap(),
        open: 13.0,
        high: 100.0,
        low: 12.0,
        close: 99.0,
        volume: 200.0,
    });

    for (id, input) in cases {
        let before = charts::evaluate(id, &prefix, &input).unwrap();
        let after = charts::evaluate(id, &full, &input).unwrap();
        let before_bars = before["chart"]["bars"].as_array().unwrap();
        assert_eq!(
            before_bars,
            &after["chart"]["bars"].as_array().unwrap()[..before_bars.len()],
            "later OHLCV bars changed completed {id} history"
        );
    }

    for id in [
        "book_chart_heikin_ashi",
        "book_chart_renko",
        "book_chart_point_figure",
        "book_chart_kagi",
        "book_chart_three_line_break",
    ] {
        assert_eq!(
            charts::catalog()
                .iter()
                .find(|concept| concept.id == id)
                .unwrap()
                .input_kind,
            "market_bars"
        );
    }
}

#[test]
fn market_charts_reject_price_fixtures_and_invalid_or_missing_ohlcv() {
    let valid = close_bars(&[10.0, 12.0]);
    assert!(
        charts::evaluate("book_chart_renko", &valid, &json!({"prices": [10.0, 12.0]})).is_err()
    );
    assert!(charts::evaluate("book_chart_renko", &[], &json!({})).is_err());

    let mut invalid = valid.clone();
    invalid[1].timestamp = invalid[0].timestamp;
    assert!(charts::evaluate("book_chart_renko", &invalid, &json!({})).is_err());
    invalid[1].timestamp = DateTime::from_timestamp(60, 0).unwrap();
    invalid[1].high = invalid[1].open - 0.1;
    assert!(charts::evaluate("book_chart_renko", &invalid, &json!({})).is_err());
    let mut future = valid;
    future[1].timestamp = chrono::Utc::now() + chrono::Duration::hours(1);
    assert!(charts::evaluate("book_chart_renko", &future, &json!({})).is_err());
}

#[test]
fn market_chart_thresholds_keep_their_established_reversal_rules() {
    let kagi = charts::evaluate(
        "book_chart_kagi",
        &close_bars(&[10.0, 10.2]),
        &json!({"reversal_size": 1.0}),
    )
    .unwrap();
    assert_eq!(kagi["chart"]["bars"], json!([]));

    let kagi_switch = charts::evaluate(
        "book_chart_kagi",
        &close_bars(&[10.0, 12.0, 10.0, 13.0]),
        &json!({"reversal_size": 1.0}),
    )
    .unwrap();
    assert_eq!(kagi_switch["chart"]["bars"][2]["line_style"], "yang");
    assert_eq!(kagi_switch["chart"]["bars"][2]["switch_price"], 12.0);

    let renko = charts::evaluate(
        "book_chart_renko",
        &close_bars(&[10.0, 12.0, 10.0]),
        &json!({"brick_size": 1.0}),
    )
    .unwrap();
    assert_eq!(renko["chart"]["bars"][2]["open"], 11.0);
    assert_eq!(renko["chart"]["bars"][2]["close"], 10.0);

    let line_break = charts::evaluate(
        "book_chart_three_line_break",
        &close_bars(&[10.0, 9.0, 8.0, 7.0, 9.5]),
        &json!({"line_count": 3.0}),
    )
    .unwrap();
    assert_eq!(line_break["chart"]["bars"].as_array().unwrap().len(), 3);
    assert!(charts::evaluate(
        "book_chart_point_figure",
        &close_bars(&[10.0, 12.0]),
        &json!({"reversal_boxes": 1.5})
    )
    .is_err());
}

#[test]
fn range_bars_need_ordered_trade_prices_and_only_close_on_a_completed_range() {
    let result = charts::evaluate(
        "book_chart_range_bars",
        &[],
        &json!({"ticks":[10.0,10.4,11.0,10.7,9.5],"range_size":1.0}),
    )
    .unwrap();
    assert_eq!(result["provenance"], "editable_teaching_inputs");
    assert_eq!(result["chart"]["input"], "explicit_ordered_ticks");
    assert_eq!(result["chart"]["bars"].as_array().unwrap().len(), 2);
    assert_eq!(result["chart"]["bars"][0]["high"], 11.0);
    assert_eq!(result["chart"]["bars"][0]["low"], 10.0);
    assert_eq!(result["chart"]["bars"][1]["close"], 9.5);
    assert!(charts::evaluate("book_chart_range_bars", &[], &json!({"range_size":0})).is_err());
    assert!(charts::evaluate("book_chart_range_bars", &[], &json!({"ticks":[]})).is_err());
    assert!(charts::evaluate("unknown", &[], &json!({})).is_err());
}

#[test]
fn nonstandard_charts_bound_output_size_and_ignore_unchanged_closes() {
    let runaway = close_bars(&[2.0, 200.0]);
    assert!(
        charts::evaluate("book_chart_renko", &runaway, &json!({"brick_size":0.001}))
            .unwrap_err()
            .contains("10000")
    );
    let repeated = charts::evaluate(
        "book_chart_three_line_break",
        &close_bars(&[10.0, 10.0, 11.0]),
        &json!({}),
    )
    .unwrap();
    assert_eq!(repeated["chart"]["bars"].as_array().unwrap().len(), 1);
}

#[test]
fn automatic_threshold_scales_across_very_different_market_prices() {
    for first in [10.0, 20_000.0] {
        let bars = close_bars(&[first, first * 1.03]);
        for (id, field) in [
            ("book_chart_renko", "brick_size"),
            ("book_chart_point_figure", "box_size"),
            ("book_chart_kagi", "reversal_size"),
        ] {
            let output = charts::evaluate(id, &bars, &json!({})).unwrap();
            assert_eq!(
                output["chart"][field].as_f64(),
                Some(first * 0.01),
                "{id}: threshold should be based on the first observed close"
            );
            assert!(
                charts::evaluate(id, &bars, &json!({(field): -1.0})).is_err(),
                "{id}: negative thresholds must fail"
            );
        }
    }
}

#[test]
fn non_market_chart_inputs_remain_unchanged() {
    let ticks = charts::evaluate(
        "book_chart_tick_bars",
        &[],
        &json!({"ticks": [10.0, 12.0, 9.0, 11.0], "ticks_per_bar": 4.0}),
    )
    .unwrap();
    assert_eq!(ticks["provenance"], "editable_teaching_inputs");
    assert_eq!(
        ticks["chart"]["bars"][0],
        json!({"open": 10.0, "high": 12.0, "low": 9.0, "close": 11.0, "direction": 1})
    );
}
