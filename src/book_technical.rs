//! Source-mapped executable technical exercises for chapters 12–25, 28 and appendix A.
use crate::indicators::{
    ma, momentum as mom, options, statistics as stats, trend, volatility as vol, volume,
};
use crate::{
    knowledge::KnowledgeEntry,
    practice::{
        arr, boolean, div, mean, num, period, stddev, validate_bars, Output, PracticeConcept,
        PracticeInput,
    },
    types::Bar,
};
use serde_json::{json, Value};
mod external;
mod market;
fn specs() -> &'static [Value] {
    static SPECS: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    SPECS.get_or_init(|| {
        serde_json::from_str::<Value>(include_str!("../docs/book/technical-map.json"))
            .expect("valid embedded technical registry")["concepts"]
            .as_array()
            .expect("concept array")
            .clone()
    })
}
pub fn catalog() -> Vec<PracticeConcept> {
    specs()
        .iter()
        .map(|s| PracticeConcept {
            id: s["id"].as_str().unwrap().into(),
            name: s["name"].as_str().unwrap().into(),
            category: "原书补充·技术实践".into(),
            input_kind: s["input_kind"].as_str().unwrap().into(),
            inputs: s["defaults"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| PracticeInput {
                    key: k.clone(),
                    label: k.clone(),
                    default: v.clone(),
                })
                .collect(),
            notes: s["summary"].as_str().unwrap().into(),
        })
        .collect()
}
pub fn entries() -> Vec<KnowledgeEntry> {
    specs().iter().map(|s|{let text=|k:&str|s[k].as_str().unwrap_or("").to_string();KnowledgeEntry{id:text("id"),summary:text("summary"),example:text("example"),related:vec![],code_url:"https://github.com/Sigma711/axiom/blob/main/src/book_technical.rs#symbol-evaluate".into(),code_ref:"src/book_technical.rs::evaluate".into(),category:"原书补充·技术实践".into(),name:text("name"),formula:if text("formula").is_empty(){text("summary")}else{text("formula")},meaning:text("summary"),signals:"四页同一计算：探索观察、历史时点评估、模拟盘观察、统一输入对比；结果不自动等于交易指令。".into(),pitfalls:"独立输入为可编辑教学数据；市场序列只在确认时点可用。专有指标仅核验用户导入信号，不声称复制未公开算法。".into(),implementation:format!("src/book_technical.rs::evaluate; 来源 {}",s["source_ids"]),diagram:None}}).collect()
}
pub fn evaluate(id: &str, bars: &[Bar], inputs: &Value) -> Result<Value, String> {
    let s = specs()
        .iter()
        .find(|s| s["id"] == id)
        .ok_or_else(|| format!("未知技术概念: {id}"))?;
    let mut v = s["defaults"].clone();
    let provided = inputs.as_object().ok_or("inputs必须为对象")?;
    for (k, x) in provided {
        let d = v.get(k).ok_or_else(|| format!("{id}不支持参数{k}"))?;
        let ok = if d.is_number() {
            x.as_f64().is_some_and(f64::is_finite)
        } else if d.is_boolean() {
            x.is_boolean()
        } else if d.is_array() {
            x.as_array()
                .is_some_and(|a| a.iter().all(|x| x.as_f64().is_some_and(f64::is_finite)))
        } else {
            false
        };
        if !ok {
            return Err(format!("{k}输入类型错误"));
        }
        v[k] = x.clone();
    }
    let kind = s["input_kind"].as_str().unwrap();
    let mut out = Output::default();
    if kind == "market_bars" {
        validate_bars(bars)?;
        if bars.is_empty() {
            out.value(id, None, "", "没有已收盘K线");
        } else {
            market::evaluate(id.strip_prefix("book_").unwrap_or(id), bars, &v, &mut out)?;
        }
    } else {
        external::evaluate(id.strip_prefix("book_").unwrap_or(id), &v, &mut out)?;
    }
    out.note(s["summary"].as_str().unwrap());
    let computed = out.values.values().any(|x| !x.is_null());
    Ok(
        json!({"concept_id":id,"input_kind":kind,"status":if computed{"computed"}else{"undefined"},"reason":if computed{Value::Null}else{json!(out.reasons.join("；"))},"provenance":if kind=="market_bars"{"provided_market_bars"}else{"editable_teaching_inputs"},"values":out.values,"units":out.units,"series":out.series,"notes":out.reasons,"inputs":v,"source_ids":s["source_ids"]}),
    )
}
fn pair(a: &[f64], b: &[f64]) -> Result<(), String> {
    if a.len() != b.len() || a.is_empty() {
        Err("配对数组须非空、等长、同频同日".into())
    } else {
        Ok(())
    }
}
fn positive(v: &Value, k: &str) -> Result<f64, String> {
    let x = num(v, k)?;
    if x <= 0.0 {
        return Err(format!("{k}须为正"));
    }
    Ok(x)
}
fn nonnegative(v: &Value, k: &str) -> Result<f64, String> {
    let x = num(v, k)?;
    if x < 0.0 {
        return Err(format!("{k}不能为负"));
    }
    Ok(x)
}
fn index(v: &Value, k: &str) -> Result<usize, String> {
    let x = nonnegative(v, k)?;
    if x.fract() != 0.0 || x > 1e8 {
        return Err(format!("{k}须为有限整数索引"));
    }
    Ok(x as usize)
}
fn periods(v: &Value, k: &str) -> Result<Vec<usize>, String> {
    arr(v, k)?
        .into_iter()
        .map(|x| {
            if (1.0..=10000.0).contains(&x) && x.fract() == 0.0 {
                Ok(x as usize)
            } else {
                Err(format!("{k}须为正整数周期数组"))
            }
        })
        .collect()
}
fn smooth(v: &[Option<f64>], p: usize) -> Vec<Option<f64>> {
    ma::optional_smooth(v, p, ma::ema)
}
fn rolling<F: Fn(&[f64]) -> Option<f64>>(v: &[f64], p: usize, f: F) -> Vec<Option<f64>> {
    (0..v.len())
        .map(|i| {
            if i + 1 < p {
                None
            } else {
                f(&v[i + 1 - p..=i])
            }
        })
        .collect()
}
fn ranks(x: &[f64]) -> Vec<f64> {
    x.iter()
        .map(|v| {
            let lower = x.iter().filter(|z| **z < *v).count();
            let equal = x.iter().filter(|z| **z == *v).count();
            lower as f64 + (equal as f64 + 1.0) / 2.0
        })
        .collect()
}
