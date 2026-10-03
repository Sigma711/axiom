//! Fixed, source-verified valuation and financial-quality cases.
//!
//! Issuer documents are verified byte-for-byte. Historical market/CPI APIs are
//! verified by hashing only the reviewed symbol/date/value observations, so
//! unrelated mutable response metadata cannot silently change the case.

use futures::StreamExt;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::OnceLock, time::Duration};
use tokio::sync::Mutex;

pub const SUPPORTED_IDS: &[&str] = &[
    "altman_z",
    "beneish_m",
    "book_dcf",
    "book_dividend_yield",
    "book_earnings_yield",
    "book_ebitda_margin",
    "book_ev",
    "book_ev_ebit",
    "book_ev_sales",
    "book_fcf_yield",
    "book_intangibles_ratio",
    "book_interest_coverage",
    "book_nav_discount",
    "book_net_debt_ebitda",
    "book_payout_ratio",
    "book_price_cashflow",
    "book_ptbv",
    "book_qoq",
    "ev_ebitda",
    "goodwill_ratio",
    "industry_pe_compare",
    "pb",
    "pe",
    "peg",
    "piotroski",
    "ps",
    "roic",
    "book_cape",
];

static SOURCE_LOCK: Mutex<()> = Mutex::const_new(());
const NUVEEN_PROXY_2025_ARCHIVED_ORIGINAL: &[u8] =
    include_bytes!("../data/verified-sources/nuveen-proxy-2025.pdf");

fn archived_original(source_id: &str) -> Option<&'static [u8]> {
    (source_id == "nuveen_proxy_2025").then_some(NUVEEN_PROXY_2025_ARCHIVED_ORIGINAL)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValuationSourceConfig {
    pub url: String,
    /// Exact document SHA-256, or SHA-256 of canonical reviewed observations.
    pub sha256: String,
    /// Exact document bytes, or canonical reviewed-observation bytes.
    pub bytes: usize,
    pub format: String,
    pub verification_mode: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValuationSourceRegistry {
    pub sources: BTreeMap<String, ValuationSourceConfig>,
}

impl Default for ValuationSourceRegistry {
    fn default() -> Self {
        let root = data().expect("embedded valuation cases must be valid");
        let mut sources = BTreeMap::new();
        for (id, source) in root["sources"].as_object().unwrap() {
            sources.insert(
                id.clone(),
                ValuationSourceConfig {
                    url: source["url"].as_str().unwrap().into(),
                    sha256: source["sha256"].as_str().unwrap().into(),
                    bytes: source["bytes"].as_u64().unwrap() as usize,
                    format: source["format"].as_str().unwrap().into(),
                    verification_mode: source["verification_mode"]
                        .as_str()
                        .unwrap_or("exact_bytes")
                        .into(),
                },
            );
        }
        Self { sources }
    }
}

#[derive(Clone, Copy)]
struct Definition {
    formula: &'static str,
    inputs: &'static [&'static str],
    sources: &'static [&'static str],
    assumptions: &'static [&'static str],
    note: &'static str,
}

pub fn is_supported(id: &str) -> bool {
    SUPPORTED_IDS.contains(&id)
}

pub fn fixed_symbol(id: &str) -> &'static str {
    match id {
        "book_intangibles_ratio" | "book_interest_coverage" | "book_ptbv" | "goodwill_ratio" => {
            "EBAY"
        }
        "book_nav_discount" => "DIAX",
        "industry_pe_compare" => "AAPL",
        _ if is_supported(id) => "AAPL",
        _ => "",
    }
}

pub fn required_datasets(id: &str) -> Vec<&'static str> {
    let Some(def) = definition(id) else {
        return Vec::new();
    };
    let mut datasets = vec!["issuer_financial_disclosures"];
    if def.sources.iter().any(|source| source.ends_with("_price")) {
        datasets.push("nasdaq_historical_close_observations");
    }
    if id == "book_cape" {
        datasets.push("ten_year_diluted_eps_split_adjusted_history");
        datasets.push("fred_cpiaucsl_revised_snapshot_post_revisions");
    }
    if !def.assumptions.is_empty() {
        datasets.push("explicit_model_assumptions");
    }
    datasets
}

pub fn configure_concept(concept: &mut crate::practice::PracticeConcept) {
    if is_supported(&concept.id) {
        concept.input_kind = "industry_case".into();
        concept.inputs.clear();
        concept.notes = "固定历史估值案例：财报原文按字节验证，行情与指数按具名日期记录验证；不代表当前价格、任意股票或实时筛选结果。".into();
    }
}

pub fn configure_knowledge(entry: &mut crate::knowledge::KnowledgeEntry) {
    if let Some(def) = definition(&entry.id) {
        entry.formula = def.formula.into();
        entry.signals = "结果只适用于返回的发行人、基金、报告期和估值日；来源、单位、发布日期及推导假设随结果一并返回。".into();
        entry.pitfalls = def.note.into();
    }
}

fn definition(id: &str) -> Option<Definition> {
    let apple = &["apple_2025"] as &'static [&'static str];
    let apple_price = &["apple_2025", "apple_price"] as &'static [&'static str];
    let none = &[] as &'static [&'static str];
    Some(match id {
        "altman_z" => Definition { formula: "Z=1.2×营运资本/总资产+1.4×留存收益/总资产+3.3×EBIT/总资产+0.6×市值/总负债+营收/总资产", inputs: &["current_assets","current_liabilities","retained_earnings","operating_income","price","shares","liabilities","sales","assets"], sources: apple_price, assumptions: none, note: "采用 Altman 原始上市制造业模型；Apple 不是该模型的理想样本，分数只用于展示因子。" },
        "beneish_m" => Definition { formula: "M=-4.84+0.920DSRI+0.528GMI+0.404AQI+0.892SGI+0.115DEPI-0.172SGAI+4.679TATA-0.327LVGI", inputs: &["receivables","receivables_prev","sales","sales_prev","gross_profit","gross_profit_prev","current_assets","current_assets_prev","ppe","ppe_prev","assets","assets_prev","da","da_prev","sga","sga_prev","current_liabilities","current_liabilities_prev","long_term_debt","long_term_debt_prev","net_income","cfo"], sources: apple, assumptions: none, note: "八个因子全部由两年原始披露重新计算；GMI 用毛利率、LVGI 用流动负债加非流动定期债务口径。M-Score 是筛查工具，不是造假结论。" },
        "book_dcf" => Definition { formula: "股权价值=五年FCFE现值+终值现值；FCFE=CFO-Capex+净举债", inputs: &["cfo","capex","shares"], sources: apple, assumptions: &["dcf_growth","dcf_discount","dcf_terminal_growth","dcf_years","dcf_net_borrowing"], note: "CFO-Capex 仍需加净举债才是完整 FCFE；本教学情景显式假设净举债为0，按股权现金流折现，不再减债加现金。增长率、股权折现率、永续增长率与显式期均是教学假设。" },
        "book_dividend_yield" => Definition { formula: "股息率=每股宣告股利/股价", inputs: &["dividend_per_share","price"], sources: apple_price, assumptions: none, note: "分子是 FY2025 每股宣告额，分母是年报公布后 2025-11-03 收盘价。" },
        "book_earnings_yield" => Definition { formula: "盈利收益率=稀释EPS/股价", inputs: &["eps","price"], sources: apple_price, assumptions: none, note: "使用财报公布后的历史收盘价和 FY2025 稀释 EPS。" },
        "book_ebitda_margin" => Definition { formula: "EBITDA利润率=(营业利润+折旧与摊销)/营收", inputs: &["operating_income","da","sales"], sources: apple, assumptions: none, note: "EBITDA 由 GAAP 营业利润加现金流量表折旧与摊销推导，不冒充发行人报告的非 GAAP 指标。" },
        "book_ev" => Definition { formula: "EV=市值+有息负债-现金及现金等价物", inputs: &["price","shares","debt","cash"], sources: apple_price, assumptions: none, note: "未减去有价证券；结果对现金口径敏感。" },
        "book_ev_ebit" => Definition { formula: "EV/EBIT=(市值+有息负债-现金)/营业利润", inputs: &["price","shares","debt","cash","operating_income"], sources: apple_price, assumptions: none, note: "EBIT 以 GAAP 营业利润代表。" },
        "book_ev_sales" => Definition { formula: "EV/Sales=(市值+有息负债-现金)/营收", inputs: &["price","shares","debt","cash","sales"], sources: apple_price, assumptions: none, note: "市值按估值日收盘价与期末股数近似。" },
        "book_fcf_yield" => Definition { formula: "FCF收益率=(CFO-Capex)/市值", inputs: &["cfo","capex","price","shares"], sources: apple_price, assumptions: none, note: "自由现金流按 CFO 减购置物业厂房设备付款定义。" },
        "book_intangibles_ratio" => Definition { formula: "(商誉+可辨认无形资产)/股东权益", inputs: &["ebay_goodwill","ebay_intangibles","ebay_equity"], sources: &["ebay_2024"], assumptions: none, note: "使用 eBay FY2024 同期资产负债表与附注口径。" },
        "book_interest_coverage" => Definition { formula: "利息保障倍数=营业利润/利息费用", inputs: &["ebay_operating_income","ebay_interest_expense"], sources: &["ebay_2024"], assumptions: none, note: "利息费用不是净利息支出。" },
        "book_nav_discount" => Definition { formula: "NAV折溢价=(市场价格-NAV)/NAV", inputs: &["diax_market_price","diax_nav"], sources: &["nuveen_proxy_2025"], assumptions: none, note: "使用 Nuveen 发行人披露的 DIAX 季内最高收盘价及该交易日对应 NAV；文件只披露所属季度，未披露该笔最高价的具体交易日。" },
        "book_net_debt_ebitda" => Definition { formula: "净债务/EBITDA=(有息负债-现金)/(营业利润+折旧与摊销)", inputs: &["debt","cash","operating_income","da"], sources: apple, assumptions: none, note: "净债务未扣除有价证券。" },
        "book_payout_ratio" => Definition { formula: "现金分红支付率=已支付股利/净利润", inputs: &["dividends","net_income"], sources: apple, assumptions: none, note: "现金流量表的已支付股利与当期宣告额存在时点差。" },
        "book_price_cashflow" => Definition { formula: "P/OCF=市值/CFO；P/FCF=市值/(CFO-Capex)", inputs: &["price","shares","cfo","capex"], sources: apple_price, assumptions: none, note: "同时返回市现率与自由现金流倍数。" },
        "book_ptbv" => Definition { formula: "P/TBV=市值/(股东权益-商誉-可辨认无形资产)", inputs: &["ebay_price","ebay_shares","ebay_equity","ebay_goodwill","ebay_intangibles"], sources: &["ebay_2024","ebay_price"], assumptions: none, note: "eBay 2025-10-01 Nasdaq 收盘价为 87.58 美元；90.75 美元是开盘价，未冒充收盘价。" },
        "book_qoq" => Definition { formula: "Q4环比=(FY2025营收-前九月营收)/Q3营收-1", inputs: &["sales","nine_month_sales","q3_sales"], sources: &["apple_2025","apple_q3_2025"], assumptions: none, note: "Apple 财年 Q4 由全年减前九月推导，再与已披露 Q3 比较。" },
        "ev_ebitda" => Definition { formula: "EV/EBITDA=(市值+有息负债-现金)/(营业利润+折旧与摊销)", inputs: &["price","shares","debt","cash","operating_income","da"], sources: apple_price, assumptions: none, note: "EBITDA 是由已披露数字推导的教学口径。" },
        "goodwill_ratio" => Definition { formula: "(商誉+可辨认无形资产)/股东权益", inputs: &["ebay_goodwill","ebay_intangibles","ebay_equity"], sources: &["ebay_2024"], assumptions: none, note: "比率高不等于必然减值，需结合现金生成单元测试。" },
        "industry_pe_compare" => Definition { formula: "同日硬件同行PE=收盘价/最新已公布财年稀释EPS", inputs: &["peer_apple_price","eps_2025","hpq_price","hpq_eps"], sources: &["apple_2025","hpq_2025","apple_price","hpq_price"], assumptions: none, note: "只比较具名硬件同行 Apple 与 HP Inc.；价格均为 2025-11-26，FY2025 EPS 均已于该日前公布。" },
        "pb" => Definition { formula: "PB=市值/股东权益", inputs: &["price","shares","equity"], sources: apple_price, assumptions: none, note: "市值按年末股数与估值日收盘价近似。" },
        "pe" => Definition { formula: "PE=收盘价/FY2025稀释EPS", inputs: &["price","eps"], sources: apple_price, assumptions: none, note: "负 EPS 时 PE 无经济意义；本案例 EPS 为正。" },
        "peg" => Definition { formula: "PEG=PE/(稀释EPS同比增长率×100)", inputs: &["price","eps","eps_prev"], sources: apple_price, assumptions: none, note: "增长率来自历史年度 EPS，不是分析师前瞻预测。" },
        "piotroski" => Definition { formula: "F-Score=九个盈利、杠杆/流动性与营运二元信号之和", inputs: &["net_income","net_income_prev","cfo","assets","assets_prev","assets_prev2","long_term_debt","long_term_debt_prev","current_assets","current_assets_prev","current_liabilities","current_liabilities_prev","common_shares_issued","gross_profit","gross_profit_prev","sales","sales_prev"], sources: &["apple_2025","apple_2024"], assumptions: none, note: "股本信号直接检查年内普通股发行数，不用被回购净额掩盖的期末股数变化代替；杠杆使用非流动定期债务/平均总资产。" },
        "ps" => Definition { formula: "PS=市值/营业收入", inputs: &["price","shares","sales"], sources: apple_price, assumptions: none, note: "PS 不反映毛利率、资本开支或盈利质量。" },
        "roic" => Definition { formula: "ROIC=EBIT×(1-实际税率)/平均(有息负债+股东权益-现金)", inputs: &["operating_income","income_tax","pretax_income","debt","debt_prev","equity","equity_prev","cash","cash_prev"], sources: apple, assumptions: none, note: "投入资本口径未将有价证券扣除；有效税率来自当期报表。" },
        "book_cape" => Definition { formula: "CAPE=估值日收盘价/过去10年按2025年9月CPI折算的稀释EPS均值", inputs: &["cape_price","eps_2016","eps_2017","eps_2018","eps_2019","eps_2020","eps_2021","eps_2022","eps_2023","eps_2024","eps_2025","cpi_2016","cpi_2017","cpi_2018","cpi_2019","cpi_2020","cpi_2021","cpi_2022","cpi_2023","cpi_2024","cpi_2025"], sources: &["apple_2016","apple_2019","apple_2022","apple_2025","fred_cpi","apple_price"], assumptions: none, note: "2016–2019 EPS 显式按 2020 年 4:1 拆股调整；CPI 是 2026-10-03 检索时的 FRED 后见修订序列，因此这是后见教学案例，不声称修订后 CPI 在 2025-11-21 估值日已知。" },
        _ => return None,
    })
}

fn data() -> Result<&'static Value, String> {
    static DATA: OnceLock<Result<Value, String>> = OnceLock::new();
    match DATA.get_or_init(|| {
        let root: Value = serde_json::from_str(include_str!("../docs/book/valuation_cases.json"))
            .map_err(|e| format!("invalid embedded valuation cases: {e}"))?;
        validate_data(&root)?;
        Ok(root)
    }) {
        Ok(root) => Ok(root),
        Err(error) => Err(error.clone()),
    }
}

fn validate_data(root: &Value) -> Result<(), String> {
    if root["issuer"]["ticker"] != "AAPL" || root["case_id"].as_str().is_none() {
        return Err("invalid valuation case identity".into());
    }
    for (id, source) in root["sources"]
        .as_object()
        .ok_or("missing valuation sources")?
    {
        if source["url"].as_str().is_none()
            || source["sha256"].as_str().is_none_or(|v| v.len() != 64)
            || source["bytes"].as_u64().is_none_or(|v| v == 0)
            || source["format"].as_str().is_none()
        {
            return Err(format!("invalid valuation source: {id}"));
        }
    }
    for id in SUPPORTED_IDS {
        let def = definition(id).ok_or_else(|| format!("missing valuation definition: {id}"))?;
        for key in def.inputs {
            let fact = &root["facts"][*key];
            if !fact["value"].as_f64().is_some_and(f64::is_finite)
                || fact["unit"].as_str().is_none()
                || fact["label"].as_str().is_none()
                || fact["source_id"].as_str().is_none()
            {
                return Err(format!("invalid valuation fact: {key}"));
            }
        }
        for key in def.assumptions {
            if !root["assumptions"][*key]["value"]
                .as_f64()
                .is_some_and(f64::is_finite)
            {
                return Err(format!("invalid valuation assumption: {key}"));
            }
        }
    }
    Ok(())
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn canonical_observations(source_id: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    if source_id == "fred_cpi" {
        let text = std::str::from_utf8(bytes).map_err(|_| "FRED response is not UTF-8")?;
        let mut lines = text.lines();
        if lines.next() != Some("observation_date,CPIAUCSL") {
            return Err("FRED response does not identify CPIAUCSL".into());
        }
        let rows: BTreeMap<&str, &str> = lines.filter_map(|line| line.split_once(',')).collect();
        let mut out = String::new();
        for year in 2016..=2025 {
            let date = format!("{year}-09-01");
            let raw = rows
                .get(date.as_str())
                .ok_or_else(|| format!("missing FRED observation {date}"))?;
            let value: f64 = raw
                .parse()
                .map_err(|_| format!("invalid FRED observation {date}"))?;
            out.push_str(&format!("CPIAUCSL|{date}|{value:.3}\n"));
        }
        return Ok(out.into_bytes());
    }
    let (symbol, dates): (&str, &[&str]) = match source_id {
        "apple_price" => (
            "AAPL",
            &["11/01/2024", "11/03/2025", "11/21/2025", "11/26/2025"],
        ),
        "ebay_price" => ("EBAY", &["10/01/2025"]),
        "hpq_price" => ("HPQ", &["11/26/2025"]),
        _ => {
            return Err(format!(
                "no canonical verifier for valuation source {source_id}"
            ))
        }
    };
    let response: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid Nasdaq response: {e}"))?;
    if response["data"]["symbol"] != symbol {
        return Err(format!("Nasdaq response symbol does not match {symbol}"));
    }
    let rows = response["data"]["tradesTable"]["rows"]
        .as_array()
        .ok_or("Nasdaq response has no historical rows")?;
    let mut selected = BTreeMap::new();
    for row in rows {
        if let (Some(date), Some(close)) = (row["date"].as_str(), row["close"].as_str()) {
            if dates.contains(&date) {
                let value: f64 = close
                    .trim_start_matches('$')
                    .parse()
                    .map_err(|_| format!("invalid Nasdaq close for {date}"))?;
                if selected.insert(date, value).is_some() {
                    return Err(format!("duplicate Nasdaq observation for {symbol} {date}"));
                }
            }
        }
    }
    let mut out = String::new();
    for date in dates {
        let value = selected
            .get(date)
            .ok_or_else(|| format!("missing Nasdaq {symbol} observation {date}"))?;
        out.push_str(&format!("{symbol}|{date}|{value:.2}\n"));
    }
    Ok(out.into_bytes())
}

async fn verified_source(
    source_id: &str,
    cache_dir: &Path,
    expected: &ValuationSourceConfig,
) -> Result<&'static str, String> {
    if expected.bytes == 0 || expected.bytes > 20_000_000 || expected.sha256.len() != 64 {
        return Err("valuation source expectation is invalid".into());
    }
    let _guard = SOURCE_LOCK.lock().await;
    let directory = cache_dir.join("valuation-sources");
    let cached = directory.join(format!("{source_id}.source"));
    let archived_marker = directory.join(format!("{source_id}.archived-original"));
    if expected.verification_mode == "canonical_observations"
        && source_id == "fred_cpi"
        && !expected.url.contains("id=CPIAUCSL")
    {
        return Err("FRED canonical source is missing the reviewed series identity".into());
    }
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        let reviewed = if expected.verification_mode == "canonical_observations" {
            canonical_observations(source_id, &bytes)
        } else {
            Ok(bytes.clone())
        };
        if reviewed.as_ref().is_ok_and(|reviewed| {
            reviewed.len() == expected.bytes && fingerprint(reviewed) == expected.sha256
        }) {
            if archived_marker.exists() {
                if tokio::fs::read(&archived_marker)
                    .await
                    .is_ok_and(|marker| marker == expected.sha256.as_bytes())
                {
                    return Ok("verified_archived_original");
                }
                let _ = tokio::fs::remove_file(&archived_marker).await;
            } else {
                return Ok("verified_immutable_cache");
            }
        }
        tokio::fs::remove_file(&cached)
            .await
            .map_err(|e| format!("corrupt valuation cache removal failed: {e}"))?;
        let _ = tokio::fs::remove_file(&archived_marker).await;
    }
    let download = async {
        let response = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("Axiom educational source verifier")
            .build()
            .map_err(|e| format!("valuation source client failed: {e}"))?
            .get(&expected.url)
            .header("Accept", "application/json, text/plain, */*")
            .send()
            .await
            .map_err(|e| format!("valuation source request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("valuation source returned an error: {e}"))?;
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("valuation source body failed: {e}"))?;
            if bytes.len().saturating_add(chunk.len()) > 20_000_000 {
                return Err("valuation source body exceeds safety limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let reviewed = if expected.verification_mode == "canonical_observations" {
            canonical_observations(source_id, &bytes)?
        } else {
            bytes.clone()
        };
        if reviewed.len() != expected.bytes || fingerprint(&reviewed) != expected.sha256 {
            if source_id == "nuveen_proxy_2025" && !bytes.starts_with(b"%PDF-") {
                return Err("Nuveen original endpoint did not return a PDF".into());
            }
            return Err("downloaded valuation source differs from the reviewed evidence".into());
        }
        Ok::<Vec<u8>, String>(bytes)
    };
    let (bytes, status, archived) = match download.await {
        Ok(bytes) => (bytes, "verified_then_cached", false),
        Err(live_error) => {
            if live_error == "downloaded valuation source differs from the reviewed evidence" {
                return Err(live_error);
            }
            let Some(bytes) = archived_original(source_id) else {
                return Err(live_error);
            };
            if expected.verification_mode != "exact_bytes"
                || bytes.len() != expected.bytes
                || fingerprint(bytes) != expected.sha256
            {
                return Err(format!(
                    "archived valuation original differs from the reviewed evidence after live failure: {live_error}"
                ));
            }
            (bytes.to_vec(), "verified_archived_original", true)
        }
    };
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| format!("valuation cache directory failed: {e}"))?;
    tokio::fs::write(&cached, bytes)
        .await
        .map_err(|e| format!("valuation cache write failed: {e}"))?;
    if archived {
        tokio::fs::write(&archived_marker, expected.sha256.as_bytes())
            .await
            .map_err(|e| format!("valuation archived-original marker failed: {e}"))?;
    } else {
        let _ = tokio::fs::remove_file(&archived_marker).await;
    }
    Ok(status)
}

fn number(root: &Value, key: &str) -> Result<f64, String> {
    root["facts"][key]["value"]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing valuation fact {key}"))
}

fn assumption(root: &Value, key: &str) -> Result<f64, String> {
    root["assumptions"][key]["value"]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing valuation assumption {key}"))
}

fn ratio(numerator: f64, denominator: f64) -> Result<f64, String> {
    if denominator == 0.0 {
        return Err("valuation calculation has a zero denominator".into());
    }
    let value = numerator / denominator;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| "valuation calculation is invalid".into())
}

type Calculation = (Map<String, Value>, Map<String, Value>);

fn calculate(id: &str, root: &Value) -> Result<Calculation, String> {
    let n = |key: &str| number(root, key);
    let mut values = Map::new();
    let mut units = Map::new();
    let mut put = |key: &str, value: Value, unit: &str| {
        values.insert(key.into(), value);
        units.insert(key.into(), json!(unit));
    };
    let market_cap = || Ok::<_, String>(n("price")? * n("shares")?);
    let enterprise_value = || Ok::<_, String>(market_cap()? + n("debt")? - n("cash")?);
    match id {
        "altman_z" => put(
            "altman_z",
            json!(
                1.2 * ratio(
                    n("current_assets")? - n("current_liabilities")?,
                    n("assets")?
                )? + 1.4 * ratio(n("retained_earnings")?, n("assets")?)?
                    + 3.3 * ratio(n("operating_income")?, n("assets")?)?
                    + 0.6 * ratio(market_cap()?, n("liabilities")?)?
                    + ratio(n("sales")?, n("assets")?)?
            ),
            "score",
        ),
        "beneish_m" => {
            let dsri = ratio(
                ratio(n("receivables")?, n("sales")?)?,
                ratio(n("receivables_prev")?, n("sales_prev")?)?,
            )?;
            let gmi = ratio(
                ratio(n("gross_profit_prev")?, n("sales_prev")?)?,
                ratio(n("gross_profit")?, n("sales")?)?,
            )?;
            let aqi = ratio(
                1.0 - ratio(n("current_assets")? + n("ppe")?, n("assets")?)?,
                1.0 - ratio(
                    n("current_assets_prev")? + n("ppe_prev")?,
                    n("assets_prev")?,
                )?,
            )?;
            let sgi = ratio(n("sales")?, n("sales_prev")?)?;
            let depi = ratio(
                ratio(n("da_prev")?, n("da_prev")? + n("ppe_prev")?)?,
                ratio(n("da")?, n("da")? + n("ppe")?)?,
            )?;
            let sgai = ratio(
                ratio(n("sga")?, n("sales")?)?,
                ratio(n("sga_prev")?, n("sales_prev")?)?,
            )?;
            let lvgi = ratio(
                ratio(
                    n("current_liabilities")? + n("long_term_debt")?,
                    n("assets")?,
                )?,
                ratio(
                    n("current_liabilities_prev")? + n("long_term_debt_prev")?,
                    n("assets_prev")?,
                )?,
            )?;
            let tata = ratio(n("net_income")? - n("cfo")?, n("assets")?)?;
            for (key, value) in [
                ("dsri", dsri),
                ("gmi", gmi),
                ("aqi", aqi),
                ("sgi", sgi),
                ("depi", depi),
                ("sgai", sgai),
                ("tata", tata),
                ("lvgi", lvgi),
            ] {
                put(key, json!(value), "factor");
            }
            put(
                "beneish_m",
                json!(
                    -4.84 + 0.920 * dsri + 0.528 * gmi + 0.404 * aqi + 0.892 * sgi + 0.115 * depi
                        - 0.172 * sgai
                        + 4.679 * tata
                        - 0.327 * lvgi
                ),
                "score",
            );
        }
        "book_dcf" => {
            let base = n("cfo")? - n("capex")? + assumption(root, "dcf_net_borrowing")?;
            let g = assumption(root, "dcf_growth")?;
            let r = assumption(root, "dcf_discount")?;
            let tg = assumption(root, "dcf_terminal_growth")?;
            let years = assumption(root, "dcf_years")? as i32;
            if years <= 0 || r <= tg {
                return Err("invalid DCF assumptions".into());
            }
            let explicit = (1..=years)
                .map(|year| base * (1.0 + g).powi(year) / (1.0 + r).powi(year))
                .sum::<f64>();
            let terminal =
                base * (1.0 + g).powi(years) * (1.0 + tg) / (r - tg) / (1.0 + r).powi(years);
            let equity = explicit + terminal;
            put("base_free_cash_flow", json!(base), "USD millions");
            put("dcf_equity_value", json!(equity), "USD millions");
            put(
                "dcf_value_per_share",
                json!(ratio(equity, n("shares")?)?),
                "USD/share",
            );
        }
        "book_dividend_yield" => put(
            "dividend_yield",
            json!(ratio(n("dividend_per_share")?, n("price")?)?),
            "fraction",
        ),
        "book_earnings_yield" => put(
            "earnings_yield",
            json!(ratio(n("eps")?, n("price")?)?),
            "fraction",
        ),
        "book_ebitda_margin" => put(
            "ebitda_margin",
            json!(ratio(n("operating_income")? + n("da")?, n("sales")?)?),
            "fraction",
        ),
        "book_ev" => put(
            "enterprise_value",
            json!(enterprise_value()?),
            "USD millions",
        ),
        "book_ev_ebit" => put(
            "ev_ebit",
            json!(ratio(enterprise_value()?, n("operating_income")?)?),
            "multiple",
        ),
        "book_ev_sales" => put(
            "ev_sales",
            json!(ratio(enterprise_value()?, n("sales")?)?),
            "multiple",
        ),
        "book_fcf_yield" => put(
            "fcf_yield",
            json!(ratio(n("cfo")? - n("capex")?, market_cap()?)?),
            "fraction",
        ),
        "book_intangibles_ratio" => put(
            "intangibles_ratio",
            json!(ratio(
                n("ebay_goodwill")? + n("ebay_intangibles")?,
                n("ebay_equity")?
            )?),
            "fraction",
        ),
        "book_interest_coverage" => put(
            "interest_coverage",
            json!(ratio(
                n("ebay_operating_income")?,
                n("ebay_interest_expense")?
            )?),
            "multiple",
        ),
        "book_nav_discount" => put(
            "nav_premium_discount",
            json!(ratio(
                n("diax_market_price")? - n("diax_nav")?,
                n("diax_nav")?
            )?),
            "fraction",
        ),
        "book_net_debt_ebitda" => put(
            "net_debt_ebitda",
            json!(ratio(
                n("debt")? - n("cash")?,
                n("operating_income")? + n("da")?
            )?),
            "multiple",
        ),
        "book_payout_ratio" => put(
            "payout_ratio",
            json!(ratio(n("dividends")?, n("net_income")?)?),
            "fraction",
        ),
        "book_price_cashflow" => {
            put(
                "price_operating_cash_flow",
                json!(ratio(market_cap()?, n("cfo")?)?),
                "multiple",
            );
            put(
                "price_free_cash_flow",
                json!(ratio(market_cap()?, n("cfo")? - n("capex")?)?),
                "multiple",
            );
        }
        "book_ptbv" => put(
            "price_tangible_book",
            json!(ratio(
                n("ebay_price")? * n("ebay_shares")?,
                n("ebay_equity")? - n("ebay_goodwill")? - n("ebay_intangibles")?
            )?),
            "multiple",
        ),
        "book_qoq" => put(
            "qoq_growth",
            json!(ratio(n("sales")? - n("nine_month_sales")?, n("q3_sales")?)? - 1.0),
            "fraction",
        ),
        "ev_ebitda" => put(
            "ev_ebitda",
            json!(ratio(
                enterprise_value()?,
                n("operating_income")? + n("da")?
            )?),
            "multiple",
        ),
        "goodwill_ratio" => put(
            "goodwill_intangibles_to_equity",
            json!(ratio(
                n("ebay_goodwill")? + n("ebay_intangibles")?,
                n("ebay_equity")?
            )?),
            "fraction",
        ),
        "industry_pe_compare" => {
            let a = ratio(n("peer_apple_price")?, n("eps_2025")?)?;
            let h = ratio(n("hpq_price")?, n("hpq_eps")?)?;
            put("apple_pe", json!(a), "multiple");
            put("hp_pe", json!(h), "multiple");
            put("apple_to_hp_pe", json!(ratio(a, h)?), "ratio");
        }
        "pb" => put("pb", json!(ratio(market_cap()?, n("equity")?)?), "multiple"),
        "pe" => put("pe", json!(ratio(n("price")?, n("eps")?)?), "multiple"),
        "peg" => {
            let growth = ratio(n("eps")?, n("eps_prev")?)? - 1.0;
            put("earnings_growth", json!(growth), "fraction");
            put(
                "peg",
                json!(ratio(ratio(n("price")?, n("eps")?)?, growth * 100.0)?),
                "multiple per percentage point",
            );
        }
        "piotroski" => {
            let roa = ratio(n("net_income")?, n("assets_prev")?)?;
            let prior_roa = ratio(n("net_income_prev")?, n("assets_prev2")?)?;
            let current_average_assets = (n("assets")? + n("assets_prev")?) / 2.0;
            let prior_average_assets = (n("assets_prev")? + n("assets_prev2")?) / 2.0;
            let signals = [
                ("positive_roa", roa > 0.0),
                ("positive_cfo", n("cfo")? > 0.0),
                ("improving_roa", roa > prior_roa),
                ("cfo_exceeds_net_income", n("cfo")? > n("net_income")?),
                (
                    "declining_leverage",
                    ratio(n("long_term_debt")?, current_average_assets)?
                        < ratio(n("long_term_debt_prev")?, prior_average_assets)?,
                ),
                (
                    "improving_liquidity",
                    ratio(n("current_assets")?, n("current_liabilities")?)?
                        > ratio(n("current_assets_prev")?, n("current_liabilities_prev")?)?,
                ),
                (
                    "no_common_share_issuance",
                    n("common_shares_issued")? == 0.0,
                ),
                (
                    "improving_gross_margin",
                    ratio(n("gross_profit")?, n("sales")?)?
                        > ratio(n("gross_profit_prev")?, n("sales_prev")?)?,
                ),
                (
                    "improving_asset_turnover",
                    ratio(n("sales")?, n("assets_prev")?)?
                        > ratio(n("sales_prev")?, n("assets_prev2")?)?,
                ),
            ];
            for (key, passed) in signals {
                put(key, json!(u8::from(passed)), "binary");
            }
            put(
                "piotroski_f_score",
                json!(signals.iter().filter(|(_, passed)| *passed).count()),
                "0-9 score",
            );
        }
        "ps" => put("ps", json!(ratio(market_cap()?, n("sales")?)?), "multiple"),
        "roic" => {
            let nopat =
                n("operating_income")? * (1.0 - ratio(n("income_tax")?, n("pretax_income")?)?);
            let invested = ((n("debt")? + n("equity")? - n("cash")?)
                + (n("debt_prev")? + n("equity_prev")? - n("cash_prev")?))
                / 2.0;
            put("nopat", json!(nopat), "USD millions");
            put("roic", json!(ratio(nopat, invested)?), "fraction");
        }
        "book_cape" => {
            let target = n("cpi_2025")?;
            let mut sum = 0.0;
            for year in 2016..=2025 {
                sum += n(&format!("eps_{year}"))? * ratio(target, n(&format!("cpi_{year}"))?)?;
            }
            let average = sum / 10.0;
            put("real_average_eps", json!(average), "2025 USD/share");
            put("cape", json!(ratio(n("cape_price")?, average)?), "multiple");
        }
        _ => return Err(format!("unsupported valuation concept: {id}")),
    }
    Ok((values, units))
}

fn result_key(id: &str) -> &'static str {
    match id {
        "altman_z" => "altman_z",
        "beneish_m" => "beneish_m",
        "book_dcf" => "dcf_value_per_share",
        "book_dividend_yield" => "dividend_yield",
        "book_earnings_yield" => "earnings_yield",
        "book_ebitda_margin" => "ebitda_margin",
        "book_ev" => "enterprise_value",
        "book_ev_ebit" => "ev_ebit",
        "book_ev_sales" => "ev_sales",
        "book_fcf_yield" => "fcf_yield",
        "book_intangibles_ratio" => "intangibles_ratio",
        "book_interest_coverage" => "interest_coverage",
        "book_nav_discount" => "nav_premium_discount",
        "book_net_debt_ebitda" => "net_debt_ebitda",
        "book_payout_ratio" => "payout_ratio",
        "book_price_cashflow" => "price_operating_cash_flow",
        "book_ptbv" => "price_tangible_book",
        "book_qoq" => "qoq_growth",
        "ev_ebitda" => "ev_ebitda",
        "goodwill_ratio" => "goodwill_intangibles_to_equity",
        "industry_pe_compare" => "apple_pe",
        "pb" => "pb",
        "pe" => "pe",
        "peg" => "peg",
        "piotroski" => "piotroski_f_score",
        "ps" => "ps",
        "roic" => "roic",
        "book_cape" => "cape",
        _ => "",
    }
}

fn derived_metric(key: &str) -> (&'static str, &'static str) {
    match key {
        "altman_z" => (
            "Altman Z分数",
            "由五个偿债与经营因子加权得到的上市制造业财务风险筛查分数。",
        ),
        "dsri" => (
            "应收账款指数（DSRI）",
            "本期应收账款收入比除以上期同口径比率。",
        ),
        "gmi" => (
            "毛利率指数（GMI）",
            "上期毛利率除以本期毛利率；高于1表示毛利率下降。",
        ),
        "aqi" => (
            "资产质量指数（AQI）",
            "本期与上期非流动非固定资产占比的比值。",
        ),
        "sgi" => ("销售增长指数（SGI）", "本期营业收入除以上期营业收入。"),
        "depi" => ("折旧指数（DEPI）", "上期折旧率除以本期折旧率。"),
        "sgai" => (
            "销售管理费用指数（SGAI）",
            "本期销售管理费用率除以上期同口径比率。",
        ),
        "tata" => ("总应计项（TATA）", "净利润减经营现金流后除以总资产。"),
        "lvgi" => (
            "杠杆指数（LVGI）",
            "本期流动负债加非流动定期债务占资产比，除以上期同口径比率。",
        ),
        "beneish_m" => (
            "Beneish M分数",
            "八个Beneish因子按模型系数加权后的筛查分数。",
        ),
        "base_free_cash_flow" => ("基期简化FCFE", "经营现金流减资本开支并加零净举债情景假设。"),
        "dcf_equity_value" => ("DCF股权价值", "显式期简化FCFE现值与终值现值之和。"),
        "dcf_value_per_share" => ("DCF每股价值", "DCF股权价值除以期末流通股数。"),
        "dividend_yield" => ("股息率", "每股宣告股利除以估值日股价。"),
        "earnings_yield" => ("盈利收益率", "稀释每股收益除以估值日股价。"),
        "ebitda_margin" => ("EBITDA利润率", "营业利润加折旧摊销后除以营业收入。"),
        "enterprise_value" => ("企业价值", "市值加有息负债再减现金及现金等价物。"),
        "ev_ebit" => ("EV/EBIT", "企业价值除以营业利润。"),
        "ev_sales" => ("EV/Sales", "企业价值除以营业收入。"),
        "fcf_yield" => ("自由现金流收益率", "经营现金流减资本开支后除以市值。"),
        "intangibles_ratio" => ("无形资产权益比", "商誉加可辨认无形资产除以股东权益。"),
        "interest_coverage" => ("利息保障倍数", "营业利润除以利息费用。"),
        "nav_premium_discount" => ("NAV折溢价", "市场价格减每股NAV后除以每股NAV。"),
        "net_debt_ebitda" => ("净债务/EBITDA", "有息负债减现金后除以营业利润加折旧摊销。"),
        "payout_ratio" => ("现金分红支付率", "现金流量表已支付股利除以净利润。"),
        "price_operating_cash_flow" => ("市现率", "市值除以经营现金流。"),
        "price_free_cash_flow" => ("市值/自由现金流", "市值除以经营现金流减资本开支。"),
        "price_tangible_book" => (
            "市值/有形账面价值",
            "市值除以扣除商誉与可辨认无形资产后的权益。",
        ),
        "qoq_growth" => (
            "第四季度营收环比",
            "全年减前九月得到第四季度营收，再与第三季度比较。",
        ),
        "ev_ebitda" => ("EV/EBITDA", "企业价值除以营业利润加折旧摊销。"),
        "goodwill_intangibles_to_equity" => {
            ("商誉及无形资产权益比", "商誉加可辨认无形资产除以股东权益。")
        }
        "apple_pe" => ("Apple市盈率", "Apple同日收盘价除以最新已公布财年稀释EPS。"),
        "hp_pe" => ("HP市盈率", "HP同日收盘价除以最新已公布财年稀释EPS。"),
        "apple_to_hp_pe" => ("Apple/HP市盈率比", "Apple市盈率除以HP市盈率。"),
        "pb" => ("市净率", "市值除以股东权益。"),
        "pe" => ("市盈率", "估值日收盘价除以财年稀释EPS。"),
        "earnings_growth" => ("EPS同比增速", "本期稀释EPS相对上期的增长率。"),
        "peg" => ("PEG", "市盈率除以EPS同比增长率的百分点数。"),
        "positive_roa" => ("资产收益率为正", "本期净利润除以上期末总资产大于零时记1。"),
        "positive_cfo" => ("经营现金流为正", "本期经营现金流大于零时记1。"),
        "improving_roa" => ("资产收益率改善", "本期ROA高于上期ROA时记1。"),
        "cfo_exceeds_net_income" => ("现金流高于净利润", "经营现金流高于净利润时记1。"),
        "declining_leverage" => ("杠杆下降", "非流动定期债务除以平均总资产的比率下降时记1。"),
        "improving_liquidity" => ("流动性改善", "流动比率高于上期时记1。"),
        "no_common_share_issuance" => (
            "未发行普通股",
            "年内普通股发行数为零时记1；回购不抵销发行判断。",
        ),
        "improving_gross_margin" => ("毛利率改善", "本期毛利率高于上期时记1。"),
        "improving_asset_turnover" => (
            "资产周转改善",
            "营业收入除以期初总资产的比率高于上期时记1。",
        ),
        "piotroski_f_score" => (
            "Piotroski F分数",
            "九个盈利、杠杆流动性与营运信号的0/1得分之和。",
        ),
        "ps" => ("市销率", "市值除以营业收入。"),
        "nopat" => (
            "税后营业利润（NOPAT）",
            "营业利润按当期实际税率扣税后的结果。",
        ),
        "roic" => ("投入资本回报率", "NOPAT除以平均投入资本。"),
        "real_average_eps" => (
            "十年实际平均EPS",
            "十年稀释EPS按目标月CPI折算后的算术平均。",
        ),
        "cape" => ("周期调整市盈率（CAPE）", "估值日股价除以十年实际平均EPS。"),
        _ => ("派生指标", "由所列来源事实按返回公式计算。"),
    }
}

fn source_title(id: &str) -> &'static str {
    match id {
        "apple_2025" => "Apple 2025 年 Form 10-K",
        "apple_q3_2025" => "Apple 2025 财年第三季度财务报表",
        "apple_2024" => "Apple 2024 年 Form 10-K",
        "apple_2022" => "Apple 2022 年 Form 10-K",
        "apple_2019" => "Apple 2019 年 Form 10-K",
        "apple_2016" => "Apple 2016 年 Form 10-K",
        "fred_cpi" => "FRED CPIAUCSL 于 2026-10-03 检索的后见修订观察值",
        "apple_price" => "Nasdaq AAPL 历史收盘价",
        "ebay_2024" => "eBay 2024 年 Form 10-K",
        "ebay_price" => "Nasdaq EBAY 历史收盘价",
        "hpq_2024" => "HP Inc. 2024 财年业绩公告",
        "hpq_2025" => "HP Inc. 2025 财年业绩公告",
        "hpq_price" => "Nasdaq HPQ 历史收盘价",
        "nuveen_proxy_2025" => "Nuveen 封闭式基金联合委托书/招股说明书",
        _ => "已验证公开来源",
    }
}

fn actual_pdf_page(source_id: &str, printed_page: Option<u64>) -> Option<u64> {
    printed_page.map(|page| {
        if matches!(
            source_id,
            "apple_2016" | "apple_2019" | "apple_2022" | "apple_2024" | "apple_2025"
        ) {
            page + 3
        } else {
            page
        }
    })
}

fn case_metadata(id: &str, root: &Value) -> (Value, Value, &'static str, &'static str) {
    match id {
        "book_intangibles_ratio" | "book_interest_coverage" | "book_ptbv" | "goodwill_ratio" => (
            json!({"name":"eBay Inc.","ticker":"EBAY","alternate_tickers":[],"reporting_entity":"eBay Inc. and its consolidated subsidiaries","metric_entity":"eBay Inc. and its consolidated subsidiaries"}),
            json!({"label":"FY2024","start":"2024-01-01","end":"2024-12-31"}),
            "USD",
            "USD millions except per-share facts",
        ),
        "book_nav_discount" => (
            json!({"name":"Nuveen Dow 30 Dynamic Overwrite Fund","ticker":"DIAX","alternate_tickers":[],"reporting_entity":"Nuveen Dow 30 Dynamic Overwrite Fund","metric_entity":"DIAX common shares"}),
            json!({"label":"截至2025-09-30季度内最高收盘价观察（具体交易日未披露）","start":"2025-07-01","end":"2025-09-30"}),
            "USD",
            "USD per common share",
        ),
        "industry_pe_compare" => (
            json!({"name":"Apple Inc. and HP Inc. hardware peer set","ticker":"AAPL","alternate_tickers":["HPQ"],"reporting_entity":"Apple Inc. and HP Inc., each on its own fiscal-year basis","metric_entity":"Named hardware peers Apple Inc. and HP Inc."}),
            json!({"label":"FY2025 EPS at 2025-11-26 close","start":"2025-09-27","end":"2025-11-26"}),
            "USD",
            "USD/share and valuation multiples",
        ),
        _ => (
            json!({"name":root["issuer"]["name"],"ticker":"AAPL","alternate_tickers":[],"reporting_entity":"Apple Inc. and its consolidated subsidiaries","metric_entity":"Apple Inc. and its consolidated subsidiaries"}),
            root["period"].clone(),
            "USD",
            "USD millions except per-share and ratio facts",
        ),
    }
}

pub async fn evaluate(
    id: &str,
    cache_dir: &Path,
    registry: &ValuationSourceRegistry,
) -> Result<Value, String> {
    let def = definition(id).ok_or_else(|| format!("unsupported valuation concept: {id}"))?;
    let root = data()?;
    let mut verified = BTreeMap::new();
    for source_id in def.sources {
        let config = registry
            .sources
            .get(*source_id)
            .ok_or_else(|| format!("missing valuation source config: {source_id}"))?;
        verified.insert(
            *source_id,
            (config, verified_source(source_id, cache_dir, config).await?),
        );
    }
    let (values, units) = calculate(id, root)?;
    let mut sources = Vec::new();
    for source_id in def.sources {
        let meta = &root["sources"][*source_id];
        let (config, status) = verified[source_id];
        let basis = if config.verification_mode == "canonical_observations" {
            "canonical_selected_observations"
        } else {
            "exact_document_bytes"
        };
        let retrieval_note = (status == "verified_archived_original").then_some(
            "Nuveen 原站当前未返回已核验 PDF；本次使用字节数与 SHA-256 完全相同的已核验原文备份。",
        );
        sources.push(json!({"id":source_id,"title":source_title(source_id),"url":meta["url"],"sha256":meta["sha256"],"bytes":meta["bytes"],"format":meta["format"],"published":meta["published"],"verification_basis":basis,"verification":{"status":status,"verified_at":chrono::Utc::now(),"requested_url":config.url,"matched_sha256":config.sha256,"matched_bytes":config.bytes,"fingerprint_scope":basis,"retrieval_note":retrieval_note}}));
    }
    let mut facts = Vec::new();
    let mut symbol_mapping = Map::new();
    let mut field_provenance = Map::new();
    let mut pdf_pages = Vec::new();
    for key in def.inputs {
        let fact = &root["facts"][*key];
        let source_id = fact["source_id"].as_str().unwrap();
        let source = &root["sources"][source_id];
        let page = actual_pdf_page(source_id, fact.get("page").and_then(Value::as_u64));
        if let Some(page) = page {
            pdf_pages.push(page);
        }
        let kind = fact
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("reported");
        symbol_mapping.insert((*key).into(), fact["label"].clone());
        field_provenance.insert((*key).into(), json!({"kind":kind,"pdf_page":page,"note":format!("Source field: {}",fact["label"].as_str().unwrap_or(key))}));
        facts.push(json!({"key":key,"label":fact["label"],"value":fact["value"],"unit":fact["unit"],"pdf_page":page,"source_url":source["url"],"source_format":source["format"],"source_section":fact.get("section").cloned().unwrap_or(Value::Null),"published":source["published"],"as_of":fact["as_of"],"kind":fact.get("kind").cloned().unwrap_or(json!("reported"))}));
    }
    for key in def.assumptions {
        let item = &root["assumptions"][*key];
        symbol_mapping.insert((*key).into(), item["label"].clone());
        field_provenance.insert(
            (*key).into(),
            json!({"kind":"assumption","note":"Explicit teaching assumption; not issuer guidance"}),
        );
        facts.push(json!({"key":key,"label":item["label"],"value":item["value"],"unit":item["unit"],"pdf_page":Value::Null,"source_url":Value::Null,"source_format":"teaching assumption","source_section":"explicit DCF assumption","published":Value::Null,"as_of":Value::Null,"kind":"assumption"}));
    }
    for key in values.keys() {
        field_provenance.insert(key.clone(), json!({"kind":"derived","note":format!("Calculated from listed facts and assumptions using {}",def.formula)}));
    }
    pdf_pages.sort_unstable();
    pdf_pages.dedup();
    let primary = &sources[0];
    let (issuer, mut period, currency, scale) = case_metadata(id, root);
    let price_dates = def
        .inputs
        .iter()
        .filter_map(|key| {
            let fact = &root["facts"][*key];
            fact["source_id"]
                .as_str()
                .filter(|source| source.ends_with("_price"))?;
            fact["as_of"].as_str()
        })
        .collect::<std::collections::BTreeSet<_>>();
    if !price_dates.is_empty() {
        period["label"] = json!(format!(
            "{} · 股价日期 {}",
            period["label"].as_str().unwrap(),
            price_dates.iter().copied().collect::<Vec<_>>().join("、")
        ));
        period["end"] = json!(price_dates.last().unwrap());
    }
    if id == "book_cape" {
        period["label"] = json!("2016–2025 EPS 与 CPI · 股价日期 2025-11-21");
        period["start"] = json!("2016-09-01");
    }
    let derived_metrics = values
        .keys()
        .filter(|key| values[*key].is_number())
        .map(|key| {
            let (label, description) = derived_metric(key);
            json!({"key": key, "label": label, "description": description})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"concept_id":id,"input_kind":"industry_case","provenance":"verified_original_public_sources","status":"computed","reason":Value::Null,"values":values,"units":units,"series":[],"inputs":{},"bars":[],"notes":["固定历史案例，不代表当前行情、任意股票或实时估值结论。",def.note],"industry_case":{"label":"财报与估值","case_id":format!("{}-{id}",root["case_id"].as_str().unwrap()),"issuer":issuer,"period":period,"published":primary["published"],"audited":false,"audit_boundary":"来源财务报表的审计状态以原报告为准；本页估值公式、派生结果、历史行情与宏观观察记录未接受单独审计。","status":"reviewed_historical_source","url":primary["url"],"format":primary["format"],"pdf_pages":pdf_pages,"sha256":primary["sha256"],"bytes":primary["bytes"],"source_kind":"verified_original_public_source","sources":sources,"verification":primary["verification"]},"industry_facts":{"currency":currency,"scale":scale,"reported_facts":facts,"derived_metrics":derived_metrics,"calculation":{"formula":def.formula,"result_key":result_key(id),"operands":def.inputs,"symbol_mapping":symbol_mapping},"field_provenance":field_provenance,"definitions":{"case_boundary":"只适用于返回的具名发行人或基金、来源集、报告期与估值日；不从调用者选择的任意数据外推。","metric":def.note}}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn nasdaq(symbol: &str, rows: &[(&str, &str)]) -> Vec<u8> {
        serde_json::to_vec(&json!({"data":{"symbol":symbol,"tradesTable":{"rows":rows.iter().map(|(date,close)|json!({"date":date,"close":format!("${close}")})).collect::<Vec<_>>()}}})).unwrap()
    }

    async fn serve_once(body: Vec<u8>, content_type: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request).await.unwrap();
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).await.unwrap();
            stream.write_all(&body).await.unwrap();
        });
        format!("http://{address}/nuveen.pdf")
    }

    fn seeded(root: &Path) -> ValuationSourceRegistry {
        let mut registry = ValuationSourceRegistry::default();
        let directory = root.join("valuation-sources");
        std::fs::create_dir_all(&directory).unwrap();
        for (id, source) in &mut registry.sources {
            let bytes=match id.as_str() {
                "apple_price"=>nasdaq("AAPL",&[("11/01/2024","222.91"),("11/03/2025","269.05"),("11/21/2025","271.49"),("11/26/2025","277.55")]),
                "ebay_price"=>nasdaq("EBAY",&[("10/01/2025","87.58")]),
                "hpq_price"=>nasdaq("HPQ",&[("11/26/2025","23.98")]),
                "fred_cpi"=>b"observation_date,CPIAUCSL\n2016-09-01,241.176\n2017-09-01,246.435\n2018-09-01,252.182\n2019-09-01,256.430\n2020-09-01,259.997\n2021-09-01,273.910\n2022-09-01,296.349\n2023-09-01,307.276\n2024-09-01,314.732\n2025-09-01,324.245\n".to_vec(),
                _=>{ let fixture=format!("fixture:{id}").into_bytes(); source.bytes=fixture.len(); source.sha256=fingerprint(&fixture); fixture }
            };
            std::fs::write(directory.join(format!("{id}.source")), bytes).unwrap();
        }
        registry
    }

    #[tokio::test]
    async fn every_case_computes_from_independent_expected_literals() {
        let root = std::env::temp_dir().join(format!("axiom-valuation-{}", uuid::Uuid::new_v4()));
        let registry = seeded(&root);
        let expected = [
            ("altman_z", "altman_z", 10.61901749051463),
            ("beneish_m", "beneish_m", -2.294943021712222),
            ("book_dcf", "dcf_value_per_share", 125.12238523309892),
            ("book_dividend_yield", "dividend_yield", 1.02 / 269.05),
            ("book_earnings_yield", "earnings_yield", 7.46 / 269.05),
            ("book_ebitda_margin", "ebitda_margin", 144748.0 / 416161.0),
            ("book_ev", "enterprise_value", 4037468.603),
            ("book_ev_ebit", "ev_ebit", 4037468.603 / 133050.0),
            ("book_ev_sales", "ev_sales", 4037468.603 / 416161.0),
            ("book_fcf_yield", "fcf_yield", 98767.0 / 3974745.603),
            (
                "book_intangibles_ratio",
                "intangibles_ratio",
                4388.0 / 5158.0,
            ),
            (
                "book_interest_coverage",
                "interest_coverage",
                2318.0 / 259.0,
            ),
            (
                "book_nav_discount",
                "nav_premium_discount",
                (14.91 - 16.30) / 16.30,
            ),
            (
                "book_net_debt_ebitda",
                "net_debt_ebitda",
                62723.0 / 144748.0,
            ),
            ("book_payout_ratio", "payout_ratio", 15421.0 / 112010.0),
            (
                "book_price_cashflow",
                "price_operating_cash_flow",
                3974745.603 / 111482.0,
            ),
            ("book_ptbv", "price_tangible_book", 87.58 * 471.0 / 770.0),
            ("book_qoq", "qoq_growth", 102466.0 / 94036.0 - 1.0),
            ("ev_ebitda", "ev_ebitda", 4037468.603 / 144748.0),
            (
                "goodwill_ratio",
                "goodwill_intangibles_to_equity",
                4388.0 / 5158.0,
            ),
            ("industry_pe_compare", "apple_pe", 277.55 / 7.46),
            ("pb", "pb", 3974745.603 / 73733.0),
            ("pe", "pe", 269.05 / 7.46),
            (
                "peg",
                "peg",
                (269.05 / 7.46) / ((7.46 / 6.08 - 1.0) * 100.0),
            ),
            ("piotroski", "piotroski_f_score", 7.0),
            ("ps", "ps", 3974745.603 / 416161.0),
            ("roic", "roic", 0.8314270092598283),
        ];
        for (id, key, want) in expected {
            let result = evaluate(id, &root, &registry).await.unwrap();
            let got = result["values"][key].as_f64().unwrap();
            assert!((got - want).abs() < 1e-10, "{id}: {got} != {want}");
            assert!(result["industry_case"]["sources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["verification"]["status"] == "verified_immutable_cache"));
            assert_eq!(result["industry_facts"]["calculation"]["result_key"], key);
            assert!(result["industry_case"]["issuer"]["reporting_entity"]
                .as_str()
                .is_some());
            assert!(result["industry_facts"]["field_provenance"]
                .as_object()
                .is_some());
        }
        let cape = evaluate("book_cape", &root, &registry).await.unwrap();
        assert!(
            (cape["values"]["real_average_eps"].as_f64().unwrap() - 5.101838897780571).abs()
                < 1e-12
        );
        assert!((cape["values"]["cape"].as_f64().unwrap() - 53.214146004905224).abs() < 1e-12);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn public_registry_and_error_guards_are_complete() {
        assert!(data().is_ok());
        assert!(SUPPORTED_IDS.iter().all(|id| is_supported(id)
            && !fixed_symbol(id).is_empty()
            && definition(id).is_some()));
        assert!(!is_supported("unknown"));
        assert_eq!(fixed_symbol("unknown"), "");
        assert_eq!(required_datasets("unknown"), Vec::<&str>::new());
        assert_eq!(
            required_datasets("book_ebitda_margin"),
            vec!["issuer_financial_disclosures"]
        );
        assert_eq!(
            required_datasets("pe"),
            vec![
                "issuer_financial_disclosures",
                "nasdaq_historical_close_observations"
            ]
        );
        assert_eq!(
            required_datasets("book_dcf"),
            vec!["issuer_financial_disclosures", "explicit_model_assumptions"]
        );
        assert_eq!(
            required_datasets("book_cape"),
            vec![
                "issuer_financial_disclosures",
                "nasdaq_historical_close_observations",
                "ten_year_diluted_eps_split_adjusted_history",
                "fred_cpiaucsl_revised_snapshot_post_revisions"
            ]
        );
        assert!(ratio(1.0, 0.0).is_err());
        assert!(calculate("unknown", data().unwrap()).is_err());
        assert!(canonical_observations("unknown", b"{}").is_err());
        assert!(canonical_observations("apple_price", b"{}").is_err());
        assert!(canonical_observations(
            "apple_price",
            &nasdaq("MSFT", &[("11/03/2025", "269.05")])
        )
        .is_err());
        assert!(canonical_observations(
            "ebay_price",
            &nasdaq("EBAY", &[("10/01/2025", "87.58"), ("10/01/2025", "87.58")])
        )
        .is_err());
        assert!(canonical_observations("fred_cpi", b"observation_date,CPIAUCSL\n").is_err());
        assert!(canonical_observations("fred_cpi", b"DATE,WRONG\n").is_err());
    }

    #[tokio::test]
    async fn validation_and_formula_error_paths_fail_closed() {
        let pristine = data().unwrap().clone();

        let mut invalid = pristine.clone();
        invalid["issuer"]["ticker"] = json!("OTHER");
        assert!(validate_data(&invalid).is_err());

        let mut invalid = pristine.clone();
        invalid["sources"]["apple_2025"]["url"] = Value::Null;
        assert!(validate_data(&invalid).is_err());

        let mut invalid = pristine.clone();
        invalid["facts"]["price"]["value"] = Value::Null;
        assert!(validate_data(&invalid).is_err());

        let mut invalid = pristine.clone();
        invalid["assumptions"]["dcf_growth"]["value"] = Value::Null;
        assert!(validate_data(&invalid).is_err());

        let mut invalid = pristine.clone();
        invalid["assumptions"]["dcf_discount"]["value"] = json!(0.02);
        assert!(calculate("book_dcf", &invalid).is_err());

        assert_eq!(result_key("unknown"), "");
        assert_eq!(source_title("unknown"), "已验证公开来源");
        assert!(canonical_observations("fred_cpi", &[0xff]).is_err());
        assert!(
            canonical_observations("fred_cpi", b"observation_date,CPIAUCSL\n2016-09-01,bad\n")
                .is_err()
        );
        assert!(canonical_observations(
            "apple_price",
            br#"{"data":{"symbol":"AAPL","tradesTable":{}}}"#
        )
        .is_err());
        assert!(
            canonical_observations("apple_price", &nasdaq("AAPL", &[("11/03/2025", "bad")]))
                .is_err()
        );
        assert!(canonical_observations(
            "apple_price",
            &nasdaq("AAPL", &[("11/03/2025", "269.05")])
        )
        .is_err());

        let root =
            std::env::temp_dir().join(format!("axiom-valuation-guards-{}", uuid::Uuid::new_v4()));
        let mut invalid_source = ValuationSourceRegistry::default().sources["apple_2025"].clone();
        invalid_source.bytes = 0;
        assert!(verified_source("apple_2025", &root, &invalid_source)
            .await
            .is_err());

        let mut invalid_fred = ValuationSourceRegistry::default().sources["fred_cpi"].clone();
        invalid_fred.url = "https://example.invalid/not-cpi".into();
        assert!(verified_source("fred_cpi", &root, &invalid_fred)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn controlled_nuveen_document_downloads_and_then_uses_immutable_cache() {
        let root = std::env::temp_dir().join(format!(
            "axiom-valuation-live-source-{}",
            uuid::Uuid::new_v4()
        ));
        let mut source = ValuationSourceRegistry::default().sources["nuveen_proxy_2025"].clone();
        source.url = serve_once(
            NUVEEN_PROXY_2025_ARCHIVED_ORIGINAL.to_vec(),
            "application/pdf",
        )
        .await;
        assert_eq!(
            verified_source("nuveen_proxy_2025", &root, &source)
                .await
                .unwrap(),
            "verified_then_cached"
        );
        assert_eq!(
            verified_source("nuveen_proxy_2025", &root, &source)
                .await
                .unwrap(),
            "verified_immutable_cache"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn nuveen_security_page_uses_only_the_fingerprint_matched_archived_original() {
        let root = std::env::temp_dir().join(format!(
            "axiom-valuation-archived-source-{}",
            uuid::Uuid::new_v4()
        ));
        let mut registry = ValuationSourceRegistry::default();
        registry.sources.get_mut("nuveen_proxy_2025").unwrap().url =
            serve_once(b"<html>TIAA security page</html>".to_vec(), "text/html").await;
        let result = evaluate("book_nav_discount", &root, &registry)
            .await
            .unwrap();
        let verification = &result["industry_case"]["verification"];
        assert_eq!(verification["status"], "verified_archived_original");
        assert_eq!(verification["matched_bytes"], 1_438_050);
        assert_eq!(
            verification["matched_sha256"],
            "328251373d35c20d0450538dad87c1bf28ca6747393dbc7ddb72d57bb1ccfb19"
        );
        assert!(verification["retrieval_note"]
            .as_str()
            .unwrap()
            .contains("原站当前未返回已核验 PDF"));
        let cached =
            std::fs::read(root.join("valuation-sources/nuveen_proxy_2025.source")).unwrap();
        assert_eq!(
            fingerprint(&cached),
            verification["matched_sha256"].as_str().unwrap()
        );

        let mut wrong = registry.sources["nuveen_proxy_2025"].clone();
        wrong.sha256 = "0".repeat(64);
        let wrong_root = std::env::temp_dir().join(format!(
            "axiom-valuation-wrong-archive-{}",
            uuid::Uuid::new_v4()
        ));
        wrong.url = serve_once(b"<html>TIAA security page</html>".to_vec(), "text/html").await;
        assert!(verified_source("nuveen_proxy_2025", &wrong_root, &wrong)
            .await
            .is_err());
        assert!(!wrong_root
            .join("valuation-sources/nuveen_proxy_2025.source")
            .exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn corrupt_canonical_cache_is_removed_before_fresh_fetch() {
        let root =
            std::env::temp_dir().join(format!("axiom-valuation-corrupt-{}", uuid::Uuid::new_v4()));
        let directory = root.join("valuation-sources");
        std::fs::create_dir_all(&directory).unwrap();
        let cached = directory.join("apple_price.source");
        std::fs::write(&cached, b"not-json").unwrap();
        let mut source = ValuationSourceRegistry::default().sources["apple_price"].clone();
        source.url = "http://127.0.0.1:1/unreachable".into();
        assert!(verified_source("apple_price", &root, &source)
            .await
            .is_err());
        assert!(!cached.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
