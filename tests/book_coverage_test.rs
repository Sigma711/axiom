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

#[test]
fn coverage_report_counts_and_preserves_the_auditable_source_manifest() {
    let report = book_sources::coverage();
    assert_eq!(
        report["source_title"],
        "股票交易软件专业指标全解_完整版.pdf"
    );
    assert_eq!(report["pdf_pages"], 83);
    assert_eq!(
        report["source_records"].as_u64(),
        Some(273 + 99 + 209 + 4 + 34)
    );
    assert_eq!(
        report["mapped_records"].as_u64(),
        report["records"].as_array().map(|rows| rows
            .iter()
            .filter(|row| !row["concept_ids"].as_array().unwrap().is_empty())
            .count() as u64)
    );
    assert!(report["sha256"].as_str().unwrap().len() >= 32);
    assert!(report["note"]
        .as_str()
        .unwrap()
        .contains("不证明已接入独立真实数据"));
}
