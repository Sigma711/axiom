use axiom::{supplement, types::Bar};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

fn hourly_bars(volumes: &[f64]) -> Vec<Bar> {
    let start = DateTime::from_timestamp(0, 0).unwrap();
    volumes
        .iter()
        .enumerate()
        .map(|(i, &volume)| Bar {
            timestamp: start + Duration::hours(i as i64),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume,
        })
        .collect()
}

#[test]
fn onchain_supply_counts_addresses_separately_from_tokens_and_value() {
    let out = supplement::evaluate("book_large_holdings", &[], &json!({})).unwrap();
    assert_eq!(out["values"]["address_count"], 2.0);
    assert_eq!(out["values"]["held_tokens"], 400.0);
    assert_eq!(out["values"]["supply_fraction"], 0.4);
    assert_eq!(out["values"]["held_value"], 800.0);
    assert_eq!(out["provenance"], "editable_teaching_inputs");
}

#[test]
fn funding_is_signed_and_sopr_uses_value_weighting() {
    let long = supplement::evaluate("book_funding", &[], &json!({})).unwrap();
    let short = supplement::evaluate("book_funding", &[], &json!({"is_long":false})).unwrap();
    assert_eq!(long["values"]["payment"], 1.0);
    assert_eq!(short["values"]["payment"], -1.0);
    let sopr = supplement::evaluate("book_sopr", &[], &json!({})).unwrap();
    assert_eq!(sopr["values"]["sopr"], 1.3125);
}

#[test]
fn address_deduplication_and_net_issuance_are_not_transaction_counts() {
    let out = supplement::evaluate("book_eth_depositors", &[], &json!({})).unwrap();
    assert_eq!(out["values"]["new_unique_addresses"], 2.0);
    assert_eq!(out["values"]["total_unique_addresses"], 4.0);
    let oi = supplement::evaluate("book_open_interest", &[], &json!({})).unwrap();
    assert_eq!(oi["values"]["current_count"], 1120.0);
}

#[test]
fn every_supplement_has_working_inputs_and_units() {
    for concept in supplement::catalog() {
        if matches!(
            concept.id.as_str(),
            "book_block_height"
                | "book_block_size"
                | "book_transaction_fees"
                | "book_transaction_bytes"
        ) {
            assert_eq!(concept.input_kind, "market_bars");
            assert!(concept.inputs.is_empty());
            continue;
        }
        let bars = if concept.id == "book_volume_24h" {
            hourly_bars(&[100.0; 24])
        } else {
            Vec::new()
        };
        let out = supplement::evaluate(&concept.id, &bars, &json!({}))
            .unwrap_or_else(|e| panic!("{}: {e}", concept.id));
        assert_eq!(out["status"], "computed", "{}: {out}", concept.id);
        assert!(!out["values"].as_object().unwrap().is_empty());
        for key in out["values"].as_object().unwrap().keys() {
            assert!(out["units"][key].is_string());
        }
    }
}

#[test]
fn rolling_24h_volume_requires_continuous_hourly_ohlcv_bars() {
    let volumes = (1..=24).map(f64::from).collect::<Vec<_>>();
    let bars = hourly_bars(&volumes);
    let out = supplement::evaluate("book_volume_24h", &bars, &json!({})).unwrap();
    assert_eq!(out["input_kind"], "market_bars");
    assert_eq!(out["provenance"], "provided_market_bars");
    assert_eq!(out["values"]["rolling_24h_volume"], 300.0);
    assert_eq!(out["series"][0]["values"].as_array().unwrap().len(), 24);
    let concept = supplement::catalog()
        .into_iter()
        .find(|concept| concept.id == "book_volume_24h")
        .unwrap();
    assert_eq!(concept.input_kind, "market_bars");
    assert!(concept.inputs.is_empty());

    let long_volumes = (1..=200).map(f64::from).collect::<Vec<_>>();
    let long_window =
        supplement::evaluate("book_volume_24h", &hourly_bars(&long_volumes), &json!({})).unwrap();
    assert_eq!(long_window["values"]["rolling_24h_volume"], 4524.0);
    assert_eq!(
        long_window["series"][0]["values"][0], 177.0,
        "only the final 24 hourly bars are in the rolling window"
    );
}

#[test]
fn rolling_24h_volume_rejects_manual_volumes_gaps_and_non_hourly_bars() {
    let bars = hourly_bars(&[100.0; 24]);
    let manual_volumes = vec![100.0; 24];
    assert!(supplement::evaluate(
        "book_volume_24h",
        &bars,
        &json!({"hourly_volumes": manual_volumes})
    )
    .is_err());
    assert!(supplement::evaluate("book_volume_24h", &bars[..23], &json!({})).is_err());

    let mut gapped = bars.clone();
    gapped[12].timestamp += Duration::hours(1);
    assert!(supplement::evaluate("book_volume_24h", &gapped, &json!({})).is_err());

    let daily = (0..24)
        .map(|i| Bar {
            timestamp: DateTime::<Utc>::from_timestamp(0, 0).unwrap() + Duration::days(i),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 100.0,
        })
        .collect::<Vec<_>>();
    assert!(supplement::evaluate("book_volume_24h", &daily, &json!({})).is_err());

    let mut incomplete = bars.clone();
    let now = Utc::now();
    for (i, bar) in incomplete.iter_mut().enumerate() {
        bar.timestamp = now - Duration::hours(23 - i as i64);
    }
    assert!(supplement::evaluate("book_volume_24h", &incomplete, &json!({})).is_err());
}

#[test]
fn invalid_external_data_is_rejected_and_missing_denominator_is_undefined() {
    assert!(supplement::evaluate("book_index_price", &[], &json!({"weights":[1]})).is_err());
    assert!(
        supplement::evaluate("book_open_interest", &[], &json!({"closed_contracts":2000})).is_err()
    );
    assert!(
        supplement::evaluate("book_transaction_rate", &[], &json!({"elapsed_seconds":-1})).is_err()
    );
    let out = supplement::evaluate(
        "book_supply_equality",
        &[],
        &json!({"top_one_percent_tokens":0}),
    )
    .unwrap();
    assert_eq!(out["status"], "undefined");
    assert!(out["values"]["supply_equality_ratio"].is_null());
}

#[test]
fn cape_inflation_adjusts_ten_years_before_averaging() {
    let r=axiom::supplement::evaluate("book_cape",&[],&serde_json::json!({"price":60,"annual_eps":[2,2,2,2,2,2,2,2,2,2],"annual_cpi":[100,100,100,100,100,200,200,200,200,200],"current_cpi":200})).unwrap();
    assert_eq!(r["values"]["inflation_adjusted_mean_eps"], 3.0);
    assert_eq!(r["values"]["cape"], 20.0);
    assert!(axiom::supplement::evaluate(
        "book_cape",
        &[],
        &serde_json::json!({"annual_cpi":[0,100,100,100,100,100,100,100,100,100]})
    )
    .is_err());
    assert!(axiom::supplement::evaluate(
        "book_cape",
        &[],
        &serde_json::json!({"annual_eps":[2,3]})
    )
    .is_err());
    let loss = axiom::supplement::evaluate(
        "book_cape",
        &[],
        &serde_json::json!({"annual_eps":[-2,-2,-2,-2,-2,-2,-2,-2,-2,-2]}),
    )
    .unwrap();
    assert!(loss["values"]["cape"].is_null());
}
