use axiom::{alpha_research, types::Bar};
use serde_json::Value;

#[test]
fn all_executable_alpha158_series_match_official_qlib_operator_oracle() {
    for raw in [
        include_str!("fixtures/alpha158-qlib-oracle.json"),
        include_str!("fixtures/alpha158-qlib-edge-oracle.json"),
    ] {
        let fixture: Value = serde_json::from_str(raw).unwrap();
        assert_eq!(fixture["qlib_version"], "0.9.7");
        let bars: Vec<Bar> = serde_json::from_value(fixture["bars"].clone()).unwrap();
        let factors = fixture["factors"].as_object().unwrap();
        assert_eq!(factors.len(), 157);
        for (id, expected) in factors {
            let actual = alpha_research::research("qlib_alpha158", id, &bars).unwrap();
            let expected = expected.as_array().unwrap();
            assert_eq!(actual.series.len(), expected.len(), "{id}");
            for (index, (observed, reference)) in actual.series.iter().zip(expected).enumerate() {
                match (observed.value, reference.as_f64()) {
                    (None, None) => {}
                    (Some(a), Some(b)) => {
                        let tolerance = 1e-8_f64.max(b.abs() * 1e-8);
                        assert!(
                            (a - b).abs() <= tolerance,
                            "{id}[{index}]: Rust={a}, Qlib={b}"
                        );
                    }
                    mismatch => panic!("{id}[{index}]: {mismatch:?}"),
                }
            }
        }
    }
}
