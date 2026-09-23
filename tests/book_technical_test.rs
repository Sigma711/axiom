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
        let mut daily_cdp_bars = Vec::new();
        if c.id == "book_cdp" {
            daily_cdp_bars = bars.clone();
            for (index, bar) in daily_cdp_bars.iter_mut().enumerate() {
                bar.timestamp = chrono::DateTime::from_timestamp(index as i64 * 86_400, 0).unwrap();
            }
        }
        let selected_bars = if c.id == "book_cdp" {
            &daily_cdp_bars
        } else {
            &bars
        };
        let result = bt::evaluate(&c.id, selected_bars, &json!({}))
            .unwrap_or_else(|e| panic!("{}: {}", c.id, e));
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
    let mut cdp_bars = bars(2);
    cdp_bars[0].high = 110.0;
    cdp_bars[0].low = 90.0;
    cdp_bars[0].close = 100.0;
    cdp_bars[1].open = 122.0;
    cdp_bars[1].high = 130.0;
    cdp_bars[1].low = 120.0;
    cdp_bars[1].close = 125.0;
    let hourly = bt::evaluate("book_cdp", &cdp_bars, &json!({}));
    assert!(hourly.is_err());
    cdp_bars[1].timestamp = cdp_bars[0].timestamp + chrono::Duration::days(1);
    let cdp = bt::evaluate("book_cdp", &cdp_bars, &json!({})).unwrap();
    assert_eq!(cdp["input_kind"], "market_bars");
    assert_eq!(cdp["provenance"], "provided_market_bars");
    assert_eq!(cdp["values"]["cdp"], 100.0);
    assert_eq!(cdp["values"]["ah"], 120.0);
    assert_eq!(cdp["values"]["al"], 80.0);
    assert_eq!(cdp["values"]["nh"], 110.0);
    assert_eq!(cdp["values"]["nl"], 90.0);
    assert!(cdp["notes"][0]
        .as_str()
        .is_some_and(|note| note.contains("1970-01-02T00:00:00+00:00")));
    let undefined = bt::evaluate("book_cdp", &cdp_bars[..1], &json!({})).unwrap();
    assert_eq!(undefined["status"], "undefined");
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
    let mut daily_b = b.clone();
    for (index, bar) in daily_b.iter_mut().enumerate() {
        bar.timestamp = chrono::DateTime::from_timestamp(index as i64 * 86_400, 0).unwrap();
    }
    for c in bt::catalog()
        .into_iter()
        .filter(|c| c.input_kind == "market_bars")
    {
        let selected_bars = if c.id == "book_cdp" { &daily_b } else { &b };
        assert_eq!(
            bt::evaluate(&c.id, &[], &json!({})).unwrap()["status"],
            "undefined"
        );
        for n in [1, 2, 5, 13, 30] {
            bt::evaluate(&c.id, &selected_bars[..n], &json!({}))
                .unwrap_or_else(|e| panic!("{} {n}: {e}", c.id));
        }
        let short = bt::evaluate(&c.id, &selected_bars[..120], &json!({})).unwrap();
        let full = bt::evaluate(&c.id, selected_bars, &json!({})).unwrap();
        for (a, z) in short["series"]
            .as_array()
            .unwrap()
            .iter()
            .zip(full["series"].as_array().unwrap())
        {
            if c.id == "book_pitfall_repainting"
                && a["name"].as_str().unwrap().ends_with("occurrence")
            {
                // This is deliberately a retrospective display. The separate
                // confirmation series is the no-lookahead signal.
                continue;
            }
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
fn repainting_practice_separates_retroactive_pivot_location_from_confirmation() {
    let highs = [3.0, 4.0, 10.0, 5.0, 4.0, 7.0, 3.0];
    let bars: Vec<_> = highs
        .into_iter()
        .enumerate()
        .map(|(index, high)| Bar {
            timestamp: chrono::DateTime::from_timestamp(index as i64 * 3600, 0).unwrap(),
            open: 2.0,
            high,
            low: 1.0,
            close: 2.0,
            volume: 1.0,
        })
        .collect();
    let early = bt::evaluate("book_pitfall_repainting", &bars[..4], &json!({})).unwrap();
    assert!(early["series"]
        .as_array()
        .unwrap()
        .iter()
        .all(|series| series["values"]
            .as_array()
            .unwrap()
            .iter()
            .all(serde_json::Value::is_null)));

    let result = bt::evaluate("book_pitfall_repainting", &bars, &json!({})).unwrap();
    assert_eq!(result["input_kind"], "market_bars");
    assert_eq!(result["provenance"], "provided_market_bars");
    assert_eq!(result["inputs"], json!({}));
    assert_eq!(result["values"]["right_confirmation_bars"], 2.0);
    let series = result["series"].as_array().unwrap();
    let occurrence = series
        .iter()
        .find(|item| item["name"] == "pivot_high_occurrence")
        .unwrap();
    let confirmed = series
        .iter()
        .find(|item| item["name"] == "confirmed_pivot_high")
        .unwrap();
    let delay = series
        .iter()
        .find(|item| item["name"] == "confirmation_delay_bars")
        .unwrap();
    assert_eq!(occurrence["values"][2], 10.0);
    assert!(confirmed["values"][2].is_null());
    assert_eq!(confirmed["values"][4], 10.0);
    assert_eq!(delay["values"][4], 2.0);
    assert!(result["notes"].as_array().unwrap().iter().any(|note| note
        .as_str()
        .is_some_and(|text| text.contains("绝不是可执行信号"))));

    let mut outside = bars.clone();
    outside[2].low = 0.1;
    let outside_result = bt::evaluate("book_pitfall_repainting", &outside, &json!({})).unwrap();
    let outside_series = outside_result["series"].as_array().unwrap();
    let low_occurrence = outside_series
        .iter()
        .find(|item| item["name"] == "pivot_low_occurrence")
        .unwrap();
    let low_confirmed = outside_series
        .iter()
        .find(|item| item["name"] == "confirmed_pivot_low")
        .unwrap();
    assert_eq!(low_occurrence["values"][2], 0.1);
    assert_eq!(low_confirmed["values"][4], 0.1);

    let flat: Vec<_> = (0..4)
        .map(|index| Bar {
            timestamp: chrono::DateTime::from_timestamp(index * 3600, 0).unwrap(),
            open: 2.0,
            high: 2.0,
            low: 2.0,
            close: 2.0,
            volume: 1.0,
        })
        .collect();
    let empty = bt::evaluate("book_pitfall_repainting", &flat, &json!({})).unwrap();
    assert_eq!(empty["values"]["confirmed_pivot_count"], 0.0);
    assert!(empty["notes"].as_array().unwrap().iter().any(|note| note
        .as_str()
        .is_some_and(|text| text.contains("不是加载失败"))));
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
    let cdp = bt::entries()
        .into_iter()
        .find(|entry| entry.id == "book_cdp")
        .unwrap();
    assert!(cdp.signals.contains("适用模块"));
    assert!(!cdp.signals.contains("四页同一计算"));
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

#[test]
fn r_squared_matches_the_worked_pearson_example_and_leaves_constant_series_undefined() {
    let result = bt::evaluate(
        "book_r_squared",
        &[],
        &json!({"strategy_returns":[0.05,-0.01904761904761909,0.03],"benchmark_returns":[0.1,-0.1,0.1]}),
    ).unwrap();
    assert!(
        (result["values"]["r_squared"].as_f64().unwrap() - 0.920_773_699_023_893_5).abs() < 1e-12
    );
    let constant = bt::evaluate(
        "book_r_squared",
        &[],
        &json!({"strategy_returns":[0.01,0.01],"benchmark_returns":[0.02,0.03]}),
    )
    .unwrap();
    assert_eq!(constant["status"], "undefined");
    assert!(constant["values"]["r_squared"].is_null());
}
