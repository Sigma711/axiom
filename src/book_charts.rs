//! 可复算的非时间图表；所有路径只来自调用方显式提供的有序输入。
use crate::{
    knowledge::KnowledgeEntry,
    practice::{arr, num, Output, PracticeConcept, PracticeInput},
    types::Bar,
};
use serde_json::{json, Value};

const IDS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "book_chart_heikin_ashi",
        "Heikin-Ashi 平均K线",
        "HA收盘=(O+H+L+C)/4；HA开盘=(前HA开盘+前HA收盘)/2",
        "首根 OHLC(10,12,9,11) 的 HA 收盘为 10.5。",
        "使用已收盘 OHLC；不从 OHLC 推断逐笔路径。",
    ),
    (
        "book_chart_renko",
        "Renko 固定砖",
        "收盘价每跨越一个固定砖宽生成一砖",
        "价格 10→12.1、砖宽 1，生成两块向上砖。",
        "close-only 砖图不表示盘中触及顺序。",
    ),
    (
        "book_chart_point_figure",
        "点数图 P&F",
        "固定箱格，反转须达到指定箱格数",
        "箱格 1、反转 3 时，价格反向移动 3 才换列。",
        "仅由顺序价格输入重建。",
    ),
    (
        "book_chart_kagi",
        "Kagi 线",
        "价格反向达到反转幅度才改变线方向",
        "上行到 12 后回落 1，反转幅度 1 时转为下行。",
        "不从 OHLC 假装识别盘中反转。",
    ),
    (
        "book_chart_three_line_break",
        "三线突破",
        "收盘价突破最近三条同向线极值才生成反向线",
        "下行线的最近三条最高为 10，收盘高于 10 才反转。",
        "只处理提供的已收盘价格序列。",
    ),
    (
        "book_chart_range_bars",
        "Range Bars 区间K线",
        "每根完成条覆盖固定价格区间",
        "成交价格从 10 到 11、区间 1，完成一根区间条。",
        "必须提供有序成交价；OHLC 无法还原成交路径。",
    ),
    (
        "book_chart_tick_bars",
        "Tick Bars 成交笔数K线",
        "每固定笔数成交组成一根K线",
        "每 3 笔成交组成一根条。",
        "必须提供有序逐笔成交价；不把OHLC当作逐笔。",
    ),
];
fn defaults(id: &str) -> Value {
    match id {
        "book_chart_heikin_ashi" => {
            json!({"open":[10.,11.],"high":[12.,13.],"low":[9.,10.],"close":[11.,12.]})
        }
        "book_chart_renko" => json!({"prices":[10.,12.1,11.],"brick_size":1.}),
        "book_chart_point_figure" => {
            json!({"prices":[10.,12.,9.,8.],"box_size":1.,"reversal_boxes":3.})
        }
        "book_chart_kagi" => json!({"prices":[10.,12.,10.8,9.],"reversal_size":1.}),
        "book_chart_three_line_break" => json!({"prices":[10.,9.,8.,7.,9.],"line_count":3.}),
        "book_chart_range_bars" => json!({"ticks":[10.,10.4,11.,11.2],"range_size":1.}),
        "book_chart_tick_bars" => json!({"ticks":[10.,11.,12.,13.,14.,15.],"ticks_per_bar":3.}),
        _ => json!({}),
    }
}
pub fn catalog() -> Vec<PracticeConcept> {
    IDS.iter()
        .map(|(id, name, formula, _, boundary)| {
            let d = defaults(id);
            PracticeConcept {
                id: (*id).into(),
                name: (*name).into(),
                category: "原书非标准图表".into(),
                input_kind: "independent_inputs".into(),
                inputs: d
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(key, default)| PracticeInput {
                        key: key.clone(),
                        default: default.clone(),
                        label: (match key.as_str() {
                            "prices" => "有序收盘价格（元）",
                            "ticks" => "有序逐笔成交价（元）",
                            "brick_size" => "砖宽（元）",
                            "box_size" => "箱格（元）",
                            "reversal_boxes" => "反转箱数（格）",
                            "reversal_size" => "反转幅度（元）",
                            "line_count" => "突破线数（条）",
                            "range_size" => "区间宽度（元）",
                            "ticks_per_bar" => "每条成交笔数（笔）",
                            _ => "已收盘 OHLC（元）",
                        })
                        .into(),
                    })
                    .collect(),
                notes: format!("{}；{}", formula, boundary),
            }
        })
        .collect()
}
pub fn entries() -> Vec<KnowledgeEntry> {
    IDS.iter()
        .map(|(id, name, formula, example, boundary)| KnowledgeEntry {
            id: (*id).into(),
            summary: format!("{}：{}", name, boundary),
            example: (*example).into(),
            related: vec![],
            code_url: "https://github.com/Sigma711/axiom/blob/main/src/book_charts.rs".into(),
            code_ref: "src/book_charts.rs::evaluate".into(),
            category: "原书非标准图表".into(),
            name: (*name).into(),
            formula: (*formula).into(),
            meaning: (*boundary).into(),
            signals: "图形压缩了时间或成交路径，须先确认输入粒度。".into(),
            pitfalls: (*boundary).into(),
            implementation: "src/book_charts.rs::evaluate".into(),
            diagram: None,
        })
        .collect()
}
fn positive(v: &Value, k: &str) -> Result<f64, String> {
    let x = num(v, k)?;
    if x > 0. {
        Ok(x)
    } else {
        Err(format!("{k} 必须大于零"))
    }
}
fn positive_integer(v: &Value, k: &str) -> Result<usize, String> {
    let x = positive(v, k)?;
    if x.fract() != 0. || x > 10000. {
        Err(format!("{k} 必须是正整数"))
    } else {
        Ok(x as usize)
    }
}
fn ordered(v: &Value, k: &str) -> Result<Vec<f64>, String> {
    let x = arr(v, k)?;
    if x.is_empty() || x.iter().any(|x| *x <= 0.) {
        Err(format!("{k} 必须是非空正数有序输入"))
    } else {
        Ok(x)
    }
}
fn series(out: &mut Output, name: &str, x: Vec<f64>, unit: &str) {
    out.series(name, x.into_iter().map(Some).collect(), unit)
}
fn chart_bar(open: f64, high: f64, low: f64, close: f64, direction: i8) -> Value {
    json!({"open":open,"high":high,"low":low,"close":close,"direction":direction})
}
fn push_step(bars: &mut Vec<Value>, open: f64, close: f64) {
    bars.push(chart_bar(
        open,
        open.max(close),
        open.min(close),
        close,
        if close >= open { 1 } else { -1 },
    ));
}
fn capped_push(bars: &mut Vec<Value>, open: f64, close: f64) -> Result<(), String> {
    if bars.len() >= 10_000 {
        return Err("生成图形条数超过 10000；请增大阈值或缩短输入".into());
    }
    push_step(bars, open, close);
    Ok(())
}
fn decorate_kagi(bars: &mut [Value]) {
    let (mut style, mut peak, mut trough, mut previous_direction) = ("neutral", None, None, 0_i64);
    for bar in bars {
        let direction = bar["direction"].as_i64().unwrap_or_default();
        let open = bar["open"].as_f64().unwrap_or_default();
        let close = bar["close"].as_f64().unwrap_or_default();
        let mut switch_price = Value::Null;
        if direction > 0 {
            if let Some(level) = peak {
                if open <= level && close > level {
                    style = "yang";
                    switch_price = json!(level);
                }
            }
        } else if let Some(level) = trough {
            if open >= level && close < level {
                style = "yin";
                switch_price = json!(level);
            }
        }
        if previous_direction > 0 && direction < 0 {
            peak = Some(open);
        } else if previous_direction < 0 && direction > 0 {
            trough = Some(open);
        }
        bar["line_style"] = json!(style);
        bar["switch_price"] = switch_price;
        previous_direction = direction;
    }
}
pub fn evaluate(id: &str, _: &[Bar], inputs: &Value) -> Result<Value, String> {
    let mut v = defaults(id);
    if v.as_object().unwrap().is_empty() {
        return Err(format!("未知图表概念: {id}"));
    }
    for (k, x) in inputs.as_object().ok_or("inputs必须是对象")? {
        if v.get(k).is_none() {
            return Err(format!("{id}不支持输入{k}"));
        }
        v[k] = x.clone();
    }
    let mut out = Output::default();
    let chart: Value = match id {
        "book_chart_heikin_ashi" => {
            let (o, h, l, c) = (
                ordered(&v, "open")?,
                ordered(&v, "high")?,
                ordered(&v, "low")?,
                ordered(&v, "close")?,
            );
            if o.len() != h.len() || o.len() != l.len() || o.len() != c.len() {
                return Err("OHLC 长度必须相等".into());
            }
            let (mut opens, mut closes, mut highs, mut lows) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for i in 0..o.len() {
                if h[i] < o[i].max(c[i]) || l[i] > o[i].min(c[i]) {
                    return Err("OHLC 高低价不一致".into());
                }
                let hc = (o[i] + h[i] + l[i] + c[i]) / 4.;
                let ho = if i == 0 {
                    (o[0] + c[0]) / 2.
                } else {
                    (opens[i - 1] + closes[i - 1]) / 2.
                };
                opens.push(ho);
                closes.push(hc);
                highs.push(h[i].max(ho).max(hc));
                lows.push(l[i].min(ho).min(hc));
            }
            let bars = opens
                .iter()
                .zip(&highs)
                .zip(&lows)
                .zip(&closes)
                .map(|(((open, high), low), close)| {
                    chart_bar(
                        *open,
                        *high,
                        *low,
                        *close,
                        if close >= open { 1 } else { -1 },
                    )
                })
                .collect::<Vec<_>>();
            series(&mut out, "ha_open", opens, "currency");
            series(&mut out, "ha_close", closes, "currency");
            series(&mut out, "ha_high", highs, "currency");
            series(&mut out, "ha_low", lows, "currency");
            json!({"kind":"heikin_ashi","bars":bars,"input":"explicit_ohlc"})
        }
        "book_chart_renko" => {
            let p = ordered(&v, "prices")?;
            let b = positive(&v, "brick_size")?;
            let mut x = vec![p[0]];
            let mut last = p[0];
            let mut dir = 0.0_f64;
            let mut bars = Vec::new();
            for q in p.into_iter().skip(1) {
                let delta = q - last;
                if dir == 0.0 && delta.abs() >= b {
                    dir = delta.signum();
                }
                let same = dir * delta >= b;
                let reverse = dir * delta <= -2.0 * b;
                if same || reverse {
                    if reverse {
                        dir = -dir;
                        let open = last + dir * b;
                        last += dir * 2.0 * b;
                        capped_push(&mut bars, open, last)?;
                        x.push(last);
                    }
                    while dir * (q - last) >= b {
                        let open = last;
                        last += dir * b;
                        capped_push(&mut bars, open, last)?;
                        x.push(last);
                    }
                }
            }
            series(&mut out, "renko_close", x, "currency");
            json!({"kind":"renko","bars":bars,"input":"explicit_close_only","note":"砖是由阈值合成的图形价格，不是逐笔成交；每块固定一砖宽。"})
        }
        "book_chart_point_figure" => {
            let p = ordered(&v, "prices")?;
            let b = positive(&v, "box_size")?;
            let r = positive_integer(&v, "reversal_boxes")? as f64;
            let mut x = vec![p[0]];
            let (mut last, mut dir) = (p[0], 0.);
            let mut bars = Vec::new();
            for q in p.into_iter().skip(1) {
                let d = q - last;
                if dir == 0. && d.abs() >= b {
                    dir = d.signum()
                }
                if dir * d >= b || dir * d <= -b * r {
                    if dir * d <= -b * r {
                        dir = -dir
                    }
                    while dir * (q - last) >= b {
                        let open = last;
                        last += dir * b;
                        capped_push(&mut bars, open, last)?;
                        x.push(last)
                    }
                }
            }
            series(&mut out, "point_figure_box", x, "currency");
            json!({"kind":"point_figure","bars":bars,"input":"explicit_close_only","reversal_boxes":r})
        }
        "book_chart_kagi" => {
            let p = ordered(&v, "prices")?;
            let r = positive(&v, "reversal_size")?;
            let mut x = vec![p[0]];
            let (mut last, mut dir) = (p[0], 0.);
            let mut bars = Vec::new();
            for q in p.into_iter().skip(1) {
                let d = q - last;
                if dir == 0. {
                    if d.abs() < r {
                        continue;
                    }
                    dir = d.signum();
                    capped_push(&mut bars, last, q)?;
                    last = q;
                    x.push(last);
                    continue;
                }
                if dir * d >= 0. {
                    capped_push(&mut bars, last, q)?;
                    last = q;
                    x.push(last);
                } else if dir * d <= -r {
                    dir = -dir;
                    capped_push(&mut bars, last, q)?;
                    last = q;
                    x.push(last);
                }
            }
            series(&mut out, "kagi_turn", x, "currency");
            decorate_kagi(&mut bars);
            json!({"kind":"kagi","bars":bars,"input":"explicit_close_only","reversal_size":r})
        }
        "book_chart_three_line_break" => {
            let p = ordered(&v, "prices")?;
            let n = positive_integer(&v, "line_count")?;
            let mut x = vec![p[0]];
            let mut bars = Vec::new();
            let mut direction = 0_i8;
            for q in p.into_iter().skip(1) {
                let last = *x.last().unwrap();
                if direction == 0 {
                    if q == last {
                        continue;
                    }
                    direction = if q > last { 1 } else { -1 };
                    capped_push(&mut bars, last, q)?;
                    x.push(q);
                    continue;
                }
                let recent = &bars[bars.len().saturating_sub(n)..];
                let high = recent
                    .iter()
                    .filter_map(|b| b["high"].as_f64())
                    .fold(f64::NEG_INFINITY, f64::max);
                let low = recent
                    .iter()
                    .filter_map(|b| b["low"].as_f64())
                    .fold(f64::INFINITY, f64::min);
                let continuation = (direction > 0 && q > last) || (direction < 0 && q < last);
                let reversal = (direction > 0 && q < low) || (direction < 0 && q > high);
                if continuation || reversal {
                    if reversal {
                        direction = -direction;
                    }
                    capped_push(&mut bars, last, q)?;
                    x.push(q);
                }
            }
            series(&mut out, "three_line_close", x, "currency");
            json!({"kind":"three_line_break","bars":bars,"input":"explicit_close_only","line_count":n})
        }
        "book_chart_range_bars" => {
            let p = ordered(&v, "ticks")?;
            let r = positive(&v, "range_size")?;
            let mut x = Vec::new();
            let (mut open, mut lo, mut hi) = (p[0], p[0], p[0]);
            let mut bars = Vec::new();
            for q in p.into_iter().skip(1) {
                lo = lo.min(q);
                hi = hi.max(q);
                if hi - lo >= r {
                    x.push(q);
                    bars.push(chart_bar(open, hi, lo, q, if q >= open { 1 } else { -1 }));
                    open = q;
                    lo = q;
                    hi = q
                }
            }
            series(&mut out, "range_close", x, "currency");
            json!({"kind":"range_bars","bars":bars,"input":"explicit_ordered_ticks","range_size":r,"note":"仅在已提供成交使范围达到阈值时完成；跳空不会虚构中间成交。"})
        }
        "book_chart_tick_bars" => {
            let p = ordered(&v, "ticks")?;
            let n = positive_integer(&v, "ticks_per_bar")?;
            let completed: Vec<_> = p.chunks(n).filter(|c| c.len() == n).collect();
            let x = completed.iter().map(|c| *c.last().unwrap()).collect();
            series(&mut out, "tick_close", x, "currency");
            let bars = completed
                .into_iter()
                .map(|c| {
                    let open = c[0];
                    let close = *c.last().unwrap();
                    chart_bar(
                        open,
                        c.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        c.iter().copied().fold(f64::INFINITY, f64::min),
                        close,
                        if close >= open { 1 } else { -1 },
                    )
                })
                .collect::<Vec<_>>();
            json!({"kind":"tick_bars","bars":bars,"input":"explicit_ordered_ticks","ticks_per_bar":n})
        }
        _ => return Err(format!("未知图表概念: {id}")),
    };
    out.note("仅使用调用方提供的有序输入；不使用未来数据或回填历史。");
    Ok(
        json!({"concept_id":id,"input_kind":"independent_inputs","provenance":"editable_teaching_inputs","status":"computed","reason":null,"values":out.values,"units":out.units,"series":out.series,"chart":chart,"notes":out.reasons,"inputs":v}),
    )
}
