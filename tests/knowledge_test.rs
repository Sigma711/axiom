use axiom::knowledge;

#[test]
fn test_no_implementation_placeholder() {
    let entries = knowledge::all_entries();
    let bad: Vec<String> = entries.iter()
        .filter(|e| e.implementation.contains("未实现") || e.implementation.contains("本项目侧重"))
        .map(|e| format!("{}: {}", e.id, e.implementation))
        .collect();
    assert!(bad.is_empty(), "有 {} 个 entry 仍是占位符:\n  {}",
        bad.len(), bad.join("\n  "));
    println!("OK: 0 个占位符");
}

#[test]
fn test_all_code_refs_resolve() {
    let entries = knowledge::all_entries();
    let mut total = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for e in entries.iter() {
        let r = e.code_ref.clone();
        if r.is_empty() { continue; }
        total += 1;
        let parts: Vec<&str> = r.split("::").collect();
        let path = if parts.len() >= 2 { parts[..parts.len()-1].join("::") } else { r.clone() };
        let src_path = path.split_whitespace().next().unwrap_or(&path);
        if !std::path::Path::new(src_path).exists() {
            errors.push(format!("{}: 文件 {} 不存在", e.id, src_path));
        }
    }
    assert!(errors.is_empty(), "有 {} 个 code_ref 指向不存在文件:\n  {}",
        errors.len(), errors.join("\n  "));
    println!("OK: {} 个 code_ref 全部存在", total);
}

#[test]
fn test_diagram_present_for_key_concepts() {
    let entries = knowledge::all_entries();
    let need = ["rsi", "macd", "bbands", "ichimoku", "sharpe", "max_drawdown", "kdj", "obv"];
    let missing: Vec<&str> = need.iter().copied()
        .filter(|id| !entries.iter().any(|e| e.id.contains(id) && e.diagram.is_some()))
        .collect();
    assert!(missing.is_empty(), "缺少 diagram: {:?}", missing);
    println!("OK: 关键概念都有 diagram");
}
