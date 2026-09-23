use axiom::code_links;
#[path = "../build_support/source_map.rs"]
mod source_map;
#[test]
fn source_reordering_recomputes_exact_ast_case_lines() {
    let a="fn evaluate(id: &str) {\n match id {\n \"first\" => first(),\n \"second\" | \"alias\" => second(),\n _ => ()\n }\n}\n";
    let first = source_map::scan("src/example.rs", a).unwrap();
    assert_eq!(first[0].path, "src/example.rs");
    assert_eq!(
        first
            .iter()
            .find(|x| x.symbol == "evaluate::second")
            .unwrap()
            .kind,
        "match_arm"
    );
    assert_eq!(
        first
            .iter()
            .find(|x| x.symbol == "evaluate::second")
            .unwrap()
            .line,
        4
    );
    let moved = format!("// new module docs\n\n{}", a);
    let second = source_map::scan("src/example.rs", &moved).unwrap();
    assert_eq!(
        second
            .iter()
            .find(|x| x.symbol == "evaluate::second")
            .unwrap()
            .line,
        6
    );
    assert_eq!(
        second
            .iter()
            .find(|x| x.symbol == "evaluate::alias")
            .unwrap()
            .line,
        6
    );
}
#[test]
fn nested_case_beats_outer_group_and_methods_are_qualified() {
    let code="struct A; impl A { fn evaluate(&self,id:&str) { match id { \"x\"|\"y\" => {match id {\n\"x\"=>one(),\n\"y\"=>two(),\n_=>() }}, _=>() } } }";
    let rows = source_map::scan("src/a.rs", code).unwrap();
    assert_eq!(
        rows.iter()
            .find(|x| x.symbol == "A::evaluate::x")
            .unwrap()
            .line,
        2
    );
    assert!(rows.iter().any(|x| x.symbol == "A"));
}
#[test]
fn every_published_concept_resolves_to_an_actual_implementation_branch() {
    let mut missing = Vec::new();
    for e in axiom::knowledge::all_entries() {
        match code_links::for_concept(&e.id) {
            Some(location) => {
                assert!(location.line > 0 && location.end_line >= location.line);
                if e.id == "book_nonstandard_bar" {
                    assert_eq!(location.kind, "function");
                    assert_eq!(location.code_ref, "src/book.rs::nonstandard_bar_ohlc4");
                } else {
                    assert!(
                        matches!(location.kind.as_str(), "match_arm" | "conditional"),
                        "{} {:?}",
                        e.id,
                        location
                    );
                }
                let text = code_links::excerpt(&location).unwrap();
                assert!(!text.trim().is_empty());
                if location.dirty {
                    assert!(location.github_url.is_none());
                    assert!(location.url.starts_with("/api/code/source?"));
                } else if let Some(url) = location.github_url {
                    assert!(url.contains(location.revision.as_ref().unwrap()));
                    assert!(!url.contains("/main/"));
                }
            }
            None => missing.push(e.id),
        }
    }
    assert!(
        missing.is_empty(),
        "Missing precise implementation references: {}",
        missing.join(",")
    );
}
#[test]
fn embedded_source_lookup_cannot_read_arbitrary_files() {
    assert!(code_links::source("../../.env").is_none());
    assert!(code_links::source("/etc/passwd").is_none());
    assert!(code_links::source("src/practice/market.rs")
        .unwrap()
        .contains("fn evaluate"));
    assert!(code_links::resolve("src/practice/market.rs::does_not_exist").is_none());
}

#[test]
fn published_builds_link_every_concept_to_the_exact_github_revision() {
    if std::env::var("CI").as_deref() != Ok("true")
        && std::env::var("AXIOM_REQUIRE_GITHUB_LINKS").as_deref() != Ok("1")
    {
        return;
    }
    assert!(
        !code_links::is_dirty(),
        "Published builds must be created from a clean committed checkout"
    );
    let revision = code_links::revision().expect("Git revision must be embedded");
    for entry in axiom::knowledge::all_entries() {
        let location = code_links::for_concept(&entry.id).unwrap();
        let expected = format!(
            "https://github.com/Sigma711/axiom/blob/{revision}/{}#L{}-L{}",
            location.path, location.line, location.end_line
        );
        assert_eq!(entry.code_url, expected, "{}", entry.id);
        assert_eq!(location.github_url.as_deref(), Some(expected.as_str()));
    }
}

#[test]
fn learning_navigation_references_resolve_to_exact_source_symbols() {
    for reference in [
        "src/types.rs::Bar",
        "src/data.rs::DataFeed",
        "src/indicators/ma.rs::sma",
        "src/strategy.rs::Strategy",
        "src/risk.rs::RiskManager::allow_order",
        "src/portfolio.rs::Portfolio::on_signal",
        "src/broker.rs::Broker",
        "src/engine.rs::BacktestEngine::run",
        "src/metrics.rs::compute_metrics",
        "src/paper.rs::run_paper_loop",
        "src/workflows.rs::entries",
    ] {
        let location =
            code_links::resolve(reference).unwrap_or_else(|| panic!("missing {reference}"));
        assert!(
            location.line > 0 && location.end_line >= location.line,
            "{reference}"
        );
        assert!(
            !code_links::excerpt(&location).unwrap().trim().is_empty(),
            "{reference}"
        );
    }
}

#[test]
fn legacy_short_names_resolve_only_when_the_method_is_unambiguous() {
    let location = code_links::resolve("src/risk.rs::allow_order")
        .expect("one RiskManager allow_order method");
    assert_eq!(location.symbol, "RiskManager::allow_order");
    assert_eq!(location.code_ref, "src/risk.rs::RiskManager::allow_order");
    assert!(code_links::resolve("src/data.rs::new").is_none());
}
