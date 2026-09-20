use axiom::{book_technical as bt, types::Bar};
use serde_json::json;
fn bars(n: usize) -> Vec<Bar> {
    (0..n)
        .map(|i| Bar {
            timestamp: chrono::DateTime::from_timestamp(i as i64 * 3600, 0).unwrap(),
            open: 100.0 + i as f64 / 10.0,
            high: 103.0 + i as f64 / 10.0,
            low: 98.0 + i as f64 / 10.0,
            close: 101.0 + i as f64 / 10.0 + (i as f64).sin(),
            volume: 1000.0 + i as f64,
        })
        .collect()
}
#[test]
fn all_book_technical_cards_are_executable_and_have_real_examples() {
    let catalog = bt::catalog();
    assert!(catalog.len() >= 98);
    let bars = bars(750);
    for c in catalog {
        let result =
            bt::evaluate(&c.id, &bars, &json!({})).unwrap_or_else(|e| panic!("{}: {}", c.id, e));
        assert!(
            result["values"].as_object().is_some_and(|x| !x.is_empty()),
            "{}",
            c.id
        );
        assert!(bt::entries()
            .iter()
            .any(|e| e.id == c.id && !e.example.is_empty() && !e.summary.is_empty()));
    }
}
#[test]
fn arithmetic_reference_examples() {
    let cdp = bt::evaluate("book_cdp", &[], &json!({})).unwrap();
    assert_eq!(cdp["values"]["cdp"], 100.0);
    assert_eq!(cdp["values"]["ah"], 120.0);
    let factor = bt::evaluate(
        "book_factor_score",
        &[],
        &json!({"standardized_scores":[1.0,0.5],"weights":[0.5,0.5]}),
    )
    .unwrap();
    assert_eq!(factor["values"]["factor_score"], 0.75);
    let adjustment = bt::evaluate("book_pitfall_adjustment", &[], &json!({})).unwrap();
    assert_eq!(adjustment["values"]["economic_return"], 0.0);
}
#[test]
fn technical_series_do_not_rewrite_history_and_small_samples_do_not_panic() {
    let b = bars(180);
    for c in bt::catalog()
        .into_iter()
        .filter(|c| c.input_kind == "market_bars")
    {
        assert_eq!(
            bt::evaluate(&c.id, &[], &json!({})).unwrap()["status"],
            "undefined"
        );
        for n in [1, 2, 5, 13, 30] {
            bt::evaluate(&c.id, &b[..n], &json!({}))
                .unwrap_or_else(|e| panic!("{} {n}: {e}", c.id));
        }
        let short = bt::evaluate(&c.id, &b[..120], &json!({})).unwrap();
        let full = bt::evaluate(&c.id, &b, &json!({})).unwrap();
        for (a, z) in short["series"]
            .as_array()
            .unwrap()
            .iter()
            .zip(full["series"].as_array().unwrap())
        {
            assert_eq!(
                a["values"],
                json!(z["values"].as_array().unwrap()[..120]),
                "{} {}",
                c.id,
                a["name"]
            );
        }
    }
}
#[test]
fn constant_prices_and_rank_oracles() {
    let mut b = bars(100);
    for x in &mut b {
        x.open = 100.0;
        x.high = 100.0;
        x.low = 100.0;
        x.close = 100.0;
    }
    for id in [
        "book_tema",
        "book_kama",
        "book_alma",
        "book_mcginley",
        "book_rolling_median",
    ] {
        let result = bt::evaluate(id, &b, &json!({})).unwrap();
        let value = result["values"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .as_f64()
            .unwrap();
        assert!((value - 100.0).abs() < 1e-9, "{id}: {value}");
    }
    let r = bt::evaluate("book_ohlc_volatility", &b, &json!({})).unwrap();
    for v in r["values"].as_object().unwrap().values() {
        assert_eq!(v, 0.0);
    }
    for (i, x) in b.iter_mut().enumerate() {
        x.open = 100.0 + i as f64;
        x.close = x.open;
        x.high = x.open;
        x.low = x.open;
    }
    assert!(
        (bt::evaluate("book_rci", &b, &json!({})).unwrap()["values"]["rci"]
            .as_f64()
            .unwrap()
            - 100.0)
            .abs()
            < 1e-9
    );
    let rho = bt::evaluate("book_rho", &[], &json!({})).unwrap()["values"]["rho"]
        .as_f64()
        .unwrap();
    assert!((rho - 0.5323248).abs() < 1e-6);
}
#[test]
fn independent_boundaries_and_imported_signals_are_honest() {
    assert!(bt::evaluate(
        "book_annualized_volatility",
        &[],
        &json!({"periods_per_year":-1.0})
    )
    .is_err());
    assert_eq!(
        bt::evaluate("book_rho", &[], &json!({"time_years":0.0})).unwrap()["status"],
        "undefined"
    );
    let r = bt::evaluate(
        "book_rob_booker_adx",
        &[],
        &json!({"entry_indices":[11,21]}),
    )
    .unwrap();
    assert_eq!(r["values"]["timing_violations"], 1.0);
    assert!(r["series"][0]["values"][0].is_null());
    assert_eq!(
        bt::evaluate("book_advance_decline_ratio", &[], &json!({"declines":0.0})).unwrap()
            ["status"],
        "undefined"
    );
}
#[test]
fn source_map_covers_every_owned_section_and_appendix() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../docs/book/source-manifest.json")).unwrap();
    let map: serde_json::Value =
        serde_json::from_str(include_str!("../docs/book/technical-map.json")).unwrap();
    let mappings = map["source_mappings"].as_array().unwrap();
    let ids: std::collections::BTreeSet<_> = axiom::practice::catalog()
        .into_iter()
        .map(|c| c.id)
        .collect();
    for row in mappings {
        for id in row["concept_ids"].as_array().unwrap() {
            assert!(
                ids.contains(id.as_str().unwrap()),
                "{} -> {}",
                row["source_id"],
                id
            );
        }
    }
    for section in source["sections"].as_array().unwrap() {
        let chapter = section["chapter"].as_u64().unwrap();
        if (12..=25).contains(&chapter) || chapter == 28 {
            assert!(
                mappings.iter().any(|r| r["source_id"] == section["id"]),
                "{}",
                section["id"]
            );
        }
    }
}
#[test]
fn imported_short_signals_use_direction_and_brand_specific_evidence() {
    let r = bt::evaluate(
        "book_rob_booker_adx",
        &[],
        &json!({"sides":[1.0,-1.0],"adx_before":[18.0,18.0],"adx_now":[26.0,19.0]}),
    )
    .unwrap();
    assert!((r["series"][0]["values"][1].as_f64().unwrap() - (2.0 / 110.0 - 0.002)).abs() < 1e-10);
    assert_eq!(r["series"][1]["values"], json!([1.0, 0.0]));
    let k = bt::evaluate("book_knoxville", &[], &json!({"rsi_at_signal":[50.0,50.0]})).unwrap();
    assert_eq!(k["series"][1]["values"], json!([0.0, 0.0]));
}

#[test]
fn cointegration_uses_paired_ols_and_residual_df_oracle() {
    // x-centered is [-2,-1,0,1,2]; residuals sum to zero and are orthogonal to x.
    let r = bt::evaluate(
        "book_cointegration_diagnostic",
        &[],
        &json!({
            "asset_x":[8.,9.,10.,11.,12.], "asset_y":[17.,17.,20.,21.,25.]
        }),
    )
    .unwrap();
    let values = &r["values"];
    assert!((values["hedge_ratio"].as_f64().unwrap() - 2.0).abs() < 1e-12);
    assert!(values["intercept"].as_f64().unwrap().abs() < 1e-12);
    assert_eq!(r["series"][0]["values"], json!([1., -1., 0., -1., 1.]));
    // No-intercept DF: sum(e_lag * delta_e)=-5, sum(e_lag^2)=3,
    // SSE=5/3, residual degrees of freedom=3; t=-sqrt(15).
    assert!((values["residual_unit_root_t"].as_f64().unwrap() + 15f64.sqrt()).abs() < 1e-12);
    assert_eq!(values["df_lags"], 0.0);
    assert!(values["cointegration_p_value"].is_null());
    assert!(values["cointegration_critical_value_5pct"].is_null());
    assert!(values["half_life"].is_null());
}

#[test]
fn cointegration_degenerate_inputs_are_explicit() {
    for input in [
        json!({"asset_x":[1.,2.],"asset_y":[2.,4.]}),
        json!({"asset_x":[1.,2.,3.,4.],"asset_y":[2.,4.,6.]}),
    ] {
        assert!(bt::evaluate("book_cointegration_diagnostic", &[], &input).is_err());
    }
    for input in [
        json!({"asset_x":[1.,1.,1.,1.],"asset_y":[2.,3.,4.,5.]}),
        json!({"asset_x":[1.,2.,3.,4.],"asset_y":[2.,4.,6.,8.]}),
    ] {
        let r = bt::evaluate("book_cointegration_diagnostic", &[], &input).unwrap();
        assert!(r["values"]["residual_unit_root_t"].is_null());
        assert!(r["values"]["cointegration_p_value"].is_null());
    }
}
