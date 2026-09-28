//! Verified issuer-published historical financial-statement case.
//!
//! Facts are embedded for deterministic calculations, but the server releases a
//! result only after the cited PDF matches its exact size and SHA-256.

use futures::StreamExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::OnceLock};
use tokio::sync::Mutex;

pub const OFFICIAL_URL: &str =
    "https://www.apple.com/newsroom/pdfs/fy2025-q4/FY25_Q4_Consolidated_Financial_Statements.pdf";
pub const OFFICIAL_SHA256: &str =
    "43e7f0730b3cce0fc37301a2f43c29712bbde6ab299d97c6df345fd0c754508a";
pub const OFFICIAL_BYTES: usize = 4_919_649;
static SOURCE_LOCK: Mutex<()> = Mutex::const_new(());

pub const SUPPORTED_IDS: &[&str] = &[
    "eps",
    "book_diluted_shares",
    "book_revenue",
    "book_gross_margin",
    "book_ebit_margin",
    "book_net_margin",
    "book_current_ratio",
    "book_quick_ratio",
    "book_cash_ratio",
    "book_debt_ratio",
    "book_de_ratio",
    "book_net_debt",
    "book_cfo",
    "book_capex",
    "book_fcf",
    "fcf",
    "book_cfo_income",
    "book_asset_turnover",
    "book_roa",
    "book_inventory_turnover",
    "book_receivable_turnover",
    "book_dpo",
    "accrual_ratio",
    "book_yoy",
    "book_ccc",
    "roe",
    "dupont",
    "book_roce",
];

#[derive(Clone, Debug)]
pub struct FilingSourceConfig {
    pub url: String,
    pub sha256: String,
    pub bytes: usize,
}

impl Default for FilingSourceConfig {
    fn default() -> Self {
        Self {
            url: OFFICIAL_URL.into(),
            sha256: OFFICIAL_SHA256.into(),
            bytes: OFFICIAL_BYTES,
        }
    }
}

pub fn is_supported(id: &str) -> bool {
    SUPPORTED_IDS.contains(&id)
}

pub fn configure_concept(concept: &mut crate::practice::PracticeConcept) {
    if is_supported(&concept.id) {
        concept.input_kind = "filing_case".into();
        concept.inputs.clear();
        concept.notes = "Apple FY2025 与 FY2024 历史案例：服务器验证 Apple 官方业绩公告 PDF 的完整字节数与 SHA-256 后，才使用逐页核对的 GAAP 事实计算；公告财务报表未经审计，且不是年度报告。".into();
    }
}

pub fn configure_knowledge(entry: &mut crate::knowledge::KnowledgeEntry) {
    if is_supported(&entry.id) {
        entry.signals = "练习用已验证的 Apple FY2025/FY2024 历史公告核对公式、平均数分母和会计口径；它不是当前行情、任意股票查询或独立买卖信号。跨公司或跨期比较前仍须统一会计口径与报告时点。".into();
        match entry.id.as_str() {
            "book_dpo" => {
                entry.formula = "DPO=平均应付账款/销售成本×报告期天数；52周财年使用364天".into()
            }
            "accrual_ratio" => entry.formula = "应计比率=(净利润-经营现金流)/平均总资产".into(),
            "book_yoy" => entry.formula = "营业收入同比=本期营业收入/上年同期营业收入-1".into(),
            "book_ccc" => {
                entry.formula =
                    "CCC=DIO+DSO-DPO；各周转天数使用报告期天数，DSO需注明赊销收入或代理口径".into()
            }
            _ => {}
        }
    }
}

fn data() -> Result<&'static Value, String> {
    static DATA: OnceLock<Result<Value, String>> = OnceLock::new();
    match DATA.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../docs/book/financial_cases.json"))
            .map_err(|error| format!("invalid embedded filing case: {error}"))?;
        validate_case(&value)?;
        Ok(value)
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(error.clone()),
    }
}

fn number(root: &Value, path: &[&str]) -> Result<f64, String> {
    let mut value = root;
    for key in path {
        value = &value[*key];
    }
    value
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(|| format!("missing or invalid filing fact {}", path.join(".")))
}

fn validate_case(case: &Value) -> Result<(), String> {
    if case["case_id"] != "apple_fy2025_q4_annual_gaap_press_release"
        || case["issuer"]["name"] != "Apple Inc."
        || case["issuer"]["ticker"] != "AAPL"
        || case["source"]["audited"] != false
        || case["source"]["url"] != OFFICIAL_URL
        || case["source"]["sha256"] != OFFICIAL_SHA256
        || case["source"]["bytes"] != OFFICIAL_BYTES
        || case["source"]["published"] != "2025-10-30"
    {
        return Err("embedded filing identity or source metadata changed".into());
    }
    for (section, years, fields, page) in [
        (
            "annual_income_statement",
            vec!["2025", "2024"],
            vec![
                "sales",
                "cogs",
                "grossprofit",
                "operatingincome",
                "netincome",
                "basic_eps",
                "diluted_eps",
                "weighted_basic_shares",
                "weighted_diluted_shares",
            ],
            1,
        ),
        (
            "balance_sheets",
            vec!["2025-09-27", "2024-09-28"],
            vec![
                "cash",
                "current_securities",
                "accounts_receivable",
                "vendor_nontrade_receivables",
                "inventory",
                "current_assets",
                "accounts_payable",
                "commercial_paper",
                "current_term_debt",
                "current_liabilities",
                "noncurrent_term_debt",
                "assets",
                "liabilities",
                "equity",
            ],
            2,
        ),
        (
            "annual_cash_flows",
            vec!["2025", "2024"],
            vec![
                "cfo",
                "ppe_capex_cash_outflow",
                "depreciation_and_amortization",
                "cash_dividends_outflow",
                "share_repurchases_outflow",
                "ending_cash",
            ],
            3,
        ),
    ] {
        for year in years {
            if case[section][year]["page"] != page {
                return Err("filing page mapping changed".into());
            }
            for field in &fields {
                number(case, &[section, year, field])?;
            }
        }
    }
    for year in ["2025", "2024"] {
        let income = &case["annual_income_statement"][year];
        if number(income, &["grossprofit"])?
            != number(income, &["sales"])? - number(income, &["cogs"])?
        {
            return Err("gross profit does not reconcile".into());
        }
    }
    if case["annual_cash_flows"]["2025"]["ending_cash"]
        != case["balance_sheets"]["2025-09-27"]["cash"]
        || case["annual_cash_flows"]["2024"]["ending_cash"]
            != case["balance_sheets"]["2024-09-28"]["cash"]
    {
        return Err("ending cash does not reconcile".into());
    }
    Ok(())
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

async fn verified_source(
    cache_dir: &Path,
    expected: &FilingSourceConfig,
) -> Result<&'static str, String> {
    if expected.bytes == 0 || expected.bytes > 8_000_000 || expected.sha256.len() != 64 {
        return Err("filing source expectation is invalid".into());
    }
    let _guard = SOURCE_LOCK.lock().await;
    let directory = cache_dir.join("filing-sources");
    let cached = directory.join(format!("{}.pdf", expected.sha256));
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        if bytes.len() != expected.bytes || fingerprint(&bytes) != expected.sha256 {
            tokio::fs::remove_file(&cached)
                .await
                .map_err(|error| format!("corrupt filing cache removal failed: {error}"))?;
        } else {
            return Ok("verified_immutable_cache");
        }
    }
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| format!("filing source client failed: {error}"))?
        .get(&expected.url)
        .send()
        .await
        .map_err(|error| format!("filing source request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("filing source returned an error: {error}"))?;
    if response
        .content_length()
        .is_some_and(|size| size != expected.bytes as u64)
    {
        return Err("filing source Content-Length differs from the checked document".into());
    }
    let mut bytes = Vec::with_capacity(expected.bytes);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("filing source body failed: {error}"))?;
        if bytes.len().saturating_add(chunk.len()) > expected.bytes {
            return Err("filing source body exceeds the checked document size".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != expected.bytes || fingerprint(&bytes) != expected.sha256 {
        return Err("downloaded filing source is incomplete or has a different SHA-256".into());
    }
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| format!("filing cache directory failed: {error}"))?;
    tokio::fs::write(&cached, &bytes)
        .await
        .map_err(|error| format!("filing cache write failed: {error}"))?;
    Ok("verified_then_cached")
}

fn ratio(numerator: f64, denominator: f64) -> Result<f64, String> {
    let value = numerator / denominator;
    if denominator == 0.0 || !value.is_finite() {
        Err("filing calculation has a zero or invalid denominator".into())
    } else {
        Ok(value)
    }
}

fn metric(id: &str, c: &Value) -> Result<(Value, Value), String> {
    let i = &c["annual_income_statement"]["2025"];
    let b = &c["balance_sheets"]["2025-09-27"];
    let bp = &c["balance_sheets"]["2024-09-28"];
    let cf = &c["annual_cash_flows"]["2025"];
    let n = |v: &Value, key: &str| number(v, &[key]);
    let avg_assets = (n(b, "assets")? + n(bp, "assets")?) / 2.0;
    let avg_equity = (n(b, "equity")? + n(bp, "equity")?) / 2.0;
    let avg_inventory = (n(b, "inventory")? + n(bp, "inventory")?) / 2.0;
    let avg_receivables = (n(b, "accounts_receivable")? + n(bp, "accounts_receivable")?) / 2.0;
    let avg_payables = (n(b, "accounts_payable")? + n(bp, "accounts_payable")?) / 2.0;
    let net_debt =
        n(b, "commercial_paper")? + n(b, "current_term_debt")? + n(b, "noncurrent_term_debt")?
            - n(b, "cash")?;
    let one = |key: &str, value: f64, unit: &str| Ok((json!({key:value}), json!({key:unit})));
    match id {
        "eps" => Ok((
            json!({"reported_basic_eps":n(i,"basic_eps")?,"reported_diluted_eps":n(i,"diluted_eps")?,"approx_basic_eps_cross_check":n(i,"netincome")?*1000.0/n(i,"weighted_basic_shares")?,"approx_diluted_eps_cross_check":n(i,"netincome")?*1000.0/n(i,"weighted_diluted_shares")?}),
            json!({"reported_basic_eps":"USD/share","reported_diluted_eps":"USD/share","approx_basic_eps_cross_check":"USD/share","approx_diluted_eps_cross_check":"USD/share"}),
        )),
        "book_diluted_shares" => Ok((
            json!({"weighted_diluted_shares":n(i,"weighted_diluted_shares")?,"reported_diluted_eps":n(i,"diluted_eps")?,"approx_diluted_eps_cross_check":n(i,"netincome")?*1000.0/n(i,"weighted_diluted_shares")?}),
            json!({"weighted_diluted_shares":"thousand shares","reported_diluted_eps":"USD/share","approx_diluted_eps_cross_check":"USD/share"}),
        )),
        "book_revenue" => one("revenue", n(i, "sales")?, "USD millions"),
        "book_gross_margin" => one(
            "gross_margin",
            ratio(n(i, "grossprofit")?, n(i, "sales")?)?,
            "fraction",
        ),
        "book_ebit_margin" => one(
            "ebit_margin",
            ratio(n(i, "operatingincome")?, n(i, "sales")?)?,
            "fraction",
        ),
        "book_net_margin" => one(
            "net_margin",
            ratio(n(i, "netincome")?, n(i, "sales")?)?,
            "fraction",
        ),
        "book_current_ratio" => one(
            "current_ratio",
            ratio(n(b, "current_assets")?, n(b, "current_liabilities")?)?,
            "multiple",
        ),
        "book_quick_ratio" => one(
            "quick_ratio",
            ratio(
                n(b, "cash")? + n(b, "current_securities")? + n(b, "accounts_receivable")?,
                n(b, "current_liabilities")?,
            )?,
            "multiple",
        ),
        "book_cash_ratio" => one(
            "cash_ratio",
            ratio(n(b, "cash")?, n(b, "current_liabilities")?)?,
            "multiple",
        ),
        "book_debt_ratio" => one(
            "debt_ratio",
            ratio(n(b, "liabilities")?, n(b, "assets")?)?,
            "fraction",
        ),
        "book_de_ratio" => one(
            "debt_to_equity",
            ratio(n(b, "liabilities")?, n(b, "equity")?)?,
            "multiple",
        ),
        "book_net_debt" => one("net_debt", net_debt, "USD millions"),
        "book_cfo" => one("operating_cash_flow", n(cf, "cfo")?, "USD millions"),
        "book_capex" => one(
            "capital_expenditure",
            n(cf, "ppe_capex_cash_outflow")?,
            "USD millions",
        ),
        "book_fcf" | "fcf" => one(
            "free_cash_flow",
            n(cf, "cfo")? - n(cf, "ppe_capex_cash_outflow")?,
            "USD millions",
        ),
        "book_cfo_income" => one(
            "cfo_to_net_income",
            ratio(n(cf, "cfo")?, n(i, "netincome")?)?,
            "multiple",
        ),
        "book_asset_turnover" => one(
            "asset_turnover",
            ratio(n(i, "sales")?, avg_assets)?,
            "multiple",
        ),
        "book_roa" => one(
            "return_on_average_assets",
            ratio(n(i, "netincome")?, avg_assets)?,
            "fraction",
        ),
        "book_inventory_turnover" => one(
            "inventory_turnover",
            ratio(n(i, "cogs")?, avg_inventory)?,
            "multiple",
        ),
        "book_receivable_turnover" => one(
            "receivable_turnover_proxy",
            ratio(n(i, "sales")?, avg_receivables)?,
            "multiple",
        ),
        "book_dpo" => one(
            "days_payable_outstanding",
            ratio(avg_payables, n(i, "cogs")?)? * 364.0,
            "days",
        ),
        "accrual_ratio" => one(
            "accrual_ratio",
            ratio(n(i, "netincome")? - n(cf, "cfo")?, avg_assets)?,
            "fraction",
        ),
        "book_yoy" => {
            let prior_sales = number(c, &["annual_income_statement", "2024", "sales"])?;
            one(
                "revenue_year_over_year",
                ratio(n(i, "sales")? - prior_sales, prior_sales)?,
                "fraction",
            )
        }
        "book_ccc" => {
            let dso = ratio(avg_receivables, n(i, "sales")?)? * 364.0;
            let dio = ratio(avg_inventory, n(i, "cogs")?)? * 364.0;
            let dpo = ratio(avg_payables, n(i, "cogs")?)? * 364.0;
            Ok((
                json!({"cash_conversion_cycle":dio+dso-dpo,"days_sales_outstanding_proxy":dso,"days_inventory_outstanding":dio,"days_payable_outstanding":dpo}),
                json!({"cash_conversion_cycle":"days","days_sales_outstanding_proxy":"days","days_inventory_outstanding":"days","days_payable_outstanding":"days"}),
            ))
        }
        "roe" => one(
            "return_on_average_equity",
            ratio(n(i, "netincome")?, avg_equity)?,
            "fraction",
        ),
        "dupont" => {
            let margin = ratio(n(i, "netincome")?, n(i, "sales")?)?;
            let turnover = ratio(n(i, "sales")?, avg_assets)?;
            let multiplier = ratio(avg_assets, avg_equity)?;
            Ok((
                json!({"net_margin":margin,"asset_turnover":turnover,"equity_multiplier":multiplier,"dupont_roe":margin*turnover*multiplier}),
                json!({"net_margin":"fraction","asset_turnover":"multiple","equity_multiplier":"multiple","dupont_roe":"fraction"}),
            ))
        }
        "book_roce" => one(
            "return_on_capital_employed",
            ratio(
                n(i, "operatingincome")?,
                n(b, "assets")? - n(b, "current_liabilities")?,
            )?,
            "fraction",
        ),
        _ => Err(format!("unsupported filing case concept: {id}")),
    }
}

pub async fn evaluate(
    id: &str,
    cache_dir: &Path,
    source: &FilingSourceConfig,
) -> Result<Value, String> {
    if !is_supported(id) {
        return Err(format!("unsupported filing case concept: {id}"));
    }
    let case = data()?;
    let cache_status = verified_source(cache_dir, source).await?;
    let (values, units) = metric(id, case)?;
    let mut notes = vec![
        "这是 Apple FY2025/FY2024 的固定历史案例，不是当前行情、全市场财务数据库或任意股票查询。"
            .to_string(),
        "来源是发行人 2025-10-30 发布的未经审计业绩公告财务报表，不是经审计年度报告。".to_string(),
    ];
    match id {
        "eps" => notes.push("公告披露的基本/稀释 EPS 为权威值；净利润除以加权股数仅作受四舍五入影响的近似交叉核对，未捏造优先股股利。".into()),
        "book_quick_ratio" => notes.push("速动资产仅含现金、流动有价证券和 trade accounts receivable；不含 vendor non-trade receivables。".into()),
        "book_cash_ratio" => notes.push("现金比率分子仅为现金及现金等价物，不含流动有价证券。".into()),
        "book_debt_ratio" | "book_de_ratio" => notes.push("这里的 debt 指总负债，不等同于有息借款。".into()),
        "book_receivable_turnover" => notes.push("公告没有单列赊销收入，因此以净销售额/平均 trade accounts receivable 作为代理值。".into()),
        "book_dpo" => notes.push("FY2025 为 52 周，天数使用 364；未按 365 天假设。".into()),
        "accrual_ratio" => notes.push("应计比率=(净利润-CFO)/平均总资产；平均总资产只使用 FY2024 与 FY2025 两个已披露期末值。接近零只描述该历史期间，不自动证明盈利质量。".into()),
        "book_yoy" => notes.push("同比明确使用 FY2025 营业收入相对 FY2024 营业收入；不是 EPS、季度收入或当前增长率。".into()),
        "book_ccc" => notes.push("CCC 使用 DIO + DSO代理 - DPO，全部按 FY2025 的 52周即364天；公告未披露赊销收入，DSO以净销售额代理。负CCC是公式结果，不按零截断，也不自动代表所有业务环节。".into()),
        "book_roce" => notes.push("资本占用采用 FY2025 期末总资产减期末流动负债，不冒充平均资本口径。".into()),
        _ => {}
    }
    Ok(
        json!({"concept_id":id,"input_kind":"filing_case","provenance":"verified_issuer_filing_case","status":"computed","reason":Value::Null,"values":values,"units":units,"series":[],"notes":notes,"inputs":{},"bars":[],
            "filing_case":{"case_id":case["case_id"],"issuer":case["issuer"]["name"],"ticker":case["issuer"]["ticker"],"scope":case["issuer"]["scope"],"period":{"start":case["annual_income_statement"]["2025"]["period_start"],"end":case["annual_income_statement"]["2025"]["period_end"],"fiscal_year":2025},"comparison_period":{"start":case["annual_income_statement"]["2024"]["period_start"],"end":case["annual_income_statement"]["2024"]["period_end"],"fiscal_year":2024},"published":case["source"]["published"],"audited":case["source"]["audited"],"status":case["source"]["status"],"source_kind":case["source"]["kind"],"url":case["source"]["url"],"sha256":case["source"]["sha256"],"bytes":case["source"]["bytes"],"pages":{"income_statement":1,"balance_sheet":2,"cash_flow":3},"verification":{"cache_status":cache_status,"verified_at":chrono::Utc::now(),"requested_url":source.url,"matched_sha256":source.sha256,"matched_bytes":source.bytes}},
            "facts":{"units":case["units"],"annual_income_statement":case["annual_income_statement"],"balance_sheets":case["balance_sheets"],"annual_cash_flows":case["annual_cash_flows"],"calculation_boundaries":case["calculation_boundaries"]}
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_supported_metrics_are_finite() {
        let case = data().unwrap();
        for id in SUPPORTED_IDS {
            let (values, units) = metric(id, case).unwrap();
            for (key, value) in values.as_object().unwrap() {
                assert!(value.as_f64().is_some_and(f64::is_finite), "{id} {key}");
                assert!(units[key].is_string());
            }
        }
        assert!(metric("unknown", case).is_err());
        assert!(ratio(1.0, 0.0).is_err());
    }
    #[tokio::test]
    async fn immutable_cache_requires_exact_fingerprint() {
        let root = std::env::temp_dir().join(format!("axiom-filing-{}", uuid::Uuid::new_v4()));
        let source = FilingSourceConfig {
            url: "http://127.0.0.1:1/unreachable".into(),
            sha256: fingerprint(b"fixture"),
            bytes: 7,
        };
        let directory = root.join("filing-sources");
        tokio::fs::create_dir_all(&directory).await.unwrap();
        let path = directory.join(format!("{}.pdf", source.sha256));
        tokio::fs::write(&path, b"fixture").await.unwrap();
        assert_eq!(
            verified_source(&root, &source).await.unwrap(),
            "verified_immutable_cache"
        );
        tokio::fs::write(&path, b"changed").await.unwrap();
        assert!(verified_source(&root, &source).await.is_err());
        let invalid = FilingSourceConfig {
            bytes: 0,
            ..source.clone()
        };
        assert!(verified_source(&root, &invalid).await.is_err());
        assert!(evaluate("unknown", &root, &source).await.is_err());
        let _ = tokio::fs::remove_dir_all(root).await;
    }
}
