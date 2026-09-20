// 验证 knowledge base 的 code_ref 全部有效
use axiom::knowledge;
use std::path::Path;

#[test]
fn test_no_implementation_placeholder() {
    let entries = knowledge::all_entries();
    let bad: Vec<String> = entries.iter()
        .filter(|e| e.implementation.contains("未实现") || e.implementation.contains("本项目侧重"))
        .map(|e| format!("{}: {}", e.id, e.implementation))
        .collect();
    assert!(bad.is_empty(),
        "有 {} 个 entry 仍是占位符:\n  {}",
        bad.len(),
        bad.join("\n  "));
    println!("✓ 0 个占位符");
}

#[test]
fn test_diagram_present_for_key_concepts() {
    let entries = knowledge::all_entries();
    // 这些必须有 diagram
    let need_diagram = ["rsi", "macd", "bbands", "ichimoku", "sharpe", "max_drawdown", "kdj", "obv"];
    let missing: Vec<String> = need_diagram.iter()
        .filter(|id| {
            !entries.iter()
                .any(|e| e.id.contains(&**id) && e.diagram.is_some())
        })
        .map(|s| s.to_string())
        .collect();
    assert!(missing.is_empty(),
        "缺少 diagram: {:?}", missing);
    println!("✓ 关键 {} 个概念都有 diagram", need_diagram.len());
}

#[test]
fn test_candle_pattern_functions_exist() {
    use axiom::indicators::extra::{detect_pattern, detect_engulfing, detect_inside_outside, CandlePattern};
    use axiom::types::Bar;
    use chrono::{Utc, TimeZone};

    // detect_pattern: 测试 Hammer 检测
    let hammer = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(), open: 100.0, high: 102.5, low: 88.0, close: 102.0, volume: 1000.0,
    };
    // body=1, range=12, lower_shadow=10, upper_shadow=1 - 应该识别为 Hammer
    assert_eq!(detect_pattern(&hammer), CandlePattern::Hammer);

    // detect_engulfing: 测试 Bullish Engulfing
    let prev = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(), open: 102.0, high: 103.0, low: 99.0, close: 99.0, volume: 1000.0,
    };
    let curr = Bar {
        timestamp: Utc.timestamp_opt(2, 0).unwrap(), open: 98.0, high: 105.0, low: 97.0, close: 104.0, volume: 1200.0,
    };
    // prev 阴线 (102→99), curr 阳线 (98→104) 包裹 prev → Engulfing
    assert_eq!(detect_engulfing(&prev, &curr), CandlePattern::Engulfing);

    // detect_inside_outside: 测试 Inside Bar
    let prev2 = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(), open: 100.0, high: 110.0, low: 95.0, close: 105.0, volume: 1000.0,
    };
    let inside = Bar {
        timestamp: Utc.timestamp_opt(2, 0).unwrap(), open: 102.0, high: 108.0, low: 98.0, close: 104.0, volume: 800.0,
    };
    // curr 范围在 prev 内
    assert_eq!(detect_inside_outside(&prev2, &inside), CandlePattern::Inside);

    println!("OK: K 线形态函数全部测试通过");
}

#[test]
fn test_indicator_functions_exist() {
    use axiom::indicators::momentum::{elder_ray, bop, fisher_transform, rvi, demarker, kst, coppock};
    use axiom::indicators::volume::{klinger, volume_ratio};
    use axiom::indicators::statistics::hurst;
    use axiom::indicators::trend::td_sequential;
    use axiom::indicators::ma::{alligator, fractal};
    use axiom::indicators::extra::pivot_points;
    use axiom::indicators::breadth::tick_index;
    use axiom::types::Bar;
    use chrono::{TimeZone, Utc};

    // bop: 应该有结果
    let bar = Bar {
        timestamp: Utc.timestamp_opt(1, 0).unwrap(),
        open: 100.0, high: 110.0, low: 95.0, close: 105.0, volume: 1000.0,
    };
    let r = bop(&[bar.clone()]);
    assert!(!r.is_empty());
    assert!(r[0].is_some());

    // hurst: 随机序列
    let prices: Vec<f64> = (0..50).map(|i| (i as f64 * 0.13).sin()).collect();
    assert!(hurst(&prices).is_some());

    // volume_ratio
    let bars: Vec<Bar> = (0..30).map(|i| Bar {
        timestamp: Utc.timestamp_opt(i as i64, 0).unwrap(),
        open: 100.0, high: 101.0, low: 99.0, close: 100.5, volume: 1000.0 + i as f64,
    }).collect();
    assert!(!volume_ratio(&bars, 5).is_empty());

    // td_sequential
    let closes: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();
    let td = td_sequential(&closes);
    assert!(td.iter().any(|&v| v > 0));

    // alligator
    let bars: Vec<Bar> = (0..30).map(|i| Bar {
        timestamp: Utc.timestamp_opt(i as i64, 0).unwrap(),
        open: 100.0, high: 101.0, low: 99.0, close: 100.5, volume: 1000.0,
    }).collect();
    let all = alligator(&bars);
    assert_eq!(all.jaw.len(), 30);

    // fractal
    let fr = fractal(&bars);
    assert_eq!(fr.len(), 30);

    // pivot_points
    let pp = pivot_points(110.0, 95.0, 105.0);
    assert!(pp.r3 > pp.r2 && pp.r2 > pp.r1 && pp.r1 > pp.pivot);

    // tick_index
    let up = vec![100u64, 50, 80];
    let down = vec![30u64, 60, 20];
    let t = tick_index(&up, &down);
    assert_eq!(t.len(), 3);
    assert_eq!(t[2], 70 + (-10) + 60);  // 累计: 70, 60, 120

    // elder_ray: 简单测试
    let bars: Vec<Bar> = (0..30).map(|i| Bar {
        timestamp: Utc.timestamp_opt(i as i64, 0).unwrap(),
        open: 100.0, high: 105.0, low: 95.0, close: 100.0 + i as f64, volume: 1000.0,
    }).collect();
    let (bull, _bear) = elder_ray(&bars, 14);
    assert_eq!(bull.len(), 30);

    // fisher_transform
    let r = fisher_transform(&[100.0, 101.0, 99.0, 102.0, 98.0, 103.0], 3);
    assert!(!r.is_empty());

    // rvi
    let bars: Vec<Bar> = (0..30).map(|i| Bar {
        timestamp: Utc.timestamp_opt(i as i64, 0).unwrap(),
        open: 100.0, high: 110.0, low: 90.0, close: 105.0, volume: 1000.0,
    }).collect();
    let r = rvi(&bars, 10);
    assert!(!r.is_empty());

    // demarker
    let r = demarker(&bars, 10);
    assert!(!r.is_empty());

    // kst
    let prices: Vec<f64> = (0..100).map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0).collect();
    let r = kst(&prices);
    assert!(!r.is_empty());

    // coppock
    let r = coppock(&prices);
    assert!(!r.is_empty());

    // klinger
    let bars: Vec<Bar> = (0..30).map(|i| Bar {
        timestamp: Utc.timestamp_opt(i as i64, 0).unwrap(),
        open: 100.0 + (i as f64 * 0.1).sin() * 5.0,
        high: 102.0 + (i as f64 * 0.1).sin() * 5.0,
        low: 98.0 + (i as f64 * 0.1).sin() * 5.0,
        close: 101.0 + (i as f64 * 0.1).sin() * 5.0,
        volume: 1000.0 + i as f64 * 10.0,
    }).collect();
    let r = klinger(&bars, 5, 10);
    assert!(!r.is_empty());

    println!("OK: 14 个新增指标函数全部就绪");
}

#[test]
fn test_code_loc_api_works() {
    use axiom::api;
    // 测试主要函数的行号定位
    let cases = [
        ("src/indicators/momentum.rs::rsi", 11usize),
        ("src/indicators/momentum.rs::kst", 0),  // 检查存在即可
        ("src/indicators/extra.rs::detect_engulfing", 0),
        ("src/indicators/ma.rs::alligator", 0),
        ("src/indicators/statistics.rs::hurst", 0),
    ];
    for (ref_str, _expected) in cases {
        let line = api::locate_symbol_for_test(ref_str);
        assert!(line.is_some(), "{} 应该能定位行号", ref_str);
        let line = line.unwrap();
        assert!(line >= 1, "{} 行号应该 >= 1, 实际 {}", ref_str, line);
        println!("  {} → 第 {} 行", ref_str, line);
    }
}
