use axiom::alpha_research;
use serde_json::Value;

#[test]
fn default_alpha158_identity_and_expressions_match_pinned_qlib_generator() {
    let official: Value = serde_json::from_str(include_str!(
        "../docs/research/alpha158-official-expressions.json"
    ))
    .unwrap();
    let expected = official["features"].as_array().unwrap();
    let collections = alpha_research::catalog();
    let actual = collections
        .iter()
        .find(|collection| collection.id == "qlib_alpha158")
        .unwrap();
    assert_eq!(actual.total, 158);
    assert_eq!(actual.factors.len(), expected.len());
    for (index, (factor, source)) in actual.factors.iter().zip(expected).enumerate() {
        assert_eq!(factor.id, source["id"].as_str().unwrap(), "index {index}");
        let generated = source["expression"].as_str().unwrap();
        let implemented = factor.formula.as_deref().unwrap();
        let without_spaces = |expression: &str| {
            expression
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        };
        assert_eq!(
            without_spaces(implemented),
            without_spaces(generated),
            "{}",
            factor.id
        );
        if factor.status == "available" {
            assert!(factor.code_url.is_some(), "{}", factor.id);
        }
    }
}
