use axiom::indicators::extra::{
    detect_engulfing, detect_inside_outside, detect_pattern, CandlePattern,
};
use axiom::knowledge;
use axiom::types::Bar;
use chrono::{TimeZone, Utc};
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

#[test]
fn test_k_pattern_hammer_function_works() {
    // body=1, range=12, lower_shadow=10, upper_shadow=1 -> Hammer
    let bar = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(),
        open: 100.0,
        high: 101.2,
        low: 95.0,
        close: 101.0,
        volume: 1000.0,
    };
    assert_eq!(
        detect_pattern(&bar),
        CandlePattern::Hammer,
        "k_pattern_hammer 指向的 detect_pattern 必须能识别锤子线"
    );
}

#[test]
fn test_k_pattern_engulfing_function_works() {
    // 前阴后阳, 后阳实体完全包裹前阴 -> Engulfing
    let prev = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(),
        open: 102.0,
        high: 103.0,
        low: 99.0,
        close: 99.0,
        volume: 1000.0,
    };
    let curr = Bar {
        timestamp: Utc.timestamp_opt(2, 0).unwrap(),
        open: 98.0,
        high: 105.0,
        low: 97.0,
        close: 104.0,
        volume: 1200.0,
    };
    assert_eq!(
        detect_engulfing(&prev, &curr),
        CandlePattern::Engulfing,
        "k_pattern_engulfing 指向的 detect_engulfing 必须能识别吞没形态"
    );
}

#[test]
fn test_inside_outside_function_works() {
    let prev = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(),
        open: 100.0,
        high: 110.0,
        low: 95.0,
        close: 105.0,
        volume: 1000.0,
    };
    let inside = Bar {
        timestamp: Utc.timestamp_opt(2, 0).unwrap(),
        open: 102.0,
        high: 108.0,
        low: 98.0,
        close: 104.0,
        volume: 800.0,
    };
    assert_eq!(
        detect_inside_outside(&prev, &inside),
        CandlePattern::Inside,
        "inside_outside 指向的 detect_inside_outside 必须能识别内包线"
    );
}
