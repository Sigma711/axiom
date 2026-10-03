use axiom::{disclosure_valuation, knowledge, practice};

#[test]
fn valuation_cases_replace_teaching_inputs_in_both_public_registries() {
    let concepts = practice::catalog();
    let entries = knowledge::all_entries();
    assert_eq!(disclosure_valuation::SUPPORTED_IDS.len(), 28);
    for id in disclosure_valuation::SUPPORTED_IDS {
        let concept = concepts.iter().find(|item| item.id == *id).unwrap();
        assert_eq!(concept.input_kind, "industry_case", "{id}");
        assert!(concept.inputs.is_empty(), "{id}");
        assert!(concept.notes.contains("固定历史估值案例"), "{id}");

        let entry = entries.iter().find(|item| item.id == *id).unwrap();
        assert!(!entry.formula.trim().is_empty(), "{id}");
        assert!(entry.signals.contains("发行人"), "{id}");
        assert!(!disclosure_valuation::fixed_symbol(id).is_empty(), "{id}");
    }
    assert_eq!(disclosure_valuation::fixed_symbol("book_ptbv"), "EBAY");
    assert_eq!(
        disclosure_valuation::fixed_symbol("book_nav_discount"),
        "DIAX"
    );
    assert_eq!(disclosure_valuation::fixed_symbol("unknown"), "");
}
