use axiom::{industry_case::SUPPORTED_IDS, knowledge, practice};

#[test]
fn industry_cases_replace_teaching_inputs_in_both_public_registries() {
    let concepts = practice::catalog();
    let entries = knowledge::all_entries();
    for id in SUPPORTED_IDS {
        let concept = concepts.iter().find(|item| item.id == *id).unwrap();
        assert_eq!(concept.input_kind, "industry_case", "{id}");
        assert!(concept.inputs.is_empty(), "{id}");
        assert!(concept.notes.contains("固定发行人历史披露"), "{id}");
        for internal_term in ["服务器", "SHA-256", "selected_dataset", "symbol"] {
            assert!(
                !concept.notes.contains(internal_term),
                "{id}: {internal_term}"
            );
        }

        let entry = entries.iter().find(|item| item.id == *id).unwrap();
        assert!(entry.signals.contains("已验证的发行人历史披露"), "{id}");
        assert!(!entry.formula.trim().is_empty(), "{id}");
    }
}
