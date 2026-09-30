//! Fixed, source-verified A-share free-float and circulating-cap case.
//!
//! The case keeps three different quantities separate: unrestricted shares,
//! methodology-defined free float, and the index's bucketed adjusted shares.

use futures::StreamExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use tokio::sync::Mutex;

pub const ANNUAL_REPORT_URL: &str =
    "https://static.cninfo.com.cn/finalpage/2026-04-17/1225114741.PDF";
pub const ANNUAL_REPORT_SHA256: &str =
    "474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288";
pub const ANNUAL_REPORT_BYTES: usize = 1_082_847;
pub const CONCERT_ANNOUNCEMENT_URL: &str =
    "https://static.cninfo.com.cn/finalpage/2025-12-30/1224906220.PDF";
pub const CONCERT_ANNOUNCEMENT_SHA256: &str =
    "ae056c65f53fcacd0b2cffd3562af0f2696d32593e25543ba279b59b07e57ee6";
pub const CONCERT_ANNOUNCEMENT_BYTES: usize = 92_857;
pub const INDEX_METHODOLOGY_URL: &str = "https://oss-ch.csindex.com.cn/static/html/csindex/public/uploads/indices/detail/files/zh_CN/000300_Index_Methodology_cn.pdf";
pub const INDEX_METHODOLOGY_SHA256: &str =
    "de6491e03e5d57ecf1aca104b1412543643a59e387a5343c9bd21ecbbdeba5b6";
pub const INDEX_METHODOLOGY_BYTES: usize = 1_184_619;
pub const PRICE_ENDPOINT: &str =
    "https://web.ifzq.gtimg.cn/appstock/app/kline/kline?param=sh600519,day,2025-12-31,2025-12-31,1";

const SYMBOL: &str = "600519";
const AS_OF: &str = "2025-12-31";
const UNRESTRICTED_SHARES: u64 = 1_252_270_215;
const CONTROLLING_HOLDER_SHARES: u64 = 681_282_935;
const CONCERT_SUBSIDIARY_SHARES: u64 = 27_849_688;
const NON_FREE_FLOAT_SHARES: u64 = CONTROLLING_HOLDER_SHARES + CONCERT_SUBSIDIARY_SHARES;
const FREE_FLOAT_SHARES: u64 = UNRESTRICTED_SHARES - NON_FREE_FLOAT_SHARES;
const CLOSE_PRICE: f64 = 1377.18;
const CLOSE_PRICE_CENTS: u64 = 137_718;

pub const SUPPORTED_IDS: &[&str] = &["book_float_market_cap", "book_free_float"];
static SOURCE_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedDocumentSource {
    pub url: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AShareFloatSourceRegistry {
    pub annual_report: VerifiedDocumentSource,
    pub concert_party_announcement: VerifiedDocumentSource,
    pub index_methodology: VerifiedDocumentSource,
    pub price_endpoint: String,
}

impl Default for AShareFloatSourceRegistry {
    fn default() -> Self {
        Self {
            annual_report: VerifiedDocumentSource {
                url: ANNUAL_REPORT_URL.into(),
                sha256: ANNUAL_REPORT_SHA256.into(),
                bytes: ANNUAL_REPORT_BYTES,
            },
            concert_party_announcement: VerifiedDocumentSource {
                url: CONCERT_ANNOUNCEMENT_URL.into(),
                sha256: CONCERT_ANNOUNCEMENT_SHA256.into(),
                bytes: CONCERT_ANNOUNCEMENT_BYTES,
            },
            index_methodology: VerifiedDocumentSource {
                url: INDEX_METHODOLOGY_URL.into(),
                sha256: INDEX_METHODOLOGY_SHA256.into(),
                bytes: INDEX_METHODOLOGY_BYTES,
            },
            price_endpoint: PRICE_ENDPOINT.into(),
        }
    }
}

pub fn is_supported(id: &str) -> bool {
    SUPPORTED_IDS.contains(&id)
}

fn free_float_ratio() -> f64 {
    FREE_FLOAT_SHARES as f64 / UNRESTRICTED_SHARES as f64
}

fn circulating_market_cap() -> f64 {
    (UNRESTRICTED_SHARES * CLOSE_PRICE_CENTS) as f64 / 100.0
}

pub fn configure_concept(concept: &mut crate::practice::PracticeConcept) {
    if is_supported(&concept.id) {
        concept.input_kind = "industry_case".into();
        concept.category = "发行人历史披露".into();
        concept.inputs.clear();
        concept.notes = "贵州茅台2025-12-31固定案例：分别核验无限售条件流通股、按中证方法定义的自由流通量，以及同日未复权收盘价；三种口径不混用。".into();
    }
}

pub fn configure_knowledge(entry: &mut crate::knowledge::KnowledgeEntry) {
    match entry.id.as_str() {
        "book_float_market_cap" => {
            entry.formula = "同日流通市值=无限售条件流通股份×未复权收盘价".into();
            entry.example =
                "贵州茅台2025-12-31：1,252,270,215股×1,377.18元=1,724,601,494,693.70元。".into();
            entry.pitfalls = "这里的流通股是无限售条件流通股份；它不等于剔除战略长期持股后的自由流通量。股数日期与价格交易日必须相同。".into();
        }
        "book_free_float" => {
            entry.formula =
                "自由流通量=总股本−非自由流通股本；自由流通比例=自由流通量/总股本".into();
            entry.example = "贵州茅台2025-12-31：1,252,270,215−681,282,935−27,849,688=543,137,592股，自由流通比例约43.3722%。".into();
            entry.pitfalls = "无限售条件流通股份不等于自由流通股；543,137,592股是按公开股东事实逐项应用中证定义的透明复算值，不冒充中证公司发布的证券级自由流通量。".into();
        }
        _ => return,
    }
    entry.signals = "固定历史教学案例，不是当前行情、任意A股查询、指数权重文件或交易建议。".into();
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

async fn verify_document(
    cache_dir: &Path,
    source: &VerifiedDocumentSource,
) -> Result<Value, String> {
    if source.bytes == 0 || source.bytes > 8_000_000 || source.sha256.len() != 64 {
        return Err("A-share float document expectation is invalid".into());
    }
    let _guard = SOURCE_LOCK.lock().await;
    let directory = cache_dir.join("a-share-float-sources");
    let cached = directory.join(format!("{}.pdf", source.sha256));
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        if bytes.len() == source.bytes && fingerprint(&bytes) == source.sha256 {
            return Ok(
                json!({"status":"verified_immutable_cache","requested_url":source.url,"matched_sha256":source.sha256,"matched_bytes":source.bytes}),
            );
        }
        tokio::fs::remove_file(&cached)
            .await
            .map_err(|error| format!("corrupt A-share float cache removal failed: {error}"))?;
    }
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("A-share float document client failed: {error}"))?
        .get(&source.url)
        .send()
        .await
        .map_err(|error| format!("A-share float document request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("A-share float document returned an error: {error}"))?;
    if response
        .content_length()
        .is_some_and(|size| size != source.bytes as u64)
    {
        return Err("A-share float document Content-Length differs from reviewed bytes".into());
    }
    let mut bytes = Vec::with_capacity(source.bytes);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| format!("A-share float document body failed: {error}"))?;
        if bytes.len().saturating_add(chunk.len()) > source.bytes {
            return Err("A-share float document exceeds reviewed bytes".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != source.bytes || fingerprint(&bytes) != source.sha256 {
        return Err("A-share float document bytes or SHA-256 do not match".into());
    }
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| format!("A-share float cache directory failed: {error}"))?;
    tokio::fs::write(&cached, &bytes)
        .await
        .map_err(|error| format!("A-share float cache write failed: {error}"))?;
    Ok(
        json!({"status":"verified_then_cached","requested_url":source.url,"matched_sha256":source.sha256,"matched_bytes":source.bytes}),
    )
}

async fn fetch_close(endpoint: &str) -> Result<Value, String> {
    let raw: Value = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("A-share price client failed: {error}"))?
        .get(endpoint)
        .send()
        .await
        .map_err(|error| format!("A-share price request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("A-share price returned an error: {error}"))?
        .json()
        .await
        .map_err(|error| format!("invalid A-share price JSON: {error}"))?;
    if raw.pointer("/code").and_then(Value::as_i64) != Some(0) {
        return Err("A-share price response reports an error".into());
    }
    let rows = raw
        .pointer("/data/sh600519/day")
        .and_then(Value::as_array)
        .ok_or("A-share price response has no klines")?;
    if rows.len() != 1 {
        return Err("A-share price response must contain exactly the fixed trading date".into());
    }
    let fields = rows[0]
        .as_array()
        .ok_or("A-share price row must be an array")?;
    if fields.len() < 6 || fields[0] != AS_OF {
        return Err("A-share price row date or shape differs from fixed case".into());
    }
    let close: f64 = fields[2]
        .as_str()
        .ok_or("A-share close must be text")?
        .parse()
        .map_err(|_| "A-share close is invalid")?;
    if close != CLOSE_PRICE {
        return Err("A-share close differs from the independently checked historical value".into());
    }
    Ok(
        json!({"provider":"Tencent public historical kline API","endpoint":endpoint,"trading_date":AS_OF,"close":close,"currency":"CNY","basis":"unadjusted_daily_close","request_adjustment":"none"}),
    )
}

fn validate_request(id: &str, offline: bool) -> Result<(), String> {
    if !is_supported(id) {
        return Err("unsupported A-share float concept".into());
    }
    if offline {
        return Err("A-share float source verification is disabled in offline mode".into());
    }
    Ok(())
}

pub async fn evaluate(
    id: &str,
    cache_dir: &Path,
    sources: &AShareFloatSourceRegistry,
) -> Result<Value, String> {
    validate_request(id, std::env::var("AXIOM_OFFLINE").as_deref() == Ok("1"))?;
    let annual = verify_document(cache_dir, &sources.annual_report).await?;
    let concert = verify_document(cache_dir, &sources.concert_party_announcement).await?;
    let methodology = verify_document(cache_dir, &sources.index_methodology).await?;
    let price = fetch_close(&sources.price_endpoint).await?;
    let free_float_ratio = free_float_ratio();
    let circulating_cap = circulating_market_cap();
    Ok(json!({
        "concept_id":id,"status":"computed","reason":Value::Null,"input_kind":"industry_case",
        "provenance":"verified_independent_a_share_float_case","values":{
            "book_free_float":free_float_ratio,"book_float_market_cap":circulating_cap,
            "unrestricted_shares":UNRESTRICTED_SHARES as f64,"non_free_float_shares":NON_FREE_FLOAT_SHARES as f64,
            "free_float_shares":FREE_FLOAT_SHARES as f64,"close_price":CLOSE_PRICE
        },
        "units":{"book_free_float":"fraction","book_float_market_cap":"CNY","unrestricted_shares":"shares","non_free_float_shares":"shares","free_float_shares":"shares","close_price":"CNY/share"},
        "series":[],"bars":[],"inputs":{},
        "float_case":{
            "case_id":"moutai_2025_12_31_free_float_and_circulating_cap","issuer":{"name":"贵州茅台酒股份有限公司","ticker":SYMBOL},
            "as_of":AS_OF,"share_register_date":AS_OF,"published":"2026-04-17",
            "definition":{"provider":"China Securities Index Co., Ltd.","rule":"total shares minus restricted shares and strategic or otherwise basically non-circulating holdings","threshold":"5% including concert parties","calculation_scope":"transparent reproduction from cited public holder facts, not a CSI-published security-level estimate"},
            "non_free_float_holders":[
                {"name":"中国贵州茅台酒厂（集团）有限责任公司","shares":CONTROLLING_HOLDER_SHARES as f64,"classification":"state_owned_controlling_holder","relationship":"controller"},
                {"name":"贵州茅台酒厂（集团）技术开发有限公司","shares":CONCERT_SUBSIDIARY_SHARES as f64,"classification":"concert_party_of_controlling_holder","relationship":"wholly_owned_subsidiary_and_concert_party"}
            ],
            "price":price,
            "sources":[
                {"kind":"issuer_annual_report","official_url":sources.annual_report.url,"pdf_pages":[47,48,49],"sha256":sources.annual_report.sha256,"bytes":sources.annual_report.bytes,"verification":annual},
                {"kind":"issuer_concert_party_announcement","official_url":sources.concert_party_announcement.url,"pdf_pages":[1,2,3],"sha256":sources.concert_party_announcement.sha256,"bytes":sources.concert_party_announcement.bytes,"verification":concert},
                {"kind":"official_index_methodology","official_url":sources.index_methodology.url,"pdf_pages":[5,6],"sha256":sources.index_methodology.sha256,"bytes":sources.index_methodology.bytes,"verification":methodology},
                {"kind":"historical_price_api","official_url":sources.price_endpoint,"trading_date":AS_OF,"basis":"unadjusted_daily_close"}
            ]
        },
        "notes":[
            "无限售条件流通股份不等于自由流通股：前者为1,252,270,215股；按所列中证定义与一致行动关系透明复算的自由流通量为543,137,592股。",
            "自由流通量是依据公开规则与具名持股事实的复算值，不冒充中证公司发布的证券级自由流通量或指数调整股本。",
            "流通市值只用同为2025-12-31的无限售条件流通股份与未复权日收盘价；年报于2026-04-17发布，因此不代表该交易日当时可得信息。"
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_case_reconciles_exact_share_counts() {
        assert_eq!(NON_FREE_FLOAT_SHARES, 709_132_623);
        assert_eq!(FREE_FLOAT_SHARES, 543_137_592);
        assert!(
            (FREE_FLOAT_SHARES as f64 / UNRESTRICTED_SHARES as f64 - 0.4337223591954553).abs()
                < 1e-15
        );
        assert_eq!(
            (UNRESTRICTED_SHARES * CLOSE_PRICE_CENTS) as f64 / 100.0,
            1_724_601_494_693.70
        );
    }

    #[test]
    fn catalog_copy_states_the_two_distinct_denominators() {
        let mut entries = crate::book::entries();
        let cap = entries
            .iter_mut()
            .find(|entry| entry.id == "book_float_market_cap")
            .unwrap();
        configure_knowledge(cap);
        assert!(cap.formula.contains("无限售条件流通股份"));
        assert!(cap.pitfalls.contains("不等于"));

        let free = entries
            .iter_mut()
            .find(|entry| entry.id == "book_free_float")
            .unwrap();
        configure_knowledge(free);
        assert!(free.formula.contains("非自由流通股本"));
        assert!(free.pitfalls.contains("不冒充"));

        let mut unrelated = entries
            .into_iter()
            .find(|entry| entry.id != "book_free_float" && entry.id != "book_float_market_cap")
            .unwrap();
        let original = unrelated.formula.clone();
        configure_knowledge(&mut unrelated);
        assert_eq!(unrelated.formula, original);
    }

    #[tokio::test]
    async fn rejects_invalid_source_expectations_before_network_access() {
        let source = VerifiedDocumentSource {
            url: "https://example.invalid/source.pdf".into(),
            sha256: "invalid".into(),
            bytes: 0,
        };
        let error = verify_document(Path::new("."), &source).await.unwrap_err();
        assert!(error.contains("expectation is invalid"));
    }

    #[test]
    fn rejects_unsupported_and_offline_requests_before_source_io() {
        assert_eq!(
            validate_request("unsupported", false).unwrap_err(),
            "unsupported A-share float concept"
        );
        assert!(validate_request("book_free_float", true)
            .unwrap_err()
            .contains("disabled in offline mode"));
        assert!(validate_request("book_float_market_cap", false).is_ok());
    }
}
