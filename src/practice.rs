//! Executable concept registry. Independent inputs are editable teaching fixtures,
//! never inferred from the selected crypto candles. All time series are causal:
//! historical annotations are emitted at their confirmation time.
use crate::types::Bar;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
mod independent;
mod market;
#[derive(Debug, Clone, Serialize)]
pub struct PracticeInput {
    pub key: String,
    pub label: String,
    pub default: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct PracticeConcept {
    pub id: String,
    pub name: String,
    pub category: String,
    pub input_kind: String,
    pub inputs: Vec<PracticeInput>,
    pub notes: String,
}
pub fn base_catalog() -> Vec<PracticeConcept> {
    let mut seen = BTreeSet::new();
    crate::knowledge::base_entries().into_iter().filter(|e|seen.insert(e.id.clone())).map(|e|{
  let d=defaults(&e.id).expect("every knowledge concept has an explicit practice definition");
  let kind=if e.id=="bid_ask_spread" { "market_bars" } else { kind(&e.id) };
  let is_binance_aggressor_practice=matches!(e.id.as_str(), "inside_outside" | "cvd");
  let hides_rolling_return_aliases = e.id == "rolling_correlation";
  let is_depth_snapshot=e.id=="bid_ask_spread";
  PracticeConcept{id:e.id,name:e.name,category:e.category,input_kind:kind.into(),inputs:if is_depth_snapshot{Vec::new()}else{d.as_object().unwrap().iter().filter(|(k, _)| !(hides_rolling_return_aliases && matches!(k.as_str(), "series_x" | "series_y"))).map(|(k,v)|PracticeInput{key:k.clone(),label:k.clone(),default:v.clone()}).collect()},notes:if is_depth_snapshot{"仅使用服务器从 Binance USDT 现货深度端点取得的单次盘口快照；不接受手填报价、客户端深度或K线。"}else if is_binance_aggressor_practice{ "仅使用服务器从 Binance USDT 现货取得的最近24根连续已收盘1小时K线。按 taker-buy 与总量互补计算主动买卖，非A股内外盘或资本净流入；不接受手填或客户端K线。" }else if hides_rolling_return_aliases{"仅使用本页真实策略净值收益与同时间戳标的收盘收益；只可修改窗口，不提供教学收益数组。"}else if kind=="market_bars"{"仅使用传入、按时间排序的已收盘 K 线；回测需传入截至评估时点的前缀。预热/零分母为 null。"}else{"独立可编辑教学输入，不是当前币种真实数据；历史使用须由调用方保证输入发布时点不晚于评估时点。"}.into()}
 }).collect()
}
fn kind(id: &str) -> &'static str {
    if market::is_market(id) {
        "market_bars"
    } else if id == "elliott_wave" {
        "manual_annotation"
    } else {
        "independent_inputs"
    }
}
fn defaults(id: &str) -> Option<Value> {
    if market::is_market(id) {
        Some(market::defaults(id))
    } else {
        independent::defaults(id)
    }
}
fn evaluate_base(id: &str, bars: &[Bar], inputs: &Value) -> Result<Value, String> {
    let mut merged = defaults(id).ok_or_else(|| format!("未知概念: {id}"))?;
    let provided = inputs.as_object().ok_or("inputs 必须为 JSON 对象")?;
    for (k, v) in provided {
        if !merged.as_object().unwrap().contains_key(k) {
            return Err(format!("{id} 不支持输入 {k}"));
        }
        let expected = &merged[k];
        let valid = base_input_matches(v, expected);
        if !valid {
            return Err(format!("{k} 的输入类型或数值无效"));
        }
        merged[k] = v.clone();
    }
    let mut out = Output::default();
    let input_kind = kind(id);
    if input_kind == "market_bars" {
        validate_bars(bars)?;
        if bars.is_empty() {
            out.value(id, None, "", "没有可评估的已收盘 K 线");
        } else {
            market::evaluate(id, bars, &merged, &mut out)?;
        }
    } else {
        independent::evaluate(id, &merged, &mut out)?;
    }
    let has_value = out.values.values().any(|v| !v.is_null());
    let reason = if has_value {
        Value::Null
    } else {
        json!(out.reasons.join("；"))
    };
    Ok(
        json!({"concept_id":id,"input_kind":input_kind,"provenance":if input_kind=="market_bars"{"provided_market_bars"}else{"editable_teaching_inputs"},"status":if has_value{"computed"}else{"undefined"},"reason":reason,"values":out.values,"units":out.units,"series":out.series,"notes":out.reasons,"inputs":merged}),
    )
}

fn base_input_matches(value: &Value, expected: &Value) -> bool {
    if expected.is_number() {
        value.as_f64().is_some_and(|n| n.is_finite())
    } else if expected.is_boolean() {
        value.is_boolean()
    } else if expected.is_array() {
        value
            .as_array()
            .is_some_and(|a| a.iter().all(|x| x.as_f64().is_some_and(|n| n.is_finite())))
    } else {
        false
    }
}
pub(crate) fn validate_bars(b: &[Bar]) -> Result<(), String> {
    for (i, x) in b.iter().enumerate() {
        if [x.open, x.high, x.low, x.close, x.volume]
            .iter()
            .any(|v| !v.is_finite())
            || x.open <= 0.0
            || x.low <= 0.0
            || x.close <= 0.0
            || x.high < x.open.max(x.close)
            || x.low > x.open.min(x.close)
            || x.volume < 0.0
        {
            return Err(format!("第 {i} 根 K 线不是有效 OHLCV"));
        }
        if i > 0 && x.timestamp <= b[i - 1].timestamp {
            return Err("K 线必须严格按时间递增且无重复".into());
        }
    }
    Ok(())
}
#[derive(Default)]
pub(crate) struct Output {
    pub(crate) values: Map<String, Value>,
    pub(crate) units: Map<String, Value>,
    pub(crate) series: Vec<Value>,
    pub(crate) reasons: Vec<String>,
}
impl Output {
    pub(crate) fn value(&mut self, k: &str, v: Option<f64>, unit: &str, reason: &str) {
        let v = v.filter(|v| v.is_finite());
        self.values.insert(k.into(), json!(v));
        self.units.insert(k.into(), json!(unit));
        if v.is_none() && !reason.is_empty() {
            self.note(reason);
        }
    }
    pub(crate) fn number(&mut self, k: &str, v: f64, unit: &str) {
        self.value(k, Some(v), unit, "输入不足、零分母或计算不适用");
    }
    pub(crate) fn flag(&mut self, k: &str, v: bool) {
        self.number(k, if v { 1.0 } else { 0.0 }, "boolean (1/0)");
    }
    pub(crate) fn series(&mut self, k: &str, v: Vec<Option<f64>>, unit: &str) {
        let v: Vec<_> = v.into_iter().map(|v| v.filter(|x| x.is_finite())).collect();
        self.value(
            k,
            v.last().copied().flatten(),
            unit,
            "预热不足或输入无定义；null 不等于零",
        );
        self.series.push(json!({"name":k,"unit":unit,"values":v}));
    }
    pub(crate) fn note(&mut self, s: &str) {
        if !self.reasons.iter().any(|x| x == s) {
            self.reasons.push(s.into());
        }
    }
}
pub(crate) fn num(v: &Value, k: &str) -> Result<f64, String> {
    v[k].as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("{k} 必须是有限数值"))
}
pub(crate) fn arr(v: &Value, k: &str) -> Result<Vec<f64>, String> {
    v[k].as_array()
        .ok_or_else(|| format!("{k} 必须是数值数组"))?
        .iter()
        .map(|x| {
            x.as_f64()
                .filter(|x| x.is_finite())
                .ok_or_else(|| format!("{k} 含非数值"))
        })
        .collect()
}
pub(crate) fn boolean(v: &Value, k: &str) -> Result<bool, String> {
    v[k].as_bool().ok_or_else(|| format!("{k} 必须是布尔值"))
}
pub(crate) fn period(v: &Value, k: &str) -> Result<usize, String> {
    let x = num(v, k)?;
    if !(1.0..=10000.0).contains(&x) || x.fract() != 0.0 {
        return Err(format!("{k} 必须是 1..10000 的整数"));
    }
    Ok(x as usize)
}
pub(crate) fn div(a: f64, b: f64) -> Option<f64> {
    if b.abs() > 1e-12 {
        Some(a / b).filter(|v| v.is_finite())
    } else {
        None
    }
}
pub(crate) fn mean(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        None
    } else {
        Some(v.iter().sum::<f64>() / v.len() as f64)
    }
}
pub(crate) fn stddev(v: &[f64]) -> Option<f64> {
    if v.len() < 2 {
        return None;
    }
    let m = mean(v)?;
    Some((v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt())
}

#[derive(Clone, Copy)]
enum Owner {
    Base,
    Book,
    Technical,
    Charts,
    Supplement,
    Workflows,
}
struct Registry {
    concepts: Vec<PracticeConcept>,
    routes: std::collections::BTreeMap<String, (usize, Owner)>,
}
fn registry() -> &'static Registry {
    static REGISTRY: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut concepts = Vec::new();
        let mut routes = std::collections::BTreeMap::new();
        let labels: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../docs/book/input-labels.json"))
                .expect("valid input labels");
        for (owner, items) in [
            (Owner::Base, base_catalog()),
            (Owner::Book, crate::book::catalog()),
            (Owner::Technical, crate::book_technical::catalog()),
            (Owner::Charts, crate::book_charts::catalog()),
            (Owner::Supplement, crate::supplement::catalog()),
            (Owner::Workflows, crate::workflows::catalog()),
        ] {
            for mut item in items {
                for input in &mut item.inputs {
                    if !input.label.chars().any(|c| ('一'..='鿿').contains(&c)) {
                        if let Some(label) = labels.get(&input.key) {
                            input.label.clone_from(label);
                        }
                    }
                    if input.key == "equity" && input.default.is_array() {
                        input.label = "净值序列（按时间顺序，含交易前初始资金）".into();
                    }
                }
                assert!(
                    !routes.contains_key(&item.id),
                    "duplicate practice id {}",
                    item.id
                );
                routes.insert(item.id.clone(), (concepts.len(), owner));
                concepts.push(item);
            }
        }
        Registry { concepts, routes }
    })
}
/// Unified catalog. Registration happens once per process; evaluation is a map lookup.
pub fn catalog() -> Vec<PracticeConcept> {
    registry().concepts.clone()
}
pub fn evaluate(id: &str, bars: &[Bar], inputs: &Value) -> Result<Value, String> {
    evaluate_inner(id, bars, inputs, None)
}
pub fn evaluate_with_annualization(
    id: &str,
    bars: &[Bar],
    inputs: &Value,
    annualization: crate::book_technical::AnnualizationBasis,
) -> Result<Value, String> {
    evaluate_inner(id, bars, inputs, Some(annualization))
}
fn evaluate_inner(
    id: &str,
    bars: &[Bar],
    inputs: &Value,
    annualization: Option<crate::book_technical::AnnualizationBasis>,
) -> Result<Value, String> {
    let reg = registry();
    let (index, owner) = reg
        .routes
        .get(id)
        .ok_or_else(|| format!("未知概念: {id}"))?;
    if matches!(owner, Owner::Base) {
        return evaluate_base(id, bars, inputs);
    }
    let concept = &reg.concepts[*index];
    let mut merged = serde_json::Map::new();
    for field in &concept.inputs {
        merged.insert(field.key.clone(), field.default.clone());
    }
    for (k, v) in inputs.as_object().ok_or("inputs必须是JSON对象")? {
        let expected = merged.get(k).ok_or_else(|| format!("{id}不支持输入{k}"))?;
        let valid = input_type_matches(v, expected);
        if !valid {
            return Err(format!("{k}输入类型或数值无效"));
        }
        merged.insert(k.clone(), v.clone());
    }
    let merged = Value::Object(merged);
    let mut result = match owner {
        Owner::Book => crate::book::evaluate(id, bars, &merged),
        Owner::Technical => match annualization {
            Some(annualization) => {
                crate::book_technical::evaluate_with_annualization(id, bars, &merged, annualization)
            }
            None => crate::book_technical::evaluate(id, bars, &merged),
        },
        Owner::Charts => crate::book_charts::evaluate(id, bars, &merged),
        Owner::Supplement => crate::supplement::evaluate(id, bars, &merged),
        Owner::Workflows => crate::workflows::evaluate(id, bars, &merged),
        Owner::Base => unreachable!(),
    }?;
    if result["provenance"] == "explicit_inputs" {
        result["provenance"] = json!("editable_teaching_inputs");
    }
    Ok(result)
}

fn input_type_matches(value: &Value, example: &Value) -> bool {
    match example {
        Value::Number(_) => value.as_f64().is_some_and(f64::is_finite),
        Value::Bool(_) => value.is_boolean(),
        Value::String(_) => value.is_string(),
        Value::Array(a) => value.as_array().is_some_and(|v| {
            v.iter()
                .all(|x| a.first().is_none_or(|e| input_type_matches(x, e)))
        }),
        Value::Object(fields) => value.as_object().is_some_and(|v| {
            v.iter()
                .all(|(k, x)| fields.get(k).is_some_and(|e| input_type_matches(x, e)))
        }),
        Value::Null => value.is_null(),
    }
}

#[cfg(test)]
mod tests {
    use super::{base_input_matches, input_type_matches, stddev};
    use serde_json::{json, Value};

    #[test]
    fn input_type_guards_cover_supported_and_unsupported_shapes() {
        assert!(base_input_matches(&json!(1.0), &json!(0.0)));
        assert!(base_input_matches(&json!(true), &json!(false)));
        assert!(base_input_matches(&json!([1.0, 2.0]), &json!([0.0])));
        assert!(!base_input_matches(&json!("x"), &json!(null)));

        assert!(input_type_matches(&json!(1.0), &json!(0.0)));
        assert!(input_type_matches(&json!(true), &json!(false)));
        assert!(input_type_matches(&json!("x"), &json!("example")));
        assert!(input_type_matches(&json!([1.0, 2.0]), &json!([0.0])));
        assert!(input_type_matches(
            &json!({"nested": [true]}),
            &json!({"nested": [false]})
        ));
        assert!(input_type_matches(&Value::Null, &Value::Null));
        assert!(stddev(&[1.0]).is_none());
    }
}
