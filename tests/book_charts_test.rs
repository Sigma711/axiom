use axiom::book_charts as charts;
use serde_json::json;

#[test]
fn each_chart_has_an_independent_worked_answer() {
    let cases: &[(&str, serde_json::Value, &str, f64)] = &[
        (
            "book_chart_heikin_ashi",
            json!({"open":[10.],"high":[12.],"low":[9.],"close":[11.]}),
            "ha_close",
            10.5,
        ),
        (
            "book_chart_renko",
            json!({"prices":[10.,12.1],"brick_size":1.}),
            "renko_close",
            12.,
        ),
        (
            "book_chart_point_figure",
            json!({"prices":[10.,12.],"box_size":1.,"reversal_boxes":3.}),
            "point_figure_box",
            12.,
        ),
        (
            "book_chart_kagi",
            json!({"prices":[10.,12.,10.8],"reversal_size":1.}),
            "kagi_turn",
            10.8,
        ),
        (
            "book_chart_three_line_break",
            json!({"prices":[10.,9.,8.,7.,11.],"line_count":3.}),
            "three_line_close",
            11.,
        ),
        (
            "book_chart_range_bars",
            json!({"ticks":[10.,10.4,11.],"range_size":1.}),
            "range_close",
            11.,
        ),
        (
            "book_chart_tick_bars",
            json!({"ticks":[10.,11.,12.,13.,14.,15.],"ticks_per_bar":3.}),
            "tick_close",
            15.,
        ),
    ];
    for (id, input, name, expected) in cases {
        let r = charts::evaluate(id, &[], input).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(r["values"][name].as_f64(), Some(*expected), "{id}");
    }
}

#[test]
fn chart_outputs_have_real_ohlc_and_stable_completed_prefixes() {
    let cases = [
        (
            "book_chart_heikin_ashi",
            json!({"open":[10.,11.,12.],"high":[12.,13.,14.],"low":[9.,10.,11.],"close":[11.,12.,13.]}),
        ),
        ("book_chart_renko", json!({"prices":[10.,12.1,13.1]})),
        ("book_chart_point_figure", json!({"prices":[10.,12.,13.]})),
        ("book_chart_kagi", json!({"prices":[10.,12.,13.]})),
        (
            "book_chart_three_line_break",
            json!({"prices":[10.,9.,8.,7.]}),
        ),
        (
            "book_chart_range_bars",
            json!({"ticks":[10.,10.4,11.,11.2]}),
        ),
        (
            "book_chart_tick_bars",
            json!({"ticks":[10.,11.,12.,13.,14.,15.]}),
        ),
    ];
    for (id, prefix) in cases {
        let mut full = prefix.clone();
        let key = if id == "book_chart_heikin_ashi" {
            "close"
        } else if id == "book_chart_range_bars" || id == "book_chart_tick_bars" {
            "ticks"
        } else {
            "prices"
        };
        full[key].as_array_mut().unwrap().push(json!(99.));
        if id == "book_chart_heikin_ashi" {
            full["open"].as_array_mut().unwrap().push(json!(98.));
            full["high"].as_array_mut().unwrap().push(json!(100.));
            full["low"].as_array_mut().unwrap().push(json!(97.));
        }
        let before = charts::evaluate(id, &[], &prefix).unwrap();
        let after = charts::evaluate(id, &[], &full).unwrap();
        let bars = before["chart"]["bars"].as_array().unwrap();
        assert!(bars.iter().all(|bar| bar["open"].is_number()
            && bar["high"].is_number()
            && bar["low"].is_number()
            && bar["close"].is_number()));
        assert!(bars.iter().all(|bar| bar["low"].as_f64().unwrap()
            <= bar["open"]
                .as_f64()
                .unwrap()
                .min(bar["close"].as_f64().unwrap())
            && bar["high"].as_f64().unwrap()
                >= bar["open"]
                    .as_f64()
                    .unwrap()
                    .max(bar["close"].as_f64().unwrap())));
        assert_eq!(
            bars.as_slice(),
            &after["chart"]["bars"].as_array().unwrap()[..bars.len()],
            "{id} rewrote completed history"
        );
    }
}

#[test]
fn reversal_thresholds_and_tick_ohlc_have_independent_oracles() {
    let kagi = charts::evaluate(
        "book_chart_kagi",
        &[],
        &json!({"prices":[10.,10.2],"reversal_size":1.}),
    )
    .unwrap();
    assert_eq!(
        kagi["chart"]["bars"],
        json!([]),
        "sub-threshold initial movement is not a Kagi line"
    );
    let kagi_switch = charts::evaluate(
        "book_chart_kagi",
        &[],
        &json!({"prices":[10.,12.,10.,13.],"reversal_size":1.}),
    )
    .unwrap();
    let kagi_bars = kagi_switch["chart"]["bars"].as_array().unwrap();
    assert_eq!(kagi_bars[2]["line_style"], "yang");
    assert_eq!(kagi_bars[2]["switch_price"], 12.0);
    let renko = charts::evaluate(
        "book_chart_renko",
        &[],
        &json!({"prices":[10.,12.,10.],"brick_size":1.}),
    )
    .unwrap();
    assert_eq!(renko["chart"]["bars"][2]["open"], 11.0);
    assert_eq!(renko["chart"]["bars"][2]["close"], 10.0);
    let line_break = charts::evaluate(
        "book_chart_three_line_break",
        &[],
        &json!({"prices":[10.,9.,8.,7.,9.5],"line_count":3.}),
    )
    .unwrap();
    assert_eq!(
        line_break["chart"]["bars"].as_array().unwrap().len(),
        3,
        "9.5 must not reverse through the oldest line open of 10"
    );
    let startup_reversal = charts::evaluate(
        "book_chart_three_line_break",
        &[],
        &json!({"prices":[10.,9.,11.],"line_count":3}),
    )
    .unwrap();
    assert_eq!(
        startup_reversal["chart"]["bars"].as_array().unwrap().len(),
        2,
        "启动阶段使用已有线的高低；11 突破首条线最高 10 应反转"
    );
    let pf = charts::evaluate(
        "book_chart_point_figure",
        &[],
        &json!({"reversal_boxes":1.5}),
    );
    assert!(pf.is_err(), "P&F reversal count must be integral");
    let ticks = charts::evaluate(
        "book_chart_tick_bars",
        &[],
        &json!({"ticks":[10.,12.,9.,11.],"ticks_per_bar":4.}),
    )
    .unwrap();
    assert_eq!(
        ticks["chart"]["bars"][0],
        json!({"open":10.0,"high":12.0,"low":9.0,"close":11.0,"direction":1})
    );
    let range = charts::evaluate(
        "book_chart_range_bars",
        &[],
        &json!({"ticks":[10.,9.,11.],"range_size":1.}),
    )
    .unwrap();
    assert_eq!(
        range["chart"]["bars"][0],
        json!({"open":10.0,"high":10.0,"low":9.0,"close":9.0,"direction":-1})
    );
}

#[test]
fn chart_inputs_reject_invalid_thresholds() {
    assert!(charts::evaluate("book_chart_tick_bars", &[], &json!({"ticks_per_bar":0.0})).is_err());
    assert!(charts::evaluate("book_chart_renko", &[], &json!({"unknown":1.0})).is_err());
}
