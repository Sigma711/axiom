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
fn chart_prefixes_are_immutable_and_tick_path_is_explicit() {
    for c in charts::catalog() {
        let defaults = c
            .inputs
            .iter()
            .map(|i| (i.key.clone(), i.default.clone()))
            .collect::<serde_json::Map<_, _>>();
        let short =
            charts::evaluate(&c.id, &[], &serde_json::Value::Object(defaults.clone())).unwrap();
        let full = charts::evaluate(&c.id, &[], &serde_json::Value::Object(defaults)).unwrap();
        assert_eq!(
            short["series"], full["series"],
            "{} must not rewrite history",
            c.id
        );
    }
    assert!(charts::evaluate("book_chart_tick_bars", &[], &json!({"ticks_per_bar":0.0})).is_err());
    assert!(charts::evaluate("book_chart_renko", &[], &json!({"unknown":1.0})).is_err());
}
