use axiom::{book_sources, knowledge, practice};
use std::collections::BTreeSet;

#[test]
fn every_source_row_and_alias_maps_to_real_executable_concepts() {
    let knowledge = knowledge::all_entries();
    let ids: BTreeSet<_> = knowledge.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids.len(), knowledge.len(), "duplicate knowledge IDs");
    let concepts = practice::catalog();
    let practice_ids: BTreeSet<_> = concepts.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, practice_ids, "knowledge/practice mismatch");
    let records = book_sources::records();
    assert_eq!(records.len(), 273 + 99 + 209 + 4 + 34);
    let mut source_ids = BTreeSet::new();
    for r in records {
        let source = r["source_id"].as_str().unwrap();
        assert!(source_ids.insert(source), "duplicate source {source}");
        assert!(
            r["pdf_page"]
                .as_u64()
                .is_some_and(|page| (1..=83).contains(&page)),
            "{source} has no valid source page"
        );
        let refs = r["concept_ids"].as_array().unwrap();
        assert!(
            !refs.is_empty(),
            "uncovered source {source}: {}",
            r["title"]
        );
        for id in refs {
            assert!(
                ids.contains(id.as_str().unwrap()),
                "{source} refers to missing {id}"
            );
        }
    }
}
