use axiom::knowledge;
use std::path::Path;

#[test]
fn test_no_implementation_placeholder() {
    let entries = knowledge::all_entries();
    let bad: Vec<String> = entries
        .iter()
        .filter(|e| e.implementation.contains("未实现") || e.implementation.contains("本项目侧重"))
        .map(|e| format!("{}: {}", e.id, e.implementation))
        .collect();
    assert!(
        bad.is_empty(),
        "有 {} 个 entry 仍是占位符:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
    println!("OK: 0 个占位符");
}

#[test]
fn test_all_code_refs_point_to_existing_files() {
    let entries = knowledge::all_entries();
    let mut total = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for e in entries.iter() {
        let r = e.code_ref.clone();
        if r.is_empty() || !r.starts_with("src/") {
            continue;
        }
        total += 1;
        let parts: Vec<&str> = r.split(" - ").collect();
        let code_part = parts[0];
        let segs: Vec<&str> = code_part.split("::").collect();
        let path = segs[0];
        if !Path::new(path).exists() {
            errors.push(format!("{}: 文件 {} 不存在", e.id, path));
        }
    }
    assert!(
        errors.is_empty(),
        "有 {} 个 code_ref 指向不存在文件:\n  {}",
        errors.len(),
        errors.join("\n  ")
    );
    println!("OK: {} 个 code_ref 全部指向真实文件", total);
}

#[test]
fn test_diagram_present_for_key_concepts() {
    let entries = knowledge::all_entries();
    let need = [
        "rsi",
        "macd",
        "bbands",
        "ichimoku",
        "sharpe",
        "max_drawdown",
        "kdj",
        "obv",
    ];
    let missing: Vec<&str> = need
        .iter()
        .copied()
        .filter(|id| {
            !entries
                .iter()
                .any(|e| e.id.contains(id) && e.diagram.is_some())
        })
        .collect();
    assert!(missing.is_empty(), "缺少 diagram: {:?}", missing);
    println!("OK: 关键概念都有 diagram");
}
