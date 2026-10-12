use serde_json::Value;
use std::collections::HashSet;
fn manifest() -> Value {
    serde_json::from_str(include_str!("../src/alpha101_catalog.json")).unwrap()
}
#[test]
fn identities_and_no_fake_execution_claims() {
    let d = manifest();
    let fs = d["factors"].as_array().unwrap();
    assert_eq!(fs.len(), 101);
    assert_eq!(d["source_url"], "https://arxiv.org/pdf/1601.00991");
    let mut ids = HashSet::new();
    let mut summaries = HashSet::new();
    for (i, f) in fs.iter().enumerate() {
        assert_eq!(f["number"], i + 1);
        assert_eq!(f["id"], format!("alpha{}", i + 1));
        assert!(ids.insert(f["id"].as_str().unwrap()));
        assert!(summaries.insert(f["summary"].as_str().unwrap()));
        assert_eq!(f["computation_status"], "catalog_only");
        assert!(!f["required_fields"].as_array().unwrap().is_empty());
        assert!(!f["requirements"].as_array().unwrap().is_empty());
        let formula = f["formula"].as_str().unwrap();
        assert!(formula.is_ascii());
        assert!(!formula.contains("Alpha#"));
        let mut depth = 0;
        for c in formula.chars() {
            if c == '(' {
                depth += 1;
            }
            if c == ')' {
                depth -= 1;
            }
            assert!(depth >= 0);
        }
        assert_eq!(depth, 0);
    }
}
#[test]
fn known_formulas_and_original_semantics() {
    let d = manifest();
    let fs = d["factors"].as_array().unwrap();
    assert_eq!(fs[5]["formula"], "(-1 * correlation(open, volume, 10))");
    assert_eq!(fs[40]["formula"], "(((high * low)^0.5) - vwap)");
    assert_eq!(
        fs[52]["formula"],
        "(-1 * delta((((close - low) - (high - close)) / (close - low)), 9))"
    );
    assert_eq!(
        fs[100]["formula"],
        "((close - open) / ((high - low) + .001))"
    );
    assert!(fs[57]["formula"].as_str().unwrap().contains("3.92795"));
    assert_eq!(
        fs[55]["required_fields"],
        serde_json::json!(["returns", "cap", "cross_section_universe"])
    );
    assert_eq!(fs[6]["adv_windows"], serde_json::json!([20]));
    assert!(fs[6]["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().unwrap().contains("平均成交金额")));
    assert!(!fs[83]["semantic_notes"].as_array().unwrap().is_empty());
    assert!(!fs[70]["semantic_notes"].as_array().unwrap().is_empty());
    let delays: Vec<u64> = fs
        .iter()
        .filter(|f| f["original_delay"] == 0)
        .map(|f| f["number"].as_u64().unwrap())
        .collect();
    assert_eq!(delays, [42, 48, 53, 54]);
}
#[test]
fn requirements_match_tokens_and_distinguish_rank_from_ts_rank() {
    let d = manifest();
    for f in d["factors"].as_array().unwrap() {
        let formula = f["formula"].as_str().unwrap().to_lowercase();
        let tokens: HashSet<&str> = formula
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
            .filter(|s| !s.is_empty())
            .collect();
        let fields: HashSet<String> = f["required_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_lowercase())
            .collect();
        for field in [
            "open", "high", "low", "close", "volume", "returns", "vwap", "cap",
        ] {
            assert_eq!(tokens.contains(field), fields.contains(field));
        }
        let cross = ["rank", "scale", "indneutralize"]
            .iter()
            .any(|t| tokens.contains(t));
        assert_eq!(f["requires_cross_section"], cross);
        assert_eq!(fields.contains("cross_section_universe"), cross);
        for token in &tokens {
            if token.starts_with("adv") || token.starts_with("indclass.") {
                assert!(fields.contains(*token));
            }
        }
    }
}
