//! Fixed, source-verified ownership and analyst disclosure practices.
//!
//! Every result is scoped to the named issuer/universe and historical period.
//! The transport bytes must match the reviewed source before embedded facts are used.

use futures::StreamExt;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::OnceLock, time::Duration};
use tokio::sync::Mutex;

pub const SUPPORTED_IDS: &[&str] = &[
    "buyback_rate",
    "holder_concentration",
    "insider_trading",
    "institution_holding",
    "restricted_shares",
    "share_pledge",
    "short_interest",
    "consensus",
    "earnings_surprise",
    "forecast_dispersion",
    "revision",
    "target_upside",
];

const MOUTAI_URL: &str = "https://static.cninfo.com.cn/finalpage/2026-04-17/1225114741.PDF";
const MOUTAI_SHA256: &str = "474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288";
const MOUTAI_BYTES: usize = 1_082_847;
const CBOE_SHORT_INTEREST_URL: &str = "https://cdn.cboe.com/resources/us/equities/market-statistics/short-interest/Bats_Listed_Short_Interest-finra-20261002.csv";
const CBOE_SHORT_INTEREST_SHA256: &str =
    "564c843210b40a596568deb62da27c10e5627b00754b4372197b7ea90942763e";
const CBOE_SHORT_INTEREST_BYTES: usize = 127_080;
const AIRBUS_CONSENSUS_URL: &str = "https://mediaassets.airbus.com/pm_38_787_787398-089y4vnyo3.pdf";
const AIRBUS_CONSENSUS_SHA256: &str =
    "6152126d3b4b46b3ffad7c68dc76a6d6ec40908f28b2af85bf514a2ba62e6e9b";
const AIRBUS_CONSENSUS_BYTES: usize = 81_415;
const AIRBUS_FY_RESULTS_URL: &str = "https://www.afm.nl/downloadregisterfile.aspx?type=openbaarmaking-voorwetenschap&enc=7Rpj0BBaMD5lzfwUlyQ9TIe+XbKYS1JE7+GVfXY2PxNqAT3NVNKp3QCQcI2WUb5gS/PYmhvg5EA+E7FpgiengZj0NLAMUDbP/oPnUtKH27VA0/CZGu7E4hLGdYYwwWpdTyp/cuqK9u2A7er37RJifDgVQS3W6HmCKGWO1CEeZNXO0g0IROVIr+z+5qosB2fxjW2qofVbttWzylRmw6K7ZNM1O+nSGFVNazbTDT6ekO3vaajCoWPbTc7g3zIvR0ilTPg8vjdE1aEQ10Qyj60J7wrJWgG+ROEOcLEujFwVnZc4hQg9b/oBAl+w6cUoq0F56qLbopyLMcJnwQKx89Mczg==";
const AIRBUS_FY_RESULTS_SHA256: &str =
    "21f849df5646b6b57e006827581768016e3b0995ec8d95d096f573a7886f8610";
const AIRBUS_FY_RESULTS_BYTES: usize = 296_359;
const MWB_URL: &str =
    "https://downloads.research-hub.de/2025%2002%2024%20Airbus%20Update___kh66mlcm.pdf";
const MWB_SHA256: &str = "2819c2970703eaf90c22f57db151915bbd8a03dd259eb8478848a6fe25643471";
const MWB_BYTES: usize = 991_184;
const SIGNIFY_CONSENSUS_URL: &str =
    "https://www.signify.com/static/2025/20260114-signify-analyst-consensus-pre-q4-2025.pdf";
const SIGNIFY_CONSENSUS_SHA256: &str =
    "c2ebe6999e54928dc1f2041a0194e6a856e900b184147ad3974ac51e68842bb3";
const SIGNIFY_CONSENSUS_BYTES: usize = 231_176;

static SOURCE_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnershipSourceConfig {
    pub url: String,
    pub sha256: String,
    pub bytes: usize,
    pub format: String,
}

impl OwnershipSourceConfig {
    fn new(url: &str, sha256: &str, bytes: usize, format: &str) -> Self {
        Self {
            url: url.into(),
            sha256: sha256.into(),
            bytes,
            format: format.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnershipSourceRegistry {
    pub moutai: OwnershipSourceConfig,
    pub cboe_short_interest: OwnershipSourceConfig,
    pub airbus_consensus: OwnershipSourceConfig,
    pub airbus_fy_results: OwnershipSourceConfig,
    pub mwb_report: OwnershipSourceConfig,
    pub signify_consensus: OwnershipSourceConfig,
}

impl Default for OwnershipSourceRegistry {
    fn default() -> Self {
        Self {
            moutai: OwnershipSourceConfig::new(MOUTAI_URL, MOUTAI_SHA256, MOUTAI_BYTES, "PDF"),
            cboe_short_interest: OwnershipSourceConfig::new(
                CBOE_SHORT_INTEREST_URL,
                CBOE_SHORT_INTEREST_SHA256,
                CBOE_SHORT_INTEREST_BYTES,
                "CSV",
            ),
            airbus_consensus: OwnershipSourceConfig::new(
                AIRBUS_CONSENSUS_URL,
                AIRBUS_CONSENSUS_SHA256,
                AIRBUS_CONSENSUS_BYTES,
                "PDF",
            ),
            airbus_fy_results: OwnershipSourceConfig::new(
                AIRBUS_FY_RESULTS_URL,
                AIRBUS_FY_RESULTS_SHA256,
                AIRBUS_FY_RESULTS_BYTES,
                "PDF",
            ),
            mwb_report: OwnershipSourceConfig::new(MWB_URL, MWB_SHA256, MWB_BYTES, "PDF"),
            signify_consensus: OwnershipSourceConfig::new(
                SIGNIFY_CONSENSUS_URL,
                SIGNIFY_CONSENSUS_SHA256,
                SIGNIFY_CONSENSUS_BYTES,
                "PDF",
            ),
        }
    }
}

#[derive(Clone, Copy)]
struct Definition {
    case_key: &'static str,
    formula: &'static str,
    inputs: &'static [&'static str],
    sources: &'static [&'static str],
    note: &'static str,
}

pub fn is_supported(id: &str) -> bool {
    SUPPORTED_IDS.contains(&id)
}

pub fn fixed_symbol(id: &str) -> &'static str {
    match id {
        "short_interest" => "ARKW",
        "forecast_dispersion" => "LIGHT.AS",
        "consensus" | "earnings_surprise" | "revision" | "target_upside" => "AIR.PA",
        "buyback_rate"
        | "holder_concentration"
        | "institution_holding"
        | "restricted_shares"
        | "share_pledge"
        | "insider_trading" => "600519",
        _ => "",
    }
}

pub fn configure_concept(concept: &mut crate::practice::PracticeConcept) {
    if is_supported(&concept.id) {
        concept.input_kind = "industry_case".into();
        concept.inputs.clear();
        concept.notes = "固定历史披露练习：先逐字节验证监管机构、交易所、发行人或具名分析机构原文，再按文中实体、日期和分母复算；不代表当前股票或全市场。".into();
    }
}

pub fn configure_knowledge(entry: &mut crate::knowledge::KnowledgeEntry) {
    if let Some(def) = definition(&entry.id) {
        entry.formula = def.formula.into();
        entry.signals = "本练习只解释具名历史样本；来源、报告期、发布日期、单位和分母随结果返回，不能外推到当前市场。".into();
        entry.pitfalls = def.note.into();
    }
}

fn definition(id: &str) -> Option<Definition> {
    let moutai = &["moutai_annual_report"] as &'static [&'static str];
    Some(match id {
        "buyback_rate" => Definition { case_key: "moutai", formula: "报告期实际回购率=(第一期实际回购股份+第二期实际回购股份)/期初总股本", inputs: &["first_program_repurchased_shares", "second_program_repurchased_shares", "opening_total_shares"], sources: moutai, note: "分子采用年报第52页列示的两期实际已回购数量，不以注销数量代替回购活动；两期用途均为注销并减少注册资本。" },
        "holder_concentration" => Definition { case_key: "moutai", formula: "前十名股东持股比例=前十名股东披露持股数之和/期末总股本；列示HHI只覆盖这十个账户", inputs: &["holder_1_shares","holder_2_shares","holder_3_shares","holder_4_shares","holder_5_shares","holder_6_shares","holder_7_shares","holder_8_shares","holder_9_shares","holder_10_shares","closing_total_shares"], sources: moutai, note: "香港中央结算账户是名义持有人；前十名账户集中度不等同最终受益所有人集中度。" },
        "institution_holding" => Definition { case_key: "moutai", formula: "前十名表内四个具名指数基金账户持股下限=四账户持股数之和/期末总股本", inputs: &["holder_5_shares","holder_7_shares","holder_9_shares","holder_10_shares","closing_total_shares"], sources: moutai, note: "分子只覆盖前十名股东表中明确命名的四个指数基金账户，是可审计的机构持股下限，不是全部机构持股率。" },
        "restricted_shares" => Definition { case_key: "moutai", formula: "期末限售股份余额=期末总股本-期末无限售条件流通股份；余额比例=余额/期末总股本", inputs: &["closing_total_shares","unrestricted_shares"], sources: moutai, note: "期末限售余额为零不代表未来不会出现锁定或解禁安排；无限售股份也不等同自由流通股。" },
        "share_pledge" => Definition { case_key: "moutai", formula: "控股股东质押比例=控股股东表内质押、标记或冻结数量/控股股东期末持股数", inputs: &["controlling_holder_pledged_shares","controlling_holder_shares"], sources: moutai, note: "分母是控股股东持股，不是公司总股本；只描述报告期末表内状态。" },
        "insider_trading" => Definition { case_key: "moutai", formula: "控股股东公开增持率=报告期集中竞价增持股份/(期末持股-本期增持股份)", inputs: &["controlling_holder_acquired_shares","controlling_holder_shares"], sources: moutai, note: "这是年报披露的控股股东集中竞价增持计划完成结果，不包含期权行权，也不冒充董事、高管全体交易汇总。" },
        "short_interest" => Definition { case_key: "cboe_short_interest", formula: "空仓占自由流通股比例=全市场汇总空仓股数/自由流通股；本固定实践因来源未提供自由流通股分母，不计算该比例。可验证补充指标：回补天数=Cboe发布的该证券合并市场淡仓股数/该结算周期日均成交量", inputs: &["aggregate_short_shares","average_daily_share_volume","reported_days_to_cover"], sources: &["cboe_consolidated_short_interest"], note: "Cboe官方逐证券报告列示ARKW在2026-09-15结算日的合并市场淡仓股数和回补天数。该文件没有自由流通股分母，因此本练习只完成绝对淡仓数和回补天数，明确不计算或声称short interest占float比例。" },
        "consensus" => Definition { case_key: "airbus", formula: "一致预期=20名卖方分析师对Airbus Q4 2025报告口径EPS的汇总值", inputs: &["q4_consensus_reported_eps","q4_consensus_analyst_count"], sources: &["airbus_consensus"], note: "Airbus明确说明该一致预期由第三方分析师形成，Airbus不背书；PDF只给汇总值，不给个体预测。" },
        "earnings_surprise" => Definition { case_key: "airbus", formula: "业绩超预期=(Q4 2025报告EPS-Q4事前一致预期EPS)/|Q4事前一致预期EPS|", inputs: &["q4_consensus_reported_eps","q4_2025_reported_eps"], sources: &["airbus_consensus","airbus_fy_results"], note: "一致预期PDF说明20名分析师预测的收集窗口截至2026-02-12，早于2026-02-19业绩披露，因此预测本身形成于业绩前；但该PDF首次公开日期和当时网页可发现性未获证明，本案例不声称可用于严格的点时回测。实际值直接采用发行人负责的法规披露所列Q4报告EPS。" },
        "target_upside" => Definition { case_key: "airbus", formula: "目标价上涨空间=(mwb目标价-mwb报告采用的当前价)/当前价", inputs: &["mwb_target_price","mwb_current_price"], sources: &["mwb_report"], note: "这是一个具名分析机构在2025-02-24的历史目标价，不是市场一致目标价。" },
        "forecast_dispersion" => Definition { case_key: "signify", formula: "收入预测极差离均值比例=(最高收入预测-最低收入预测)/平均收入预测", inputs: &["q4_sales_high","q4_sales_low","q4_sales_average"], sources: &["signify_consensus"], note: "同一份事前一致预期快照中的Q4 2025总收入高值、低值和均值；公开PDF未给逐家预测或标准差，因此明确使用极差离均值比例。" },
        "revision" => Definition { case_key: "airbus", formula: "同一分析师2025E EPS修订率=(新EPS-旧EPS)/旧EPS", inputs: &["mwb_2025_eps","mwb_2025_previous_eps"], sources: &["mwb_report"], note: "mwb research同一份2025-02-24报告把2025E EPS从5.85欧元上调2.8%至6.01欧元；这是单一分析团队的匹配修订，不冒充市场修订广度。" },
        _ => return None,
    })
}

fn result_key(id: &str) -> &'static str {
    match id {
        "buyback_rate" => "buyback_rate",
        "holder_concentration" => "top_holder_fraction",
        "insider_trading" => "insider_trading_rate",
        "institution_holding" => "institution_holding",
        "restricted_shares" => "restricted_share_fraction",
        "share_pledge" => "share_pledge",
        "short_interest" => "days_to_cover",
        "consensus" => "mean_eps",
        "earnings_surprise" => "earnings_surprise",
        "forecast_dispersion" => "forecast_dispersion",
        "revision" => "earnings_revision",
        "target_upside" => "target_upside",
        _ => "",
    }
}

fn data() -> Result<&'static Value, String> {
    static DATA: OnceLock<Result<Value, String>> = OnceLock::new();
    match DATA.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../docs/book/ownership_cases.json"))
            .map_err(|e| format!("invalid embedded ownership cases: {e}"))?;
        validate_data(&value)?;
        Ok(value)
    }) {
        Ok(v) => Ok(v),
        Err(e) => Err(e.clone()),
    }
}

fn validate_data(root: &Value) -> Result<(), String> {
    for id in SUPPORTED_IDS {
        let def = definition(id).ok_or_else(|| format!("missing ownership definition: {id}"))?;
        let case = &root[def.case_key];
        if case["case_id"].as_str().is_none() || case["ticker"] != fixed_symbol(id) {
            return Err(format!("invalid ownership case identity: {id}"));
        }
        for key in def.inputs {
            let fact = &case["facts"][*key];
            if !fact["value"].as_f64().is_some_and(f64::is_finite)
                || fact["unit"].as_str().is_none()
                || fact["label"].as_str().is_none()
                || fact["source_id"].as_str().is_none()
            {
                return Err(format!("invalid ownership fact: {key}"));
            }
        }
    }
    Ok(())
}

fn source_config<'a>(
    registry: &'a OwnershipSourceRegistry,
    id: &str,
) -> Option<&'a OwnershipSourceConfig> {
    Some(match id {
        "moutai_annual_report" => &registry.moutai,
        "cboe_consolidated_short_interest" => &registry.cboe_short_interest,
        "airbus_consensus" => &registry.airbus_consensus,
        "airbus_fy_results" => &registry.airbus_fy_results,
        "mwb_report" => &registry.mwb_report,
        "signify_consensus" => &registry.signify_consensus,
        _ => return None,
    })
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

async fn verified_source(
    cache_dir: &Path,
    expected: &OwnershipSourceConfig,
) -> Result<&'static str, String> {
    if expected.bytes == 0 || expected.bytes > 8_000_000 || expected.sha256.len() != 64 {
        return Err("ownership source expectation is invalid".into());
    }
    let _guard = SOURCE_LOCK.lock().await;
    let directory = cache_dir.join("ownership-sources");
    let cached = directory.join(format!("{}.source", expected.sha256));
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        if bytes.len() == expected.bytes && fingerprint(&bytes) == expected.sha256 {
            return Ok("verified_immutable_cache");
        }
        tokio::fs::remove_file(&cached)
            .await
            .map_err(|e| format!("corrupt ownership cache removal failed: {e}"))?;
    }
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("Axiom educational source verifier")
        .build()
        .map_err(|e| format!("ownership source client failed: {e}"))?
        .get(&expected.url)
        .send()
        .await
        .map_err(|e| format!("ownership source request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("ownership source returned an error: {e}"))?;
    if response
        .content_length()
        .is_some_and(|n| n != expected.bytes as u64)
    {
        return Err("ownership source Content-Length differs from the reviewed document".into());
    }
    let mut bytes = Vec::with_capacity(expected.bytes);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("ownership source body failed: {e}"))?;
        if bytes.len().saturating_add(chunk.len()) > expected.bytes {
            return Err("ownership source body exceeds the reviewed document size".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != expected.bytes || fingerprint(&bytes) != expected.sha256 {
        return Err("downloaded ownership source is incomplete or has a different SHA-256".into());
    }
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| format!("ownership cache directory failed: {e}"))?;
    tokio::fs::write(&cached, bytes)
        .await
        .map_err(|e| format!("ownership cache write failed: {e}"))?;
    Ok("verified_then_cached")
}

fn number(case: &Value, key: &str) -> Result<f64, String> {
    case["facts"][key]["value"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| format!("missing ownership fact {key}"))
}

fn ratio(n: f64, d: f64) -> Result<f64, String> {
    if d == 0.0 {
        return Err("ownership calculation has a zero denominator".into());
    }
    let value = n / d;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| "ownership calculation is invalid".into())
}

type Calculation = (
    Map<String, Value>,
    Map<String, Value>,
    &'static str,
    Option<&'static str>,
);

fn calculate(id: &str, case: &Value) -> Result<Calculation, String> {
    let n = |key: &str| number(case, key);
    let mut values = Map::new();
    let mut units = Map::new();
    let mut put = |key: &str, value: Value, unit: &str| {
        values.insert(key.into(), value);
        units.insert(key.into(), json!(unit));
    };
    match id {
        "buyback_rate" => put(
            "buyback_rate",
            json!(ratio(
                n("first_program_repurchased_shares")? + n("second_program_repurchased_shares")?,
                n("opening_total_shares")?
            )?),
            "fraction",
        ),
        "holder_concentration" => {
            let shares: Vec<f64> = (1..=10)
                .map(|i| n(&format!("holder_{i}_shares")))
                .collect::<Result<_, _>>()?;
            let total = n("closing_total_shares")?;
            put(
                "top_holder_fraction",
                json!(ratio(shares.iter().sum(), total)?),
                "fraction",
            );
            put(
                "listed_holder_hhi",
                json!(shares.iter().map(|s| (s / total).powi(2)).sum::<f64>()),
                "fraction squared",
            );
        }
        "institution_holding" => put(
            "institution_holding",
            json!(ratio(
                n("holder_5_shares")?
                    + n("holder_7_shares")?
                    + n("holder_9_shares")?
                    + n("holder_10_shares")?,
                n("closing_total_shares")?
            )?),
            "fraction",
        ),
        "restricted_shares" => {
            let balance = n("closing_total_shares")? - n("unrestricted_shares")?;
            put("restricted_shares_balance", json!(balance), "shares");
            put(
                "restricted_share_fraction",
                json!(ratio(balance, n("closing_total_shares")?)?),
                "fraction",
            );
        }
        "share_pledge" => put(
            "share_pledge",
            json!(ratio(
                n("controlling_holder_pledged_shares")?,
                n("controlling_holder_shares")?
            )?),
            "fraction",
        ),
        "insider_trading" => {
            let acquired = n("controlling_holder_acquired_shares")?;
            let ending = n("controlling_holder_shares")?;
            let opening = ending - acquired;
            put("acquired_shares", json!(acquired), "shares");
            put(
                "insider_trading_rate",
                json!(ratio(acquired, opening)?),
                "fraction of pre-transaction holdings",
            );
        }
        "short_interest" => {
            let short_shares = n("aggregate_short_shares")?;
            let days_to_cover = ratio(short_shares, n("average_daily_share_volume")?)?;
            if (days_to_cover - n("reported_days_to_cover")?).abs() > 0.005 {
                return Err("Cboe days-to-cover cross-check failed".into());
            }
            put("aggregate_short_shares", json!(short_shares), "shares");
            put("days_to_cover", json!(days_to_cover), "days");
        }
        "consensus" => {
            put(
                "mean_eps",
                json!(n("q4_consensus_reported_eps")?),
                "EUR/share",
            );
            put(
                "analyst_count",
                json!(n("q4_consensus_analyst_count")?),
                "analysts",
            );
        }
        "earnings_surprise" => put(
            "earnings_surprise",
            json!(ratio(
                n("q4_2025_reported_eps")? - n("q4_consensus_reported_eps")?,
                n("q4_consensus_reported_eps")?.abs()
            )?),
            "fraction",
        ),
        "target_upside" => put(
            "target_upside",
            json!(ratio(
                n("mwb_target_price")? - n("mwb_current_price")?,
                n("mwb_current_price")?
            )?),
            "fraction",
        ),
        "forecast_dispersion" => {
            put(
                "forecast_dispersion",
                json!(ratio(
                    n("q4_sales_high")? - n("q4_sales_low")?,
                    n("q4_sales_average")?
                )?),
                "forecast range divided by mean",
            );
        }
        "revision" => {
            let old = n("mwb_2025_previous_eps")?;
            let new = n("mwb_2025_eps")?;
            put(
                "earnings_revision",
                json!(ratio(new - old, old)?),
                "fraction",
            );
        }
        _ => return Err(format!("unsupported ownership disclosure concept: {id}")),
    }
    if id == "short_interest" {
        Ok((
            values,
            units,
            "partial",
            Some("Cboe逐证券合并市场淡仓报告提供汇总淡仓股数与回补天数，但该固定来源没有自由流通股分母，因此不计算空仓占自由流通股比例。"),
        ))
    } else {
        Ok((values, units, "computed", None))
    }
}

pub async fn evaluate(
    id: &str,
    cache_dir: &Path,
    registry: &OwnershipSourceRegistry,
) -> Result<Value, String> {
    let def =
        definition(id).ok_or_else(|| format!("unsupported ownership disclosure concept: {id}"))?;
    let root = data()?;
    let case = &root[def.case_key];
    let mut verifications = BTreeMap::new();
    for source_id in def.sources {
        let config = source_config(registry, source_id)
            .ok_or_else(|| format!("missing ownership source config: {source_id}"))?;
        verifications.insert(
            *source_id,
            (config, verified_source(cache_dir, config).await?),
        );
    }
    let (values, units, status, reason) = calculate(id, case)?;
    let mut sources = Vec::new();
    for source_id in def.sources {
        let meta = &case["sources"][*source_id];
        let (config, verification_status) = verifications[source_id];
        sources.push(json!({ "id": source_id, "title": meta["title"], "url": meta["url"], "canonical_url": meta.get("canonical_url").cloned().unwrap_or(Value::Null), "registry_page_url": meta.get("registry_page_url").cloned().unwrap_or(Value::Null), "sha256": meta["sha256"], "bytes": meta["bytes"], "format": meta["format"], "published": meta["published"], "as_of": meta["as_of"], "verification": {"status": verification_status, "verified_at": chrono::Utc::now(), "requested_url": config.url, "matched_sha256": config.sha256, "matched_bytes": config.bytes} }));
    }
    let mut facts = Vec::new();
    let mut symbol_mapping = Map::new();
    let mut field_provenance = Map::new();
    let mut pdf_pages = Vec::new();
    for key in def.inputs {
        let fact = &case["facts"][*key];
        let source_id = fact["source_id"].as_str().unwrap();
        let source = &case["sources"][source_id];
        let page = fact["pdf_page"].as_u64();
        if let Some(page) = page {
            pdf_pages.push(page);
        }
        symbol_mapping.insert((*key).into(), fact["label"].clone());
        field_provenance.insert(
            (*key).into(),
            json!({"kind": "reported", "pdf_page": page, "note": format!("Source-reported as {}", fact["label"].as_str().unwrap_or(key))}),
        );
        let mut reported = json!({
            "key": key,
            "label": fact["label"],
            "value": fact["value"],
            "unit": fact["unit"],
            "pdf_page": fact["pdf_page"],
            "source_id": source_id,
            "source_url": source["url"],
            "source_format": source["format"],
            "published": source["published"],
            "as_of": source["as_of"],
            "kind": "reported"
        });
        if let Some(section) = fact.get("section") {
            reported["source_section"] = section.clone();
        }
        facts.push(reported);
    }
    pdf_pages.sort_unstable();
    pdf_pages.dedup();
    for key in values.keys() {
        field_provenance.insert(
            key.clone(),
            json!({"kind": "derived", "note": format!("Calculated only from the listed reported facts using {}", def.formula)}),
        );
    }
    let primary = &sources[0];
    let label = match id {
        "short_interest" => "市场空仓报告",
        "consensus" | "forecast_dispersion" => "预测汇总",
        "earnings_surprise" => "预测汇总与公司业绩原文",
        "revision" | "target_upside" => "分析师报告",
        _ => "公司股东披露",
    };
    let issuer_name = case["issuer"].as_str().unwrap_or_default();
    let reporting_entity = match id {
        "short_interest" => "Cboe BZX 合并空仓报告",
        "consensus" => "Airbus SE（外部分析师预测汇总）",
        "earnings_surprise" => "Airbus SE（外部分析师预测汇总与法规披露）",
        "forecast_dispersion" => "Signify N.V.（外部分析师预测汇总）",
        "revision" | "target_upside" => "mwb research",
        _ => issuer_name,
    };
    let audit_boundary = if id == "short_interest" {
        "Cboe发布的固定结算日逐证券合并市场淡仓报告；不是审计数据，且没有自由流通股分母，因此只计算绝对淡仓数和回补天数。"
    } else if id == "earnings_surprise" {
        "实际EPS来自发行人负责的法规披露；预测收集早于业绩披露，但一致预期PDF首次公开日期及当时网页可发现性未获证明。"
    } else if matches!(id, "consensus" | "forecast_dispersion") {
        "发行人汇编的外部卖方分析师预测，未经发行人背书或审计。"
    } else if matches!(id, "revision" | "target_upside") {
        "具名卖方研究报告中的分析师估计，未经发行人审计。"
    } else {
        "来源为年度报告中的股东或股份信息；不把财务报表审计意见泛化为对该表格的单独鉴证。"
    };
    Ok(json!({
        "concept_id": id,
        "input_kind": "industry_case",
        "provenance": "verified_original_public_source",
        "status": status,
        "reason": reason,
        "values": values,
        "units": units,
        "series": [],
        "inputs": {},
        "bars": [],
        "notes": ["固定历史披露练习，不代表当前行情、任意股票或完整市场数据库。", def.note],
        "industry_case": {
            "label": label,
            "case_id": case["case_id"],
            "issuer": {
                "name": issuer_name,
                "ticker": case["ticker"],
                "alternate_tickers": [],
                "reporting_entity": reporting_entity,
                "metric_entity": issuer_name
            },
            "period": case["period"],
            "published": primary["published"],
            "audited": false,
            "audit_boundary": audit_boundary,
            "status": "reviewed_historical_source",
            "url": primary["url"],
            "format": primary["format"],
            "pdf_pages": pdf_pages,
            "sha256": primary["sha256"],
            "bytes": primary["bytes"],
            "source_kind": "verified_original_public_source",
            "sources": sources,
            "verification": primary["verification"]
        },
        "industry_facts": {
            "currency": if matches!(id, "consensus" | "earnings_surprise" | "forecast_dispersion" | "revision" | "target_upside") { "EUR" } else { "N/A" },
            "scale": "Units are stated per fact",
            "reported_facts": facts,
            "calculation": {
                "formula": def.formula,
                "result_key": result_key(id),
                "operands": def.inputs,
                "symbol_mapping": symbol_mapping
            },
            "field_provenance": field_provenance,
            "definitions": {
                "case_boundary": "Fixed issuer, source and reporting period; never generalized from caller-selected data.",
                "metric": def.note
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seeded(root: &Path) -> OwnershipSourceRegistry {
        let mut registry = OwnershipSourceRegistry::default();
        let directory = root.join("ownership-sources");
        std::fs::create_dir_all(&directory).unwrap();
        for source in [
            &mut registry.moutai,
            &mut registry.cboe_short_interest,
            &mut registry.airbus_consensus,
            &mut registry.airbus_fy_results,
            &mut registry.mwb_report,
            &mut registry.signify_consensus,
        ] {
            let bytes = format!("fixture:{}", source.url).into_bytes();
            source.bytes = bytes.len();
            source.sha256 = fingerprint(&bytes);
            std::fs::write(directory.join(format!("{}.source", source.sha256)), bytes).unwrap();
        }
        registry
    }
    #[tokio::test]
    async fn every_fixed_case_computes_from_independent_literals() {
        let root = std::env::temp_dir().join(format!("axiom-ownership-{}", uuid::Uuid::new_v4()));
        let registry = seeded(&root);
        let expected = [
            (
                "buyback_rate",
                "buyback_rate",
                (3_927_585.0 + 87_059.0) / 1_256_197_800.0,
            ),
            (
                "holder_concentration",
                "top_holder_fraction",
                874_519_547.0 / 1_252_270_215.0,
            ),
            (
                "institution_holding",
                "institution_holding",
                34_904_752.0 / 1_252_270_215.0,
            ),
            ("restricted_shares", "restricted_share_fraction", 0.0),
            ("share_pledge", "share_pledge", 0.0),
            (
                "insider_trading",
                "insider_trading_rate",
                2_071_359.0 / (681_282_935.0 - 2_071_359.0),
            ),
            ("short_interest", "days_to_cover", 378_713.0 / 56_692.0),
            ("consensus", "mean_eps", 2.82),
            (
                "earnings_surprise",
                "earnings_surprise",
                (3.27 - 2.82) / 2.82,
            ),
            ("target_upside", "target_upside", (145.0 - 159.90) / 159.90),
            (
                "forecast_dispersion",
                "forecast_dispersion",
                (1_579.0 - 1_496.0) / 1_533.0,
            ),
            ("revision", "earnings_revision", (6.01 - 5.85) / 5.85),
        ];
        for (id, key, expected) in expected {
            let value = evaluate(id, &root, &registry).await.unwrap();
            assert!(
                (value["values"][key].as_f64().unwrap() - expected).abs() < 1e-12,
                "{id}"
            );
            assert_eq!(value["input_kind"], "industry_case");
            assert_eq!(
                value["status"],
                if id == "short_interest" {
                    "partial"
                } else {
                    "computed"
                }
            );
            assert_eq!(value["industry_facts"]["calculation"]["result_key"], key);
            assert_eq!(
                value["industry_facts"]["calculation"]["operands"],
                json!(definition(id).unwrap().inputs)
            );
            assert!(value["industry_case"]["pdf_pages"].as_array().is_some());
            assert!(value["industry_case"]["issuer"]["alternate_tickers"]
                .as_array()
                .is_some());
            assert!(value["industry_case"]["issuer"]["reporting_entity"]
                .as_str()
                .is_some());
            assert!(value["industry_case"]["audited"].as_bool().is_some());
            assert!(value["industry_case"]["audit_boundary"].as_str().is_some());
            assert!(value["industry_case"]["status"].as_str().is_some());
            assert!(value["industry_facts"]["currency"].as_str().is_some());
            assert!(value["industry_facts"]["scale"].as_str().is_some());
            assert!(value["industry_facts"]["definitions"]["metric"]
                .as_str()
                .is_some());
            assert!(value["industry_facts"]["field_provenance"][key]["kind"] == "derived");
            assert!(definition(id)
                .unwrap()
                .inputs
                .iter()
                .all(
                    |operand| value["industry_facts"]["calculation"]["symbol_mapping"][operand]
                        .as_str()
                        .is_some()
                ));
            assert!(value["industry_case"]["sources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["id"].as_str().is_some()
                    && s["url"].as_str().is_some()
                    && s["sha256"].as_str().is_some()
                    && s["bytes"].as_u64().is_some()
                    && s["format"].as_str().is_some()
                    && s["published"].as_str().is_some()
                    && s["verification"]["status"] == "verified_immutable_cache"));
            assert!(value["industry_facts"]["reported_facts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["kind"] == "reported" && f["source_url"].as_str().is_some()));
        }
        assert!(evaluate("unknown", &root, &registry)
            .await
            .unwrap_err()
            .contains("unsupported"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn registry_and_public_configuration_are_complete() {
        assert!(data().is_ok());
        assert!(SUPPORTED_IDS.iter().all(|id| is_supported(id)
            && !fixed_symbol(id).is_empty()
            && definition(id).is_some()));
        assert!(!is_supported("not_a_metric"));
        assert_eq!(fixed_symbol("not_a_metric"), "");
        assert!(ratio(1.0, 0.0).is_err());
        let registry = OwnershipSourceRegistry::default();
        assert_eq!(registry.moutai.bytes, MOUTAI_BYTES);
        assert_eq!(
            registry.cboe_short_interest.sha256,
            CBOE_SHORT_INTEREST_SHA256
        );
        assert_eq!(registry.signify_consensus.url, SIGNIFY_CONSENSUS_URL);
        assert_eq!(registry.airbus_fy_results.url, AIRBUS_FY_RESULTS_URL);
        assert_eq!(registry.airbus_fy_results.format, "PDF");
        assert_eq!(registry.airbus_fy_results.bytes, AIRBUS_FY_RESULTS_BYTES);
        assert_eq!(registry.airbus_fy_results.sha256, AIRBUS_FY_RESULTS_SHA256);
    }

    #[test]
    fn public_catalog_configuration_replaces_teaching_inputs() {
        let mut concept = crate::practice::base_catalog()
            .into_iter()
            .next()
            .expect("base catalog");
        concept.id = "short_interest".into();
        configure_concept(&mut concept);
        assert_eq!(concept.input_kind, "industry_case");
        assert!(concept.inputs.is_empty());
        assert!(concept.notes.contains("固定历史披露练习"));

        let mut entry = crate::knowledge::base_entries()
            .into_iter()
            .next()
            .expect("knowledge catalog");
        entry.id = "earnings_surprise".into();
        configure_knowledge(&mut entry);
        assert!(entry.formula.contains("Q4 2025报告EPS"));
        assert!(entry.signals.contains("具名历史样本"));
        assert!(entry.pitfalls.contains("首次公开日期"));

        let old_formula = entry.formula.clone();
        entry.id = "not_an_ownership_metric".into();
        configure_knowledge(&mut entry);
        assert_eq!(entry.formula, old_formula);
    }

    #[tokio::test]
    async fn official_cold_sources_download_and_then_use_immutable_cache() {
        let root = std::env::temp_dir().join(format!(
            "axiom-ownership-live-source-{}",
            uuid::Uuid::new_v4()
        ));
        let registry = OwnershipSourceRegistry::default();
        for source in [
            registry.cboe_short_interest,
            registry.airbus_consensus,
            registry.airbus_fy_results,
        ] {
            assert_eq!(
                verified_source(&root, &source).await.unwrap(),
                "verified_then_cached"
            );
            assert_eq!(
                verified_source(&root, &source).await.unwrap(),
                "verified_immutable_cache"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn source_guardrails_reject_invalid_expectations_and_corrupt_cache() {
        assert_eq!(result_key("not_a_metric"), "");
        assert!(source_config(&OwnershipSourceRegistry::default(), "not_a_source").is_none());
        assert!(calculate("not_a_metric", &json!({})).is_err());
        let mut invalid_identity = data().unwrap().clone();
        invalid_identity["moutai"]["ticker"] = json!("WRONG");
        assert!(validate_data(&invalid_identity).is_err());
        let mut invalid_fact = data().unwrap().clone();
        invalid_fact["moutai"]["facts"]["opening_total_shares"]["value"] = Value::Null;
        assert!(validate_data(&invalid_fact).is_err());

        let root =
            std::env::temp_dir().join(format!("axiom-ownership-guard-{}", uuid::Uuid::new_v4()));
        let invalid = OwnershipSourceConfig::new("https://example.invalid", "bad", 0, "PDF");
        assert!(verified_source(&root, &invalid)
            .await
            .unwrap_err()
            .contains("expectation is invalid"));

        let directory = root.join("ownership-sources");
        std::fs::create_dir_all(&directory).unwrap();
        let expected =
            OwnershipSourceConfig::new("http://127.0.0.1:1/unreachable", &"0".repeat(64), 1, "PDF");
        let cached = directory.join(format!("{}.source", expected.sha256));
        std::fs::write(&cached, b"wrong").unwrap();
        assert!(verified_source(&root, &expected).await.is_err());
        assert!(!cached.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
