//! Auditable mapping of the user-provided book to executable knowledge concepts.
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::OnceLock};

pub fn records() -> &'static [Value] {
    static RECORDS: OnceLock<Vec<Value>> = OnceLock::new();
    RECORDS.get_or_init(||{
  let manifest:Value=serde_json::from_str(include_str!("../docs/book/source-manifest.json")).expect("book manifest");
  let foundation:Value=serde_json::from_str(include_str!("../docs/book/coverage.json")).expect("foundation map");
  let technical:Value=serde_json::from_str(include_str!("../docs/book/technical-map.json")).expect("technical map");
  let supplement:Value=serde_json::from_str(include_str!("../docs/book/supplement.json")).expect("supplement map");
  let checklist:Value=serde_json::from_str(include_str!("../docs/book/tradingview-map.json")).expect("checklist map");
  let mut mappings:BTreeMap<String,Vec<String>>=BTreeMap::new();
  for r in foundation["records"].as_array().unwrap(){if let Some(id)=r["knowledge_id"].as_str(){mappings.insert(r["source_id"].as_str().unwrap().into(),vec![id.into()]);}}
  for r in technical["source_mappings"].as_array().unwrap(){mappings.insert(r["source_id"].as_str().unwrap().into(),r["concept_ids"].as_array().unwrap().iter().map(|v|v.as_str().unwrap().into()).collect());}
  for r in supplement["concepts"].as_array().unwrap(){for id in r["source_ids"].as_array().unwrap(){mappings.insert(id.as_str().unwrap().into(),vec![r["id"].as_str().unwrap().into()]);}}
  mappings.insert("appendix_053".into(),vec!["book_dividend_yield".into()]);
  mappings.insert("book_04_17".into(),vec!["book_dcf".into(),"book_cape".into(),"target_upside".into()]);
  mappings.insert("book_03_04".into(),std::iter::once("book_nonstandard_bar".into()).chain(crate::book_charts::catalog().into_iter().map(|c|c.id)).collect());
  let mut records=Vec::new();
  for key in ["sections","appendices"]{for r in manifest[key].as_array().unwrap(){let id=r["id"].as_str().unwrap();records.push(json!({"source_id":id,"title":r.get("title").unwrap_or(&r["name"]),"pdf_page":r["pdf_page"],"concept_ids":mappings.get(id).cloned().unwrap_or_default()}));}}
  for r in checklist["mappings"].as_array().unwrap(){let target=r["target_source_id"].as_str().unwrap();records.push(json!({"source_id":r["source_id"],"title":r["name"],"pdf_page":manifest["tradingview_checklist"].as_array().unwrap().iter().find(|source|source["name"]==r["name"]).map(|source|source["pdf_page"].clone()).unwrap_or(Value::Null),"target_source_id":target,"concept_ids":mappings.get(target).cloned().unwrap_or_default()}));}
  for (chapter,page,title,id) in [(26,67,"组合分析完整例子","book_combined_analysis"),(27,68,"不同市场环境与指标搭配","book_market_regimes"),(29,70,"最实用的看盘顺序","book_reading_order"),(36,83,"数据口径、资料来源与使用边界","book_data_conventions")]{records.push(json!({"source_id":format!("chapter_{chapter:02}"),"title":title,"pdf_page":page,"concept_ids":[id]}));}
        let industry: Value = serde_json::from_str(include_str!("../docs/book/industry-map.json")).expect("industry map");
        records.extend(industry["records"].as_array().unwrap().iter().cloned());
  records
 })
}

pub fn for_concept(id: &str) -> Vec<Value> {
    records()
        .iter()
        .filter(|r| r["concept_ids"].as_array().unwrap().iter().any(|v| v == id))
        .map(|r| json!({"source_id":r["source_id"],"title":r["title"],"pdf_page":r["pdf_page"]}))
        .collect()
}

pub fn coverage() -> Value {
    let rows = records();
    json!({"source_title":"股票交易软件专业指标全解_完整版.pdf","sha256":"1f630341266d05c743c1a6d2c8be9058af428ecb65664cfafcfe0eabbe245007","pdf_pages":83,"records":rows,"source_records":rows.len(),"mapped_records":rows.iter().filter(|r|!r["concept_ids"].as_array().unwrap().is_empty()).count(),"note":"章节与附录包含复合概念；清单含别名和重复行，源行数不等于独立知识概念数。已映射只证明有对应知识概念和实现入口，不证明已接入独立真实数据或完成实证练习；每个概念的数据要求和当前来源以 /api/practice 的 plan、实际实践响应为准。"})
}
