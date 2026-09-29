//! Verified, issuer-published historical industry cases.
//!
//! These cases are fixed to one issuer and period. Results are released only
//! after the reviewed PDF matches its exact byte length and SHA-256.

use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::OnceLock, time::Duration};
use tokio::sync::Mutex;

pub const PING_AN_URL: &str =
    "https://pagroup.pingan.com/resource/pingan/IR-Docs/2025/pingan-ar24-report.pdf";
pub const PING_AN_SHA256: &str = "62a5bd793ef9a787cc95750d65e52803aa58fa424b01d94e754ea0120d5be8a3";
pub const PING_AN_BYTES: usize = 14_886_158;
pub const SHOPIFY_URL: &str =
    "https://s27.q4cdn.com/572064924/files/doc_financials/2024/q4/Q4-2024-Press-Release-Final.pdf";
pub const SHOPIFY_SHA256: &str = "4bf71232697a2270b2dbc38fc9609c11c27d545d6f4301fce3356fa60c6ef6de";
pub const SHOPIFY_BYTES: usize = 86_468;
pub const REALTY_INCOME_URL: &str = "https://www.realtyincome.com/sites/realty-income/files/2025-02/realty-income-q4-2024-supplemental-information.pdf";
pub const REALTY_INCOME_SHA256: &str =
    "a0b3bf067c7b19ebde01ceaac3ecb172ed6a4c7084eeabe276ad1d4599c62a3f";
pub const REALTY_INCOME_BYTES: usize = 17_522_920;
pub const EBAY_URL: &str =
    "https://ebay.q4cdn.com/610426115/files/doc_financials/2024/q4/eBay-10-K-2024.pdf";
pub const EBAY_SHA256: &str = "10530b8314c4dc49f9737b938f28ead7a70212885c35919fb361d145401f37fb";
pub const EBAY_BYTES: usize = 1_004_020;

pub const SUPPORTED_IDS: &[&str] = &[
    "bank_nim",
    "book_bank_nim",
    "book_bank_cost_income",
    "book_bank_npl_ratio",
    "book_insurance_solvency_ratio",
    "book_saas_arr",
    "book_saas_rule_of_40",
    "book_platform_gmv",
    "book_platform_take_rate",
    "book_reit_occupancy",
];

static SOURCE_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndustrySourceConfig {
    pub url: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndustrySourceRegistry {
    pub ping_an: IndustrySourceConfig,
    pub shopify: IndustrySourceConfig,
    pub realty_income: IndustrySourceConfig,
    pub ebay: IndustrySourceConfig,
}

impl Default for IndustrySourceRegistry {
    fn default() -> Self {
        Self {
            ping_an: IndustrySourceConfig {
                url: PING_AN_URL.into(),
                sha256: PING_AN_SHA256.into(),
                bytes: PING_AN_BYTES,
            },
            shopify: IndustrySourceConfig {
                url: SHOPIFY_URL.into(),
                sha256: SHOPIFY_SHA256.into(),
                bytes: SHOPIFY_BYTES,
            },
            realty_income: IndustrySourceConfig {
                url: REALTY_INCOME_URL.into(),
                sha256: REALTY_INCOME_SHA256.into(),
                bytes: REALTY_INCOME_BYTES,
            },
            ebay: IndustrySourceConfig {
                url: EBAY_URL.into(),
                sha256: EBAY_SHA256.into(),
                bytes: EBAY_BYTES,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct TypedFact {
    value: f64,
    unit: String,
    page: u16,
    label: String,
}

#[derive(Clone, Debug, Deserialize)]
struct IndustryCase {
    case_id: String,
    issuer: String,
    ticker: String,
    alternate_tickers: Vec<String>,
    reporting_entity: String,
    currency: String,
    scale: String,
    period_label: String,
    period_start: String,
    period_end: String,
    published: String,
    source_kind: String,
    source_status: String,
    url: String,
    sha256: String,
    bytes: usize,
    facts: BTreeMap<String, TypedFact>,
}

#[derive(Clone, Debug, Deserialize)]
struct IndustryCases {
    ping_an: IndustryCase,
    shopify: IndustryCase,
    realty_income: IndustryCase,
    ebay: IndustryCase,
}

#[derive(Clone, Copy)]
struct MetricDefinition {
    case: CaseKey,
    metric_entity: &'static str,
    formula: &'static str,
    inputs: &'static [&'static str],
    audited: bool,
    audit_boundary: &'static str,
    value_key: &'static str,
    value_unit: &'static str,
    note: &'static str,
}

#[derive(Clone, Copy)]
enum CaseKey {
    PingAn,
    Shopify,
    RealtyIncome,
    Ebay,
}

pub fn is_supported(id: &str) -> bool {
    SUPPORTED_IDS.contains(&id)
}

pub fn configure_concept(concept: &mut crate::practice::PracticeConcept) {
    if is_supported(&concept.id) {
        concept.input_kind = "industry_case".into();
        concept.inputs.clear();
        concept.notes = "固定发行人历史披露案例：用已公布的经营数据理解指标，数值可回到原文对应页核对；不代表当前所选股票。".into();
    }
}

pub fn configure_knowledge(entry: &mut crate::knowledge::KnowledgeEntry) {
    if let Some(definition) = definition(&entry.id) {
        entry.formula = definition.formula.into();
        entry.signals = "练习使用一个已验证的发行人历史披露来核对原书公式；结果只描述指定实体和报告期，不是当前行情、任意股票查询或独立买卖信号。跨公司比较前必须统一币种、单位、期间、业务定义与审计边界。".into();
    }
}

fn definition(id: &str) -> Option<MetricDefinition> {
    let outside_audit = "Management discussion facts on PDF pages 57-58 are outside EY's audit opinion covering the financial statements";
    let shopify_unaudited = "Issuer earnings release metric; no audit assurance is asserted for MRR, GMV or non-GAAP free cash flow";
    Some(match id {
        "bank_nim" | "book_bank_nim" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Bank",
            formula: "NIM=净利息收入/平均生息资产",
            inputs: &["net_interest_income", "average_earning_assets"],
            audited: false,
            audit_boundary: outside_audit,
            value_key: "net_interest_margin",
            value_unit: "fraction",
            note: "平均生息资产与净利息收入均来自 Ping An Bank 的同一 FY2024 管理层讨论表。",
        },
        "book_bank_cost_income" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Bank",
            formula: "成本收入比=经营费用/营业收入",
            inputs: &["operating_expenses", "operating_income"],
            audited: false,
            audit_boundary: outside_audit,
            value_key: "cost_income_ratio",
            value_unit: "fraction",
            note: "经营费用使用表内一般及行政费用的绝对金额，与同表营业收入相除。",
        },
        "book_bank_npl_ratio" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Bank",
            formula: "不良贷款率=不良贷款/贷款总额（不含利息）",
            inputs: &["nonperforming_loans", "gross_loans"],
            audited: false,
            audit_boundary: outside_audit,
            value_key: "nonperforming_loan_ratio",
            value_unit: "fraction",
            note: "分母是客户贷款及垫款总额且不含应计利息。",
        },
        "book_insurance_solvency_ratio" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Property & Casualty Insurance Company of China, Ltd.",
            formula: "偿付能力充足率=实际资本/最低资本",
            inputs: &["available_capital", "required_capital"],
            audited: true,
            audit_boundary: "Note 49(7), PDF page 336, is inside the audited financial statements covered by EY's opinion",
            value_key: "solvency_adequacy_ratio",
            value_unit: "fraction",
            note: "原书的可用资本/监管资本要求在该披露中映射为实际资本/最低资本，仅适用于 Ping An P&C。",
        },
        "book_saas_arr" => MetricDefinition {
            case: CaseKey::Shopify,
            metric_entity: "Shopify Inc. and its consolidated subsidiaries",
            formula: "ARR=期末MRR×12",
            inputs: &["monthly_recurring_revenue"],
            audited: false,
            audit_boundary: shopify_unaudited,
            value_key: "annualized_recurring_revenue_run_rate",
            value_unit: "USD millions annualized",
            note: "ARR 是 2024-12-31 MRR 的年化运行率，不是已报告的年度订阅收入。",
        },
        "book_saas_rule_of_40" => MetricDefinition {
            case: CaseKey::Shopify,
            metric_entity: "Shopify Inc. and its consolidated subsidiaries",
            formula: "Rule of 40=(2024收入/2023收入-1)+(2024自由现金流/2024收入)",
            inputs: &["revenue_2024", "revenue_2023", "free_cash_flow"],
            audited: false,
            audit_boundary: shopify_unaudited,
            value_key: "rule_of_40",
            value_unit: "fraction",
            note: "使用原始披露值精确推导，而不是把公告中 26% 与 18% 的整数展示值相加。",
        },
        "book_platform_gmv" => MetricDefinition {
            case: CaseKey::Shopify,
            metric_entity: "Shopify Inc. and its consolidated subsidiaries",
            formula: "GMV=报告期平台成交总额",
            inputs: &["gross_merchandise_value"],
            audited: false,
            audit_boundary: shopify_unaudited,
            value_key: "gross_merchandise_value",
            value_unit: "USD millions",
            note: "GMV 按发行人定义扣除退款，并包含运费、关税与增值税。",
        },
        "book_platform_take_rate" => MetricDefinition {
            case: CaseKey::Ebay,
            metric_entity: "eBay Inc. Marketplace 平台",
            formula: "Take Rate=平台净收入/GMV",
            inputs: &["platform_revenue", "gross_merchandise_value"],
            audited: false,
            audit_boundary: "年报第43–44页的管理层运营指标；不把财务报表审计意见泛化到GMV与Take Rate。",
            value_key: "platform_take_rate",
            value_unit: "fraction",
            note: "eBay 原文明确将 Take Rate 定义为平台净收入/GMV：收入包括市场服务与广告，GMV包含运费和税款。用披露金额复算约13.7718%，原文展示为舍入后的13.77%；不是纯交易佣金率。",
        },
        "book_reit_occupancy" => MetricDefinition {
            case: CaseKey::RealtyIncome,
            metric_entity: "Realty Income Corporation consolidated portfolio",
            formula: "面积入住率=已出租面积/可出租总面积",
            inputs: &["leased_area", "lettable_area"],
            audited: false,
            audit_boundary: "The issuer labels the supplemental operating information unaudited",
            value_key: "occupied_area_ratio",
            value_unit: "fraction",
            note: "使用平方英尺面积比 335,777,818/339,361,416，不使用按物业数量计算的 98.7%。",
        },
        _ => return None,
    })
}

fn cases() -> Result<&'static IndustryCases, String> {
    static CASES: OnceLock<Result<IndustryCases, String>> = OnceLock::new();
    match CASES.get_or_init(|| {
        let cases: IndustryCases =
            serde_json::from_str(include_str!("../docs/book/industry_cases.json"))
                .map_err(|error| format!("invalid embedded industry cases: {error}"))?;
        validate_cases(&cases)?;
        Ok(cases)
    }) {
        Ok(cases) => Ok(cases),
        Err(error) => Err(error.clone()),
    }
}

fn validate_cases(cases: &IndustryCases) -> Result<(), String> {
    for (case, expected) in [
        (&cases.ebay, (EBAY_URL, EBAY_SHA256, EBAY_BYTES, "EBAY")),
        (
            &cases.ping_an,
            (PING_AN_URL, PING_AN_SHA256, PING_AN_BYTES, "2318.HK"),
        ),
        (
            &cases.shopify,
            (SHOPIFY_URL, SHOPIFY_SHA256, SHOPIFY_BYTES, "SHOP"),
        ),
        (
            &cases.realty_income,
            (
                REALTY_INCOME_URL,
                REALTY_INCOME_SHA256,
                REALTY_INCOME_BYTES,
                "O",
            ),
        ),
    ] {
        if case.url != expected.0
            || case.sha256 != expected.1
            || case.bytes != expected.2
            || case.ticker != expected.3
            || case.case_id.trim().is_empty()
            || case.reporting_entity.trim().is_empty()
            || case.period_start.trim().is_empty()
            || case.period_end.trim().is_empty()
        {
            return Err("embedded industry case identity or source metadata changed".into());
        }
        for fact in case.facts.values() {
            if !fact.value.is_finite()
                || fact.value < 0.0
                || fact.page == 0
                || fact.unit.trim().is_empty()
                || fact.label.trim().is_empty()
            {
                return Err("embedded industry fact is invalid".into());
            }
        }
    }
    for id in SUPPORTED_IDS {
        let definition = definition(id).ok_or_else(|| format!("missing definition: {id}"))?;
        let case = select_case(cases, definition.case);
        for input in definition.inputs {
            if !case.facts.contains_key(*input) {
                return Err(format!("missing industry fact {input} for {id}"));
            }
        }
    }
    Ok(())
}

fn select_case(cases: &IndustryCases, key: CaseKey) -> &IndustryCase {
    match key {
        CaseKey::PingAn => &cases.ping_an,
        CaseKey::Shopify => &cases.shopify,
        CaseKey::RealtyIncome => &cases.realty_income,
        CaseKey::Ebay => &cases.ebay,
    }
}

fn select_source(registry: &IndustrySourceRegistry, key: CaseKey) -> &IndustrySourceConfig {
    match key {
        CaseKey::PingAn => &registry.ping_an,
        CaseKey::Shopify => &registry.shopify,
        CaseKey::RealtyIncome => &registry.realty_income,
        CaseKey::Ebay => &registry.ebay,
    }
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

async fn verified_source(
    cache_dir: &Path,
    expected: &IndustrySourceConfig,
) -> Result<&'static str, String> {
    if expected.bytes == 0 || expected.bytes > 20_000_000 || expected.sha256.len() != 64 {
        return Err("industry source expectation is invalid".into());
    }
    let _guard = SOURCE_LOCK.lock().await;
    let directory = cache_dir.join("industry-sources");
    let cached = directory.join(format!("{}.pdf", expected.sha256));
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        if bytes.len() == expected.bytes && fingerprint(&bytes) == expected.sha256 {
            return Ok("verified_immutable_cache");
        }
        tokio::fs::remove_file(&cached)
            .await
            .map_err(|error| format!("corrupt industry source cache removal failed: {error}"))?;
    }
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("industry source client failed: {error}"))?
        .get(&expected.url)
        .send()
        .await
        .map_err(|error| format!("industry source request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("industry source returned an error: {error}"))?;
    if response
        .content_length()
        .is_some_and(|size| size != expected.bytes as u64)
    {
        return Err("industry source Content-Length differs from the reviewed document".into());
    }
    let mut bytes = Vec::with_capacity(expected.bytes);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("industry source body failed: {error}"))?;
        if bytes.len().saturating_add(chunk.len()) > expected.bytes {
            return Err("industry source body exceeds the reviewed document size".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != expected.bytes || fingerprint(&bytes) != expected.sha256 {
        return Err("downloaded industry source is incomplete or has a different SHA-256".into());
    }
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| format!("industry source cache directory failed: {error}"))?;
    tokio::fs::write(&cached, &bytes)
        .await
        .map_err(|error| format!("industry source cache write failed: {error}"))?;
    Ok("verified_then_cached")
}

fn ratio(numerator: f64, denominator: f64) -> Result<f64, String> {
    let value = numerator / denominator;
    if denominator == 0.0 || !value.is_finite() {
        Err("industry calculation has a zero or invalid denominator".into())
    } else {
        Ok(value)
    }
}

fn calculate(id: &str, case: &IndustryCase) -> Result<f64, String> {
    let n = |key: &str| {
        case.facts
            .get(key)
            .map(|fact| fact.value)
            .ok_or_else(|| format!("missing industry fact {key}"))
    };
    match id {
        "bank_nim" | "book_bank_nim" => {
            ratio(n("net_interest_income")?, n("average_earning_assets")?)
        }
        "book_bank_cost_income" => ratio(n("operating_expenses")?, n("operating_income")?),
        "book_bank_npl_ratio" => ratio(n("nonperforming_loans")?, n("gross_loans")?),
        "book_insurance_solvency_ratio" => ratio(n("available_capital")?, n("required_capital")?),
        "book_saas_arr" => Ok(n("monthly_recurring_revenue")? * 12.0),
        "book_saas_rule_of_40" => Ok(ratio(
            n("revenue_2024")? - n("revenue_2023")?,
            n("revenue_2023")?,
        )? + ratio(n("free_cash_flow")?, n("revenue_2024")?)?),
        "book_platform_gmv" => n("gross_merchandise_value"),
        "book_platform_take_rate" => ratio(n("platform_revenue")?, n("gross_merchandise_value")?),
        "book_reit_occupancy" => ratio(n("leased_area")?, n("lettable_area")?),
        _ => Err(format!("unsupported industry case concept: {id}")),
    }
}

fn formula_symbol_mapping(id: &str) -> Value {
    match id {
        "bank_nim" | "book_bank_nim" => json!({
            "net_interest_income": "net_interest_income",
            "average_earning_assets": "average_earning_assets"
        }),
        "book_bank_cost_income" => json!({
            "operating_expenses": "operating_expenses",
            "operating_income": "operating_income"
        }),
        "book_bank_npl_ratio" => json!({
            "nonperforming_loans": "nonperforming_loans",
            "gross_loans": "gross_loans"
        }),
        "book_insurance_solvency_ratio" => json!({
            "available_capital": "available_capital (issuer label: actual capital)",
            "required_capital": "required_capital (issuer label: minimum capital)"
        }),
        "book_saas_arr" => json!({
            "monthly_recurring_revenue": "monthly_recurring_revenue"
        }),
        "book_saas_rule_of_40" => json!({
            "revenue_growth_rate": "revenue_2024 / revenue_2023 - 1",
            "fcf_margin": "free_cash_flow / revenue_2024"
        }),
        "book_platform_gmv" => json!({
            "gross_merchandise_value": "gross_merchandise_value"
        }),
        "book_platform_take_rate" => json!({
            "platform_revenue": "platform_revenue（eBay 原文 net revenues，包含市场服务与广告收入）",
            "gross_merchandise_value": "gross_merchandise_value"
        }),
        "book_reit_occupancy" => json!({
            "leased_area": "leased_area (issuer label: occupied square feet)",
            "lettable_area": "lettable_area (issuer label: total portfolio square feet)"
        }),
        _ => json!({}),
    }
}

pub async fn evaluate(
    id: &str,
    cache_dir: &Path,
    registry: &IndustrySourceRegistry,
) -> Result<Value, String> {
    let definition =
        definition(id).ok_or_else(|| format!("unsupported industry case concept: {id}"))?;
    let cases = cases()?;
    let case = select_case(cases, definition.case);
    let source = select_source(registry, definition.case);
    let cache_status = verified_source(cache_dir, source).await?;
    let value = calculate(id, case)?;
    let mut reported_facts = Vec::new();
    let mut field_provenance = Map::new();
    let mut pdf_pages = Vec::new();
    for key in definition.inputs {
        let fact = &case.facts[*key];
        pdf_pages.push(fact.page);
        reported_facts.push(json!({
            "key": key,
            "label": fact.label,
            "value": fact.value,
            "unit": fact.unit,
            "pdf_page": fact.page
        }));
        field_provenance.insert(
            (*key).into(),
            json!({
                "kind": "reported",
                "pdf_page": fact.page,
                "note": format!("Issuer-reported as {}", fact.label)
            }),
        );
    }
    field_provenance.insert(
        definition.value_key.into(),
        json!({
            "kind": "derived",
            "note": format!("Calculated from reviewed reported facts using {}", definition.formula)
        }),
    );
    pdf_pages.sort_unstable();
    pdf_pages.dedup();
    Ok(json!({
        "concept_id": id,
        "input_kind": "industry_case",
        "provenance": "verified_original_issuer_disclosure",
        "status": "computed",
        "reason": Value::Null,
        "values": {definition.value_key: value},
        "units": {definition.value_key: definition.value_unit},
        "series": [],
        "notes": [
            "这是固定发行人与固定报告期的历史披露案例，不是当前行情、全市场行业数据库或任意股票查询。",
            definition.note
        ],
        "inputs": {},
        "bars": [],
        "industry_case": {
            "case_id": case.case_id,
            "issuer": {
                "name": case.issuer,
                "ticker": case.ticker,
                "alternate_tickers": case.alternate_tickers,
                "reporting_entity": case.reporting_entity,
                "metric_entity": definition.metric_entity
            },
            "period": {
                "label": case.period_label,
                "start": case.period_start,
                "end": case.period_end
            },
            "published": case.published,
            "audited": definition.audited,
            "audit_boundary": definition.audit_boundary,
            "status": case.source_status,
            "source_kind": case.source_kind,
            "url": case.url,
            "pdf_pages": pdf_pages,
            "sha256": case.sha256,
            "bytes": case.bytes,
            "verification": {
                "status": cache_status,
                "verified_at": chrono::Utc::now(),
                "requested_url": source.url,
                "matched_sha256": source.sha256,
                "matched_bytes": source.bytes
            }
        },
        "industry_facts": {
            "currency": case.currency,
            "scale": case.scale,
            "reported_facts": reported_facts,
            "calculation": {
                "formula": definition.formula,
                "result_key": definition.value_key,
                "operands": definition.inputs,
                "symbol_mapping": formula_symbol_mapping(id)
            },
            "field_provenance": field_provenance,
            "definitions": {
                "case_boundary": "Fixed issuer and reporting period; never generalized from the caller's selected symbol or dataset.",
                "metric": definition.note
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_registry(root: &Path) -> IndustrySourceRegistry {
        let mut registry = IndustrySourceRegistry::default();
        let directory = root.join("industry-sources");
        std::fs::create_dir_all(&directory).unwrap();
        for source in [
            &mut registry.ping_an,
            &mut registry.shopify,
            &mut registry.realty_income,
            &mut registry.ebay,
        ] {
            let bytes = format!("%PDF fixture {}", source.url).into_bytes();
            source.bytes = bytes.len();
            source.sha256 = fingerprint(&bytes);
            std::fs::write(directory.join(format!("{}.pdf", source.sha256)), bytes).unwrap();
        }
        registry
    }

    #[tokio::test]
    async fn supported_cases_match_independent_disclosure_values() {
        let root = std::env::temp_dir().join(format!("axiom-industry-{}", uuid::Uuid::new_v4()));
        let registry = seeded_registry(&root);
        let expected = [
            ("bank_nim", "net_interest_margin", 93_427.0 / 4_994_494.0),
            (
                "book_bank_nim",
                "net_interest_margin",
                93_427.0 / 4_994_494.0,
            ),
            (
                "book_bank_cost_income",
                "cost_income_ratio",
                40_582.0 / 146_695.0,
            ),
            (
                "book_bank_npl_ratio",
                "nonperforming_loan_ratio",
                35_738.0 / 3_374_103.0,
            ),
            (
                "book_insurance_solvency_ratio",
                "solvency_adequacy_ratio",
                138_649.0 / 67_536.0,
            ),
            (
                "book_saas_arr",
                "annualized_recurring_revenue_run_rate",
                2_136.0,
            ),
            (
                "book_saas_rule_of_40",
                "rule_of_40",
                (8_880.0 / 7_060.0 - 1.0) + 1_597.0 / 8_880.0,
            ),
            ("book_platform_gmv", "gross_merchandise_value", 292_275.0),
            (
                "book_platform_take_rate",
                "platform_take_rate",
                10_283.0 / 74_667.0,
            ),
            (
                "book_reit_occupancy",
                "occupied_area_ratio",
                335_777_818.0 / 339_361_416.0,
            ),
        ];
        for (id, key, literal) in expected {
            let result = evaluate(id, &root, &registry).await.unwrap();
            let actual = result["values"][key].as_f64().unwrap();
            assert!((actual - literal).abs() < 1e-12, "{id}: {actual}");
            assert_eq!(result["input_kind"], "industry_case");
            assert_eq!(
                result["industry_case"]["verification"]["status"],
                "verified_immutable_cache"
            );
            assert!(result["industry_facts"]["reported_facts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|fact| fact["pdf_page"].as_u64().unwrap() > 0));
            assert_eq!(
                result["industry_facts"]["field_provenance"][key]["kind"],
                "derived"
            );
        }
        let take_rate = evaluate("book_platform_take_rate", &root, &registry)
            .await
            .unwrap();
        assert_eq!(
            take_rate["industry_facts"]["calculation"]["symbol_mapping"]["platform_revenue"],
            "platform_revenue（eBay 原文 net revenues，包含市场服务与广告收入）"
        );
        assert_eq!(take_rate["industry_case"]["issuer"]["ticker"], "EBAY");
        assert_eq!(take_rate["industry_case"]["pdf_pages"], json!([44]));
        assert_eq!(
            evaluate("book_reit_ffo", &root, &registry)
                .await
                .unwrap_err(),
            "unsupported industry case concept: book_reit_ffo"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn registry_and_embedded_cases_are_exact_and_complete() {
        let registry = IndustrySourceRegistry::default();
        assert_eq!(registry.ping_an.bytes, PING_AN_BYTES);
        assert_eq!(registry.shopify.sha256, SHOPIFY_SHA256);
        assert_eq!(registry.realty_income.url, REALTY_INCOME_URL);
        assert!(cases().is_ok());
        assert!(SUPPORTED_IDS
            .iter()
            .all(|id| is_supported(id) && definition(id).is_some()));
        assert!(!is_supported("book_reit_ffo"));
        assert!(ratio(1.0, 0.0).is_err());
        assert!(calculate("unknown", &cases().unwrap().ping_an).is_err());

        let mut invalid_identity = cases().unwrap().clone();
        invalid_identity.shopify.ticker.clear();
        assert!(validate_cases(&invalid_identity).is_err());
        let mut invalid_fact = cases().unwrap().clone();
        invalid_fact
            .realty_income
            .facts
            .get_mut("leased_area")
            .unwrap()
            .page = 0;
        assert!(validate_cases(&invalid_fact).is_err());
        let mut missing_fact = cases().unwrap().clone();
        missing_fact.ping_an.facts.remove("net_interest_income");
        assert!(validate_cases(&missing_fact).is_err());
    }

    #[test]
    fn catalog_and_knowledge_are_reconfigured_at_the_public_seams() {
        let mut concept = crate::book::catalog()
            .into_iter()
            .find(|concept| concept.id == "book_bank_nim")
            .unwrap();
        assert!(!concept.inputs.is_empty());
        configure_concept(&mut concept);
        assert_eq!(concept.input_kind, "industry_case");
        assert!(concept.inputs.is_empty());
        assert!(concept.notes.contains("固定发行人"));

        let mut entry = crate::book::entries()
            .into_iter()
            .find(|entry| entry.id == "book_bank_nim")
            .unwrap();
        configure_knowledge(&mut entry);
        assert_eq!(entry.formula, "NIM=净利息收入/平均生息资产");
        assert!(entry.signals.contains("已验证的发行人历史披露"));
    }

    #[tokio::test]
    async fn source_download_recovers_corrupt_cache_and_rejects_drift() {
        use axum::{body::Body, http::Response, routing::get, Router};

        static PDF: &[u8] = b"%PDF-1.4\nindustry source fixture\n%%EOF\n";
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/ok.pdf", get(|| async { PDF }))
                    .route(
                        "/stream.pdf",
                        get(|| async {
                            Response::new(Body::from_stream(futures::stream::iter([
                                Ok::<_, std::io::Error>(&PDF[..20]),
                                Ok::<_, std::io::Error>(&PDF[20..]),
                            ])))
                        }),
                    )
                    .route(
                        "/missing.pdf",
                        get(|| async { (axum::http::StatusCode::NOT_FOUND, "missing") }),
                    ),
            )
            .await
            .unwrap();
        });
        let root =
            std::env::temp_dir().join(format!("axiom-industry-source-{}", uuid::Uuid::new_v4()));
        let directory = root.join("industry-sources");
        tokio::fs::create_dir_all(&directory).await.unwrap();
        let source = IndustrySourceConfig {
            url: format!("http://{address}/ok.pdf"),
            sha256: fingerprint(PDF),
            bytes: PDF.len(),
        };
        tokio::fs::write(directory.join(format!("{}.pdf", source.sha256)), b"corrupt")
            .await
            .unwrap();
        assert_eq!(
            verified_source(&root, &source).await.unwrap(),
            "verified_then_cached"
        );
        assert_eq!(
            tokio::fs::read(directory.join(format!("{}.pdf", source.sha256)))
                .await
                .unwrap(),
            PDF
        );

        let invalid = IndustrySourceConfig {
            bytes: 0,
            ..source.clone()
        };
        assert!(verified_source(&root, &invalid).await.is_err());
        tokio::fs::remove_file(directory.join(format!("{}.pdf", source.sha256)))
            .await
            .unwrap();
        let wrong_length = IndustrySourceConfig {
            bytes: PDF.len() + 1,
            ..source.clone()
        };
        assert!(verified_source(&root, &wrong_length)
            .await
            .unwrap_err()
            .contains("Content-Length differs"));
        let wrong_hash = IndustrySourceConfig {
            sha256: "0".repeat(64),
            ..source.clone()
        };
        assert!(verified_source(&root, &wrong_hash)
            .await
            .unwrap_err()
            .contains("different SHA-256"));
        let too_long = IndustrySourceConfig {
            url: format!("http://{address}/stream.pdf"),
            bytes: PDF.len() - 1,
            ..source.clone()
        };
        assert!(verified_source(&root, &too_long)
            .await
            .unwrap_err()
            .contains("exceeds"));
        let missing = IndustrySourceConfig {
            url: format!("http://{address}/missing.pdf"),
            ..source
        };
        assert!(verified_source(&root, &missing)
            .await
            .unwrap_err()
            .contains("returned an error"));
        server.abort();
        let _ = tokio::fs::remove_dir_all(root).await;
    }
}
