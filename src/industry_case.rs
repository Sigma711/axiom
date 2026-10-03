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

pub const MOUTAI_URL: &str = "https://static.cninfo.com.cn/finalpage/2026-04-17/1225114741.PDF";
pub const MOUTAI_SHA256: &str = "474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288";
pub const MOUTAI_BYTES: usize = 1_082_847;
pub const DELTA_URL: &str = "https://s2.q4cdn.com/181345880/files/doc_financials/2024/q4/DAL-12-31-2024-10K-2-11-25-Filed.pdf";
pub const DELTA_SHA256: &str = "61116b7fe79dd0c687d88c04ac376e4d09a6c3760163bbe9433f572bb2549afa";
pub const DELTA_BYTES: usize = 897_733;
pub const COSTCO_URL: &str = "https://s201.q4cdn.com/287523651/files/doc_news/Costco-Wholesale-Corporation-Reports-Fourth-Quarter-and-Fiscal-Year-2024-Operating-Results-2024.pdf";
pub const COSTCO_SHA256: &str = "590d2dc15e168ca52697a8d8b85b0f388cbea3eab8235b2ec8aa77ed4f173c57";
pub const COSTCO_BYTES: usize = 138_152;
pub const MODERNA_URL: &str =
    "https://s29.q4cdn.com/435878511/files/doc_financials/2024/ar/MRNA010_AR_WEB_FULL.pdf";
pub const MODERNA_SHA256: &str = "2347835006ac22d5cd9b74683568893431149071740e81ff17603504ff70c1a5";
pub const MODERNA_BYTES: usize = 3_098_566;
pub const PETROBRAS_URL: &str = "https://transparencia.petrobras.com.br/documents/1357439/14971831/Relat%C3%B3rio%2Bde%2BGest%C3%A3o%2B-%2B2024.pdf/50685b26-3e9e-2ece-3035-33eebe338c73?download=true&t=1748554910000&version=1.0";
pub const PETROBRAS_SHA256: &str =
    "04372d526d67247b9ad66098a58d85ca2bf00b534478575ead5f650a7a463122";
pub const PETROBRAS_BYTES: usize = 6_302_154;
pub const BARRICK_URL: &str = "https://www.barrick.com/files/doc_financial/annual_reports/2024/Barrick_Annual_Report_2024.pdf";
pub const BARRICK_SHA256: &str = "3cb6cf59458e8799650d1c219222f8c01e41b1fbda9351523ed6b602f3875b86";
pub const BARRICK_BYTES: usize = 11_789_238;
pub const SIEMENS_URL: &str = "https://assets.new.siemens.com/siemens/assets/api/uuid:344347ec-a1bd-44cb-aaaa-711d1b3ec1b8/Siemens-Annual-Report-2024.pdf";
pub const SIEMENS_SHA256: &str = "75f568180a8d35287f970a4812817dcd2b5c690ec937bf80f17b6fe68f42521e";
pub const SIEMENS_BYTES: usize = 4_671_939;
pub const META_URL: &str = "https://investor.fb.com/files/doc_earnings/2023/q3/presentation/Earnings-Presentation-Q3-2023.pdf";
pub const META_SHA256: &str = "dfcaa1c855d2da261f0d392c4a603fddf8897934dc60b3397f272698bf071af4";
pub const META_BYTES: usize = 172_720;
pub const SPOTIFY_URL: &str = "https://investors.spotify.com/files/doc_financials/2020/q3/Shareholder-Letter-Q3-2020_FINAL.pdf";
pub const SPOTIFY_SHA256: &str = "82025cc49cce680c62ba9e5576881e6e84c867ba77f44a4f46d82f6c9ae81518";
pub const SPOTIFY_BYTES: usize = 1_172_171;
pub const SNOWFLAKE_URL: &str = "https://investors.snowflake.com/files/doc_financials/2024/q4/Q4-FY2024-Investor-Presentation-vF.pdf";
pub const SNOWFLAKE_SHA256: &str =
    "8da8efb70b65fc2c8928a1d6ccef32e530d447d510fa9da033dcb916ccf80a37";
pub const SNOWFLAKE_BYTES: usize = 5_257_197;
pub const SIMILARWEB_URL: &str = "https://d1io3yog0oux5.cloudfront.net/_8f428cad86e9f7d21dc312829a41f817/similarweb/db/2008/19607/presentation/SMWB_Q3_2024_Investor_Presentation_.pdf";
pub const SIMILARWEB_SHA256: &str =
    "3278ddfd096ebcc828579466b3e0af46b01f6fe38f72554f1d1304b0bfb53bf0";
pub const SIMILARWEB_BYTES: usize = 9_680_955;
pub const ZOOM_URL: &str =
    "https://investors.zoom.us/static-files/70629942-ff77-4bed-91d6-422766c47e6b";
pub const ZOOM_SHA256: &str = "79f0e6b5126a47869f196d07aa3ab3626c4a61cdbb46e17a79762ab264fbeaf4";
pub const ZOOM_BYTES: usize = 118_862;
const ZOOM_ARCHIVED_ORIGINAL: &[u8] =
    include_bytes!("../data/verified-sources/zoom-q1-fy2025-prepared-remarks.pdf");
pub const SMIC_URL: &str =
    "https://www1.hkexnews.hk/listedco/listconews/sehk/2025/0211/2025021100441.pdf";
pub const SMIC_SHA256: &str = "18e7cc96cc2405587fbb06e5da078fe4ce129833e6257009fe1c650a0d070a76";
pub const SMIC_BYTES: usize = 444_831;
pub const VERIZON_URL: &str =
    "https://www.verizon.com/about/sites/default/files/2024-04/FS_VZ_1Q24_042224.pdf";
pub const VERIZON_SHA256: &str = "c22a0161f9268b2d9799cbfa1ea78da0e44d16d1cb502896e5a0ac212bae4817";
pub const VERIZON_BYTES: usize = 103_314;
pub const FRONTLINE_URL: &str =
    "https://www.frontlineplc.cy/wp-content/uploads/2024/09/Presentation-Q2-2024.pdf";
pub const FRONTLINE_SHA256: &str =
    "394b72e6c229f9586a18a2b1348b8262fc11459afa7c30147df2d5f1ff3677fa";
pub const FRONTLINE_BYTES: usize = 870_579;

pub const SUPPORTED_IDS: &[&str] = &[
    "book_share_counts",
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
    "book_airline_casm",
    "book_airline_load_factor",
    "book_airline_rasm",
    "book_bank_cet1_ratio",
    "book_bank_provision_coverage",
    "book_insurance_combined_ratio",
    "book_insurance_nbv",
    "book_reit_affo",
    "book_reit_cap_rate",
    "book_reit_ffo",
    "book_retail_same_store_sales_growth",
    "book_biopharma_cash_runway",
    "book_energy_lifting_cost",
    "book_energy_reserve_life",
    "book_gold_aisc",
    "book_industrial_backlog",
    "book_industrial_book_to_bill",
    "book_internet_arpu",
    "book_internet_dau_mau",
    "book_saas_cac_payback",
    "book_saas_churn",
    "book_saas_nrr",
    "book_semiconductor_asp",
    "book_semiconductor_utilization",
    "book_shipping_tce",
    "book_telecom_arpu",
    "book_telecom_churn",
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
    pub moutai: IndustrySourceConfig,
    pub delta: IndustrySourceConfig,
    pub costco: IndustrySourceConfig,
    pub moderna: IndustrySourceConfig,
    pub petrobras: IndustrySourceConfig,
    pub barrick: IndustrySourceConfig,
    pub siemens: IndustrySourceConfig,
    pub meta: IndustrySourceConfig,
    pub spotify: IndustrySourceConfig,
    pub snowflake: IndustrySourceConfig,
    pub similarweb: IndustrySourceConfig,
    pub zoom: IndustrySourceConfig,
    pub smic: IndustrySourceConfig,
    pub verizon: IndustrySourceConfig,
    pub frontline: IndustrySourceConfig,
}

impl Default for IndustrySourceRegistry {
    fn default() -> Self {
        Self {
            moutai: IndustrySourceConfig {
                url: MOUTAI_URL.into(),
                sha256: MOUTAI_SHA256.into(),
                bytes: MOUTAI_BYTES,
            },
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
            delta: IndustrySourceConfig {
                url: DELTA_URL.into(),
                sha256: DELTA_SHA256.into(),
                bytes: DELTA_BYTES,
            },
            costco: IndustrySourceConfig {
                url: COSTCO_URL.into(),
                sha256: COSTCO_SHA256.into(),
                bytes: COSTCO_BYTES,
            },
            moderna: source(MODERNA_URL, MODERNA_SHA256, MODERNA_BYTES),
            petrobras: source(PETROBRAS_URL, PETROBRAS_SHA256, PETROBRAS_BYTES),
            barrick: source(BARRICK_URL, BARRICK_SHA256, BARRICK_BYTES),
            siemens: source(SIEMENS_URL, SIEMENS_SHA256, SIEMENS_BYTES),
            meta: source(META_URL, META_SHA256, META_BYTES),
            spotify: source(SPOTIFY_URL, SPOTIFY_SHA256, SPOTIFY_BYTES),
            snowflake: source(SNOWFLAKE_URL, SNOWFLAKE_SHA256, SNOWFLAKE_BYTES),
            similarweb: source(SIMILARWEB_URL, SIMILARWEB_SHA256, SIMILARWEB_BYTES),
            zoom: source(ZOOM_URL, ZOOM_SHA256, ZOOM_BYTES),
            smic: source(SMIC_URL, SMIC_SHA256, SMIC_BYTES),
            verizon: source(VERIZON_URL, VERIZON_SHA256, VERIZON_BYTES),
            frontline: source(FRONTLINE_URL, FRONTLINE_SHA256, FRONTLINE_BYTES),
        }
    }
}

fn source(url: &str, sha256: &str, bytes: usize) -> IndustrySourceConfig {
    IndustrySourceConfig {
        url: url.into(),
        sha256: sha256.into(),
        bytes,
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
    moutai: IndustryCase,
    delta: IndustryCase,
    costco: IndustryCase,
    moderna: IndustryCase,
    petrobras: IndustryCase,
    barrick: IndustryCase,
    siemens: IndustryCase,
    meta: IndustryCase,
    spotify: IndustryCase,
    snowflake: IndustryCase,
    similarweb: IndustryCase,
    zoom: IndustryCase,
    smic: IndustryCase,
    verizon: IndustryCase,
    frontline: IndustryCase,
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
    Moutai,
    Delta,
    Costco,
    Moderna,
    Petrobras,
    Barrick,
    Siemens,
    Meta,
    Spotify,
    Snowflake,
    Similarweb,
    Zoom,
    Smic,
    Verizon,
    Frontline,
}

pub fn is_supported(id: &str) -> bool {
    SUPPORTED_IDS.contains(&id)
}

/// The immutable issuer ticker expected by the fixed historical case.
pub fn fixed_symbol(id: &str) -> Option<&'static str> {
    Some(match definition(id)?.case {
        CaseKey::PingAn => "2318.HK",
        CaseKey::Shopify => "SHOP",
        CaseKey::RealtyIncome => "O",
        CaseKey::Ebay => "EBAY",
        CaseKey::Moutai => "600519",
        CaseKey::Delta => "DAL",
        CaseKey::Costco => "COST",
        CaseKey::Moderna => "MRNA",
        CaseKey::Petrobras => "PBR",
        CaseKey::Barrick => "GOLD",
        CaseKey::Siemens => "SIE.DE",
        CaseKey::Meta => "META",
        CaseKey::Spotify => "SPOT",
        CaseKey::Snowflake => "SNOW",
        CaseKey::Similarweb => "SMWB",
        CaseKey::Zoom => "ZM",
        CaseKey::Smic => "0981.HK",
        CaseKey::Verizon => "VZ",
        CaseKey::Frontline => "FRO",
    })
}

pub fn configure_concept(concept: &mut crate::practice::PracticeConcept) {
    if is_supported(&concept.id) {
        concept.input_kind = "industry_case".into();
        if concept.id == "book_share_counts" {
            concept.category = "发行人历史披露".into();
        }
        concept.inputs.clear();
        concept.notes = "固定发行人历史披露案例：用已公布的经营数据理解指标，数值可回到原文对应页核对；不代表当前所选股票。".into();
    }
}

pub fn configure_knowledge(entry: &mut crate::knowledge::KnowledgeEntry) {
    if let Some(definition) = definition(&entry.id) {
        entry.formula = definition.formula.into();
        if entry.id == "book_share_counts" {
            entry.category = "发行人历史披露".into();
            entry.summary =
                "贵州茅台2025年末股本结构：逐项核对总股本、无限售条件流通股份与年内注销数量。"
                    .into();
            entry.example = "1,256,197,800−3,927,585=1,252,270,215股；期末总股本减无限售条件流通股份的余额为0股。".into();
            entry.pitfalls = "无限售条件流通股份不等于自由流通股；报告发布于2026-04-17，不代表2025年末当时已知信息。".into();
        }
        entry.signals = "练习使用一个已验证的发行人历史披露来核对原书公式；结果只描述指定实体和报告期，不是当前行情、任意股票查询或独立买卖信号。跨公司比较前必须统一币种、单位、期间、业务定义与审计边界。".into();
    }
}

fn definition(id: &str) -> Option<MetricDefinition> {
    let outside_audit = "PDF第57–58页管理层讨论数据不在安永对财务报表出具的审计意见覆盖范围内";
    let shopify_unaudited = "发行人业绩公告指标；不声称MRR、GMV或非GAAP自由现金流获得审计保证";
    Some(match id {
        "book_share_counts" => MetricDefinition {
            case: CaseKey::Moutai,
            metric_entity: "贵州茅台酒股份有限公司",
            formula: "限售股份余额=期末总股本−期末无限售条件流通股份；期初总股本−注销股份=期末总股本",
            inputs: &["opening_total_shares", "cancelled_shares", "closing_total_shares", "unrestricted_shares"],
            audited: false,
            audit_boundary: "股份变动表位于年度报告第47页，不是财务报表审计意见覆盖的股本附注；本案例不声称该表获得独立审计保证",
            value_key: "restricted_shares_residual",
            value_unit: "shares",
            note: "截至2025-12-31的固定股本结构：总股本与无限售条件流通股份均为1,252,270,215股，限售余额由两者相减得到0股。年报第47页引用2025-08-30公告临2025-032，披露注销3,927,585股；公告日期不冒充注销生效日。报告于2026-04-17发布，不代表2025年末当时已知信息。无限售条件流通股份不等于自由流通股，本案例未提供自由流通股数或市值。",
        },
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
            audit_boundary: "PDF第336页附注49(7)位于安永审计意见覆盖的已审计财务报表内",
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
            audit_boundary: "发行人将该补充经营资料标注为未经审计",
            value_key: "occupied_area_ratio",
            value_unit: "fraction",
            note: "使用平方英尺面积比 335,777,818/339,361,416，不使用按物业数量计算的 98.7%。",
        },
        "book_airline_casm" => MetricDefinition {
            case: CaseKey::Delta,
            metric_entity: "Delta Air Lines consolidated operations, including regional carriers under capacity purchase agreements",
            formula: "CASM=总运营成本/可用座英里×100美分/美元",
            inputs: &["total_operating_expense", "available_seat_miles", "reported_casm"],
            audited: false,
            audit_boundary: "Form 10-K PDF第42页经营统计；不声称该经营指标获得单独鉴证",
            value_key: "cost_per_available_seat_mile",
            value_unit: "US cents per ASM",
            note: "发行人报告的FY2024总CASM为19.30美分；不是剔除燃油等项目后的非GAAP CASM-Ex。",
        },
        "book_airline_load_factor" => MetricDefinition {
            case: CaseKey::Delta,
            metric_entity: "Delta Air Lines consolidated operations, including regional carriers under capacity purchase agreements",
            formula: "客座率=收入客英里(RPM)/可用座英里(ASM)",
            inputs: &["revenue_passenger_miles", "available_seat_miles", "reported_load_factor"],
            audited: false,
            audit_boundary: "Form 10-K PDF第42页经营统计；不声称该经营指标获得单独鉴证",
            value_key: "passenger_load_factor",
            value_unit: "fraction",
            note: "FY2024合并客座率为85%；发行人说明合并口径包括容量购买协议下的区域承运人。",
        },
        "book_airline_rasm" => MetricDefinition {
            case: CaseKey::Delta,
            metric_entity: "Delta Air Lines consolidated operations, including regional carriers under capacity purchase agreements",
            formula: "RASM=总运营收入/可用座英里×100美分/美元",
            inputs: &["total_operating_revenue", "available_seat_miles", "reported_trasm"],
            audited: false,
            audit_boundary: "Form 10-K PDF第42页经营统计；不声称该经营指标获得单独鉴证",
            value_key: "total_revenue_per_available_seat_mile",
            value_unit: "US cents per ASM",
            note: "使用发行人报告的总运营收入与ASM复算21.374%左右，并与表内展示TRASM 21.37美分交叉核对；不是仅含旅客收入的PRASM。",
        },
        "book_bank_cet1_ratio" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Bank",
            formula: "CET1资本充足率=核心一级资本/风险加权资产；本案例采用发行人报告值",
            inputs: &["core_tier_1_capital_adequacy_ratio"],
            audited: false,
            audit_boundary: "PDF第4页五年经营摘要；不同于偿付能力资本所引用的已审计财务报表附注",
            value_key: "cet1_capital_adequacy_ratio",
            value_unit: "fraction",
            note: "Ping An Bank FY2024核心一级资本充足率为9.12%；不把集团或保险子公司的资本口径混入银行指标。",
        },
        "book_bank_provision_coverage" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Bank",
            formula: "拨备覆盖率=贷款减值准备/不良贷款；本案例采用发行人报告值",
            inputs: &["provision_coverage_ratio"],
            audited: false,
            audit_boundary: "PDF第4页五年经营摘要；不同于偿付能力资本所引用的已审计财务报表附注",
            value_key: "provision_coverage_ratio",
            value_unit: "fraction",
            note: "Ping An Bank FY2024拨备覆盖率为250.71%。",
        },
        "book_insurance_combined_ratio" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Property & Casualty Insurance Company of China, Ltd.",
            formula: "综合成本率=赔付率+费用率；本案例采用发行人报告值",
            inputs: &["property_casualty_combined_ratio"],
            audited: false,
            audit_boundary: "PDF第4页五年经营摘要；不声称该经营比率获得单独审计保证",
            value_key: "combined_ratio",
            value_unit: "fraction",
            note: "FY2024财产险综合成本率为98.3%；不是寿险或集团合并口径。",
        },
        "book_insurance_nbv" => MetricDefinition {
            case: CaseKey::PingAn,
            metric_entity: "Ping An Life and Health insurance business",
            formula: "NBV=发行人精算假设下报告期新业务价值；本案例采用发行人报告值",
            inputs: &["life_health_new_business_value"],
            audited: false,
            audit_boundary: "PDF第4页五年经营摘要；发行人注明2024年假设变更且本案例不将该指标视为已审计GAAP金额",
            value_key: "new_business_value",
            value_unit: "CNY millions",
            note: "采用发行人按2024年更新后的长期投资回报率和风险贴现率披露的28,534百万元；不与同页为同比可比而按2023年假设重算的40,024百万元混用。",
        },
        "book_reit_affo" => MetricDefinition {
            case: CaseKey::RealtyIncome,
            metric_entity: "Realty Income Corporation common stockholders",
            formula: "AFFO=Normalized FFO加减发行人列示的经常性和非现金调整；本案例采用发行人报告值",
            inputs: &["affo_available_to_common_stockholders"],
            audited: false,
            audit_boundary: "发行人将补充AFFO调节表标注为未经审计",
            value_key: "affo_available_to_common_stockholders",
            value_unit: "USD thousands",
            note: "FY2024归属于普通股股东的AFFO为3,621,437千美元；不使用稀释后AFFO或每股AFFO。",
        },
        "book_reit_cap_rate" => MetricDefinition {
            case: CaseKey::RealtyIncome,
            metric_entity: "Realty Income occupied properties disposed during FY2024",
            formula: "净现金资本化率=年化当月合同现金NOI/净出售所得；本案例采用发行人报告值",
            inputs: &["disposition_net_cash_cap_rate"],
            audited: false,
            audit_boundary: "发行人在未经审计补充资料中将净现金资本化率列为补充经营指标",
            value_key: "net_cash_capitalization_rate",
            value_unit: "fraction",
            note: "7.2%仅对应FY2024已出租处置物业，不代表期末整个组合、收购或市场资本化率。",
        },
        "book_reit_ffo" => MetricDefinition {
            case: CaseKey::RealtyIncome,
            metric_entity: "Realty Income Corporation common stockholders",
            formula: "FFO=普通股股东净利润+房地产折旧摊销+减值-房地产出售收益等NAREIT调整；本案例采用发行人报告值",
            inputs: &["ffo_available_to_common_stockholders"],
            audited: false,
            audit_boundary: "发行人将补充FFO调节表标注为未经审计",
            value_key: "ffo_available_to_common_stockholders",
            value_unit: "USD thousands",
            note: "FY2024归属于普通股股东的FFO为3,467,659千美元；不与Normalized FFO、稀释后FFO或每股FFO混用。",
        },
        "book_retail_same_store_sales_growth" => MetricDefinition {
            case: CaseKey::Costco,
            metric_entity: "Costco Wholesale Corporation total company comparable locations",
            formula: "同店销售增长=可比门店本期销售/上年同期销售-1；本案例采用发行人报告值",
            inputs: &["total_company_comparable_sales_growth"],
            audited: false,
            audit_boundary: "发行人业绩公告补充经营指标；不声称获得审计保证",
            value_key: "same_store_sales_growth",
            value_unit: "fraction",
            note: "FY2024 52周Total Company可比销售增长为5.3%，对应可比地点与可比零售周；未采用剔除汽油价格和汇率影响后的5.9%调整值。",
        },
        "book_biopharma_cash_runway" => MetricDefinition {
            case: CaseKey::Moderna,
            metric_entity: "Moderna, Inc.及合并子公司",
            formula: "现金跑道月数=期末现金、现金等价物及投资/(年度经营现金净流出/12)",
            inputs: &["cash_and_investments", "annual_operating_cash_burn"],
            audited: true,
            audit_boundary: "年报PDF物理第122–123页流动性分析引用经审计财务报表金额；跑道月数为本案例推导值",
            value_key: "cash_runway_months",
            value_unit: "months",
            note: "以2024年实际经营现金净流出作静态月均燃烧率；未预测收入、资本开支或2025年降本，因此不是管理层持续经营预测。",
        },
        "book_energy_lifting_cost" => MetricDefinition {
            case: CaseKey::Petrobras,
            metric_entity: "Petróleo Brasileiro S.A.巴西勘探与生产业务",
            formula: "Lifting Cost=发行人报告的每桶油当量开采成本（不含政府分成及租赁）",
            inputs: &["reported_lifting_cost"], audited: false,
            audit_boundary: "管理报告PDF物理第78页经营指标；不声称获得财务报表审计保证",
            value_key: "lifting_cost", value_unit: "USD per boe",
            note: "2024年巴西口径为6.05美元/boe，明确不含政府分成及租赁，不能与含租赁或全成本口径直接比较。",
        },
        "book_energy_reserve_life" => MetricDefinition {
            case: CaseKey::Petrobras,
            metric_entity: "Petróleo Brasileiro S.A.按SEC口径的油气业务",
            formula: "储量寿命=已探明储量/年度产量；本案例采用发行人报告R/P",
            inputs: &["reported_reserve_life"], audited: false,
            audit_boundary: "管理报告PDF物理第79页经营指标；发行人称至少90%的SEC已探明储量接受独立评价",
            value_key: "reserve_life", value_unit: "years",
            note: "13.2年对应2024-12-31按SEC口径的已探明储量与产量关系，不是所有资源量或预测寿命。",
        },
        "book_gold_aisc" => MetricDefinition {
            case: CaseKey::Barrick,
            metric_entity: "Barrick Gold Corporation应占黄金业务",
            formula: "AISC=发行人报告的应占黄金全部维持成本/售出盎司",
            inputs: &["reported_gold_aisc"], audited: false,
            audit_boundary: "年度报告PDF物理第30页非GAAP经营指标；不声称获得独立审计保证",
            value_key: "gold_aisc", value_unit: "USD per ounce",
            note: "2024年1,350美元/盎司是Barrick应占黄金AISC，定义及调节以发行人非GAAP注释为准。",
        },
        "book_industrial_backlog" => MetricDefinition {
            case: CaseKey::Siemens, metric_entity: "Siemens AG持续经营业务",
            formula: "Backlog=报告期末尚未转化为收入的订单积压；本案例采用发行人报告值",
            inputs: &["order_backlog"], audited: false,
            audit_boundary: "年度报告PDF物理第15页管理层经营指标；不声称订单积压获得单独审计保证",
            value_key: "order_backlog", value_unit: "EUR billions",
            note: "截至2024-09-30订单积压1130亿欧元；不是当年新订单，也不是合同负债。",
        },
        "book_industrial_book_to_bill" => MetricDefinition {
            case: CaseKey::Siemens, metric_entity: "Siemens AG持续经营业务",
            formula: "Book-to-Bill=订单/收入",
            inputs: &["orders", "revenue"], audited: false,
            audit_boundary: "年度报告PDF物理第15页管理层经营指标；订单不等同已审计收入",
            value_key: "book_to_bill", value_unit: "ratio",
            note: "FY2024订单840.56亿欧元除以收入约759.30亿欧元，复算约1.107，与发行人展示1.11一致。",
        },
        "book_internet_arpu" => MetricDefinition {
            case: CaseKey::Spotify, metric_entity: "Spotify Premium用户",
            formula: "Premium ARPU=发行人披露的Q3每用户平均收入",
            inputs: &["premium_arpu"], audited: false,
            audit_boundary: "Q3 2020股东信第4页经营指标，相关中期财务报表未经审计",
            value_key: "premium_arpu", value_unit: "EUR per Premium user per month",
            note: "4.19欧元是Q3 2020 Premium ARPU；发行人说明同比下降10%，且汇率及产品、地区组合会影响口径。它不是广告支持用户收入，也不是季度总额。",
        },
        "book_internet_dau_mau" => MetricDefinition {
            case: CaseKey::Meta, metric_entity: "Meta Family of Apps全球用户",
            formula: "DAP/MAP=每日活跃人数/月活跃人数",
            inputs: &["family_dap", "family_map"], audited: false,
            audit_boundary: "Q3 2023业绩演示PDF物理第10–11页估计用户指标；不声称获得审计保证",
            value_key: "daily_monthly_active_ratio", value_unit: "fraction",
            note: "2023年9月Family DAP 31.4亿、MAP 39.6亿；发行人展示比率为取整后的79%。",
        },
        "book_saas_cac_payback" => MetricDefinition {
            case: CaseKey::Similarweb, metric_entity: "Similarweb Ltd.客户获取活动",
            formula: "CAC回收期区间=发行人披露的当前21–22个月；分别保留下限与上限",
            inputs: &["cac_payback_lower_bound", "cac_payback_upper_bound"], audited: false,
            audit_boundary: "Q3 2024投资者演示PDF物理第22页非GAAP推导区间；不声称获得审计保证",
            value_key: "cac_payback_lower_bound", value_unit: "months lower bound",
            note: "发行人披露当前CAC回收期为21–22个月；结果同时保留下限与上限，不把21个月、22个月或区间中点说成精确回收期。",
        },
        "book_saas_churn" => MetricDefinition {
            case: CaseKey::Zoom, metric_entity: "Zoom Video Communications, Inc. Online客户",
            formula: "月均流失率=发行人报告Online Average Monthly Churn",
            inputs: &["online_monthly_churn"], audited: false,
            audit_boundary: "FY2025 Q1准备稿PDF物理第5页经营指标；不声称获得审计保证",
            value_key: "monthly_customer_churn", value_unit: "fraction per month",
            note: "3.2%仅对应Zoom Online自助客户；报告说明收紧未付款宽限期使部分流失提前。",
        },
        "book_saas_nrr" => MetricDefinition {
            case: CaseKey::Snowflake, metric_entity: "Snowflake使用容量合同的客户群组",
            formula: "NRR=同一客户群第二年产品收入/第一年产品收入；本案例采用发行人报告值",
            inputs: &["net_revenue_retention"], audited: false,
            audit_boundary: "FY2024 Q4投资者演示PDF物理第21页经营指标；不声称获得审计保证",
            value_key: "net_revenue_retention", value_unit: "fraction",
            note: "FY2024 Q4 NRR为131%，基于Snowflake容量合同客户群及其披露的两年测量定义。",
        },
        "book_semiconductor_asp" => MetricDefinition {
            case: CaseKey::Smic, metric_entity: "中芯国际晶圆代工业务",
            formula: "隐含每片晶圆收入=总收入×晶圆服务收入占比/8英寸标准逻辑等效晶圆出货量",
            inputs: &["revenue", "wafer_revenue_share", "wafer_shipments"], audited: false,
            audit_boundary: "Q4 2024业绩公告PDF物理第5页未经审计季度经营数据",
            value_key: "implied_revenue_per_equivalent_wafer", value_unit: "USD per 8-inch-equivalent wafer",
            note: "这是按披露收入结构推导的等效晶圆平均收入，不是单颗芯片售价；产品组合与8/12英寸换算会影响结果。",
        },
        "book_semiconductor_utilization" => MetricDefinition {
            case: CaseKey::Smic, metric_entity: "中芯国际全部晶圆厂",
            formula: "产能利用率=总晶圆产出/估算季度总产能；本案例采用发行人报告值",
            inputs: &["utilization_rate"], audited: false,
            audit_boundary: "Q4 2024业绩公告PDF物理第5页未经审计经营指标",
            value_key: "capacity_utilization", value_unit: "fraction",
            note: "Q4 2024利用率85.5%，分母是发行人估算季度总产能，不是期末月产能乘三。",
        },
        "book_shipping_tce" => MetricDefinition {
            case: CaseKey::Frontline, metric_entity: "Frontline plc VLCC现货船队",
            formula: "TCE=航次收入扣航次费用后/可用营运天数；本案例采用发行人报告日均值",
            inputs: &["vlcc_spot_tce"], audited: false,
            audit_boundary: "Q2 2024投资者演示经营指标，发行人明确标注为非IFRS",
            value_key: "vlcc_spot_tce", value_unit: "USD per day",
            note: "49,600美元/天是Q2 2024 VLCC现货TCE；不代表Suezmax、LR2/Aframax或未来已锁定费率。",
        },
        "book_telecom_arpu" => MetricDefinition {
            case: CaseKey::Verizon, metric_entity: "Verizon Consumer无线零售预付费连接",
            formula: "ARPU=预付费服务收入/平均预付费连接数；本案例采用发行人报告值",
            inputs: &["prepaid_arpu"], audited: false,
            audit_boundary: "Q1 2024补充经营统计PDF物理第7页，未经审计",
            value_key: "prepaid_arpu", value_unit: "USD per connection per month",
            note: "31.17美元是Consumer预付费每连接ARPU，包含SafeLink；不是后付费每账户ARPA。",
        },
        "book_telecom_churn" => MetricDefinition {
            case: CaseKey::Verizon, metric_entity: "Verizon Consumer无线零售预付费连接",
            formula: "月流失率=当期断开连接数/平均连接数；本案例采用发行人报告值",
            inputs: &["prepaid_churn"], audited: false,
            audit_boundary: "Q1 2024补充经营统计PDF物理第6页，未经审计",
            value_key: "prepaid_monthly_churn", value_unit: "fraction per month",
            note: "4.26%是Consumer预付费连接月流失率，包含SafeLink；与后付费手机流失率口径不同。",
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
        (
            &cases.moutai,
            (MOUTAI_URL, MOUTAI_SHA256, MOUTAI_BYTES, "600519"),
        ),
        (&cases.delta, (DELTA_URL, DELTA_SHA256, DELTA_BYTES, "DAL")),
        (
            &cases.costco,
            (COSTCO_URL, COSTCO_SHA256, COSTCO_BYTES, "COST"),
        ),
        (
            &cases.moderna,
            (MODERNA_URL, MODERNA_SHA256, MODERNA_BYTES, "MRNA"),
        ),
        (
            &cases.petrobras,
            (PETROBRAS_URL, PETROBRAS_SHA256, PETROBRAS_BYTES, "PBR"),
        ),
        (
            &cases.barrick,
            (BARRICK_URL, BARRICK_SHA256, BARRICK_BYTES, "GOLD"),
        ),
        (
            &cases.siemens,
            (SIEMENS_URL, SIEMENS_SHA256, SIEMENS_BYTES, "SIE.DE"),
        ),
        (&cases.meta, (META_URL, META_SHA256, META_BYTES, "META")),
        (
            &cases.spotify,
            (SPOTIFY_URL, SPOTIFY_SHA256, SPOTIFY_BYTES, "SPOT"),
        ),
        (
            &cases.snowflake,
            (SNOWFLAKE_URL, SNOWFLAKE_SHA256, SNOWFLAKE_BYTES, "SNOW"),
        ),
        (
            &cases.similarweb,
            (SIMILARWEB_URL, SIMILARWEB_SHA256, SIMILARWEB_BYTES, "SMWB"),
        ),
        (&cases.zoom, (ZOOM_URL, ZOOM_SHA256, ZOOM_BYTES, "ZM")),
        (&cases.smic, (SMIC_URL, SMIC_SHA256, SMIC_BYTES, "0981.HK")),
        (
            &cases.verizon,
            (VERIZON_URL, VERIZON_SHA256, VERIZON_BYTES, "VZ"),
        ),
        (
            &cases.frontline,
            (FRONTLINE_URL, FRONTLINE_SHA256, FRONTLINE_BYTES, "FRO"),
        ),
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
        CaseKey::Moutai => &cases.moutai,
        CaseKey::Delta => &cases.delta,
        CaseKey::Costco => &cases.costco,
        CaseKey::Moderna => &cases.moderna,
        CaseKey::Petrobras => &cases.petrobras,
        CaseKey::Barrick => &cases.barrick,
        CaseKey::Siemens => &cases.siemens,
        CaseKey::Meta => &cases.meta,
        CaseKey::Spotify => &cases.spotify,
        CaseKey::Snowflake => &cases.snowflake,
        CaseKey::Similarweb => &cases.similarweb,
        CaseKey::Zoom => &cases.zoom,
        CaseKey::Smic => &cases.smic,
        CaseKey::Verizon => &cases.verizon,
        CaseKey::Frontline => &cases.frontline,
    }
}

fn select_source(registry: &IndustrySourceRegistry, key: CaseKey) -> &IndustrySourceConfig {
    match key {
        CaseKey::PingAn => &registry.ping_an,
        CaseKey::Shopify => &registry.shopify,
        CaseKey::RealtyIncome => &registry.realty_income,
        CaseKey::Ebay => &registry.ebay,
        CaseKey::Moutai => &registry.moutai,
        CaseKey::Delta => &registry.delta,
        CaseKey::Costco => &registry.costco,
        CaseKey::Moderna => &registry.moderna,
        CaseKey::Petrobras => &registry.petrobras,
        CaseKey::Barrick => &registry.barrick,
        CaseKey::Siemens => &registry.siemens,
        CaseKey::Meta => &registry.meta,
        CaseKey::Spotify => &registry.spotify,
        CaseKey::Snowflake => &registry.snowflake,
        CaseKey::Similarweb => &registry.similarweb,
        CaseKey::Zoom => &registry.zoom,
        CaseKey::Smic => &registry.smic,
        CaseKey::Verizon => &registry.verizon,
        CaseKey::Frontline => &registry.frontline,
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
    let archived_marker = directory.join(format!("{}.archived-original", expected.sha256));
    let is_zoom = expected.sha256 == ZOOM_SHA256 && expected.bytes == ZOOM_BYTES;
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        if bytes.len() == expected.bytes && fingerprint(&bytes) == expected.sha256 {
            if archived_marker.exists() {
                if tokio::fs::read(&archived_marker)
                    .await
                    .is_ok_and(|marker| marker == expected.sha256.as_bytes())
                    && (!is_zoom
                        || (bytes.len() == ZOOM_ARCHIVED_ORIGINAL.len()
                            && fingerprint(&bytes) == ZOOM_SHA256))
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
            .map_err(|error| format!("corrupt industry source cache removal failed: {error}"))?;
        let _ = tokio::fs::remove_file(&archived_marker).await;
    }
    let download = async {
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
        if !is_zoom
            && response
                .content_length()
                .is_some_and(|size| size != expected.bytes as u64)
        {
            return Err("industry source Content-Length differs from the reviewed document".into());
        }
        if is_zoom
            && response
                .content_length()
                .is_some_and(|size| size > 20_000_000)
        {
            return Err("industry source body exceeds safety limit".into());
        }
        let mut bytes = Vec::with_capacity(expected.bytes);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| format!("industry source body failed: {error}"))?;
            let limit = if is_zoom { 20_000_000 } else { expected.bytes };
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(if is_zoom {
                    "industry source body exceeds safety limit".into()
                } else {
                    "industry source body exceeds the reviewed document size".into()
                });
            }
            bytes.extend_from_slice(&chunk);
        }
        if is_zoom && !bytes.starts_with(b"%PDF-") {
            return Err("Zoom original endpoint did not return a PDF".into());
        }
        if bytes.len() != expected.bytes || fingerprint(&bytes) != expected.sha256 {
            return Err(
                "downloaded industry source is incomplete or has a different SHA-256".into(),
            );
        }
        Ok::<Vec<u8>, String>(bytes)
    };
    let (bytes, status, archived) = match download.await {
        Ok(bytes) => (bytes, "verified_then_cached", false),
        Err(error)
            if is_zoom
                && (error.starts_with("industry source request failed")
                    || error.starts_with("industry source returned an error")
                    || error.starts_with("industry source body failed")
                    || error == "Zoom original endpoint did not return a PDF") =>
        {
            if ZOOM_ARCHIVED_ORIGINAL.len() != expected.bytes
                || fingerprint(ZOOM_ARCHIVED_ORIGINAL) != expected.sha256
            {
                return Err(format!(
                    "archived Zoom original differs from reviewed evidence after live failure: {error}"
                ));
            }
            (
                ZOOM_ARCHIVED_ORIGINAL.to_vec(),
                "verified_archived_original",
                true,
            )
        }
        Err(error) => return Err(error),
    };
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| format!("industry source cache directory failed: {error}"))?;
    tokio::fs::write(&cached, &bytes)
        .await
        .map_err(|error| format!("industry source cache write failed: {error}"))?;
    if archived {
        tokio::fs::write(&archived_marker, expected.sha256.as_bytes())
            .await
            .map_err(|error| format!("industry source archived marker failed: {error}"))?;
    } else {
        let _ = tokio::fs::remove_file(&archived_marker).await;
    }
    Ok(status)
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
        "book_share_counts" => {
            let opening = n("opening_total_shares")?;
            let cancelled = n("cancelled_shares")?;
            let closing = n("closing_total_shares")?;
            let unrestricted = n("unrestricted_shares")?;
            if opening - cancelled != closing || unrestricted > closing {
                return Err("share structure does not reconcile".into());
            }
            Ok(closing - unrestricted)
        }
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
        "book_airline_casm" => {
            Ok(ratio(n("total_operating_expense")?, n("available_seat_miles")?)? * 100.0)
        }
        "book_airline_load_factor" => {
            ratio(n("revenue_passenger_miles")?, n("available_seat_miles")?)
        }
        "book_airline_rasm" => {
            Ok(ratio(n("total_operating_revenue")?, n("available_seat_miles")?)? * 100.0)
        }
        "book_bank_cet1_ratio" => n("core_tier_1_capital_adequacy_ratio"),
        "book_bank_provision_coverage" => n("provision_coverage_ratio"),
        "book_insurance_combined_ratio" => n("property_casualty_combined_ratio"),
        "book_insurance_nbv" => n("life_health_new_business_value"),
        "book_reit_affo" => n("affo_available_to_common_stockholders"),
        "book_reit_cap_rate" => n("disposition_net_cash_cap_rate"),
        "book_reit_ffo" => n("ffo_available_to_common_stockholders"),
        "book_retail_same_store_sales_growth" => n("total_company_comparable_sales_growth"),
        "book_biopharma_cash_runway" => {
            Ok(ratio(n("cash_and_investments")?, n("annual_operating_cash_burn")?)? * 12.0)
        }
        "book_energy_lifting_cost" => n("reported_lifting_cost"),
        "book_energy_reserve_life" => n("reported_reserve_life"),
        "book_gold_aisc" => n("reported_gold_aisc"),
        "book_industrial_backlog" => n("order_backlog"),
        "book_industrial_book_to_bill" => ratio(n("orders")?, n("revenue")?),
        "book_internet_arpu" => n("premium_arpu"),
        "book_internet_dau_mau" => ratio(n("family_dap")?, n("family_map")?),
        "book_saas_cac_payback" => n("cac_payback_lower_bound"),
        "book_saas_churn" => n("online_monthly_churn"),
        "book_saas_nrr" => n("net_revenue_retention"),
        "book_semiconductor_asp" => Ok(ratio(
            n("revenue")? * n("wafer_revenue_share")? * 1_000.0,
            n("wafer_shipments")?,
        )?),
        "book_semiconductor_utilization" => n("utilization_rate"),
        "book_shipping_tce" => n("vlcc_spot_tce"),
        "book_telecom_arpu" => n("prepaid_arpu"),
        "book_telecom_churn" => n("prepaid_churn"),
        _ => Err(format!("unsupported industry case concept: {id}")),
    }
}

fn formula_symbol_mapping(id: &str) -> Value {
    match id {
        "book_share_counts" => json!({
            "restricted_shares_residual": "closing_total_shares - unrestricted_shares",
            "closing_total_shares": "opening_total_shares - cancelled_shares"
        }),
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
        "book_airline_casm" => {
            json!({"total_operating_expense": "total_operating_expense", "available_seat_miles": "available_seat_miles"})
        }
        "book_airline_load_factor" => {
            json!({"revenue_passenger_miles": "revenue_passenger_miles", "available_seat_miles": "available_seat_miles"})
        }
        "book_airline_rasm" => {
            json!({"total_operating_revenue": "total_operating_revenue", "available_seat_miles": "available_seat_miles"})
        }
        "book_bank_cet1_ratio" => {
            json!({"reported_cet1_ratio": "core_tier_1_capital_adequacy_ratio"})
        }
        "book_bank_provision_coverage" => {
            json!({"reported_provision_coverage": "provision_coverage_ratio"})
        }
        "book_insurance_combined_ratio" => {
            json!({"reported_combined_ratio": "property_casualty_combined_ratio"})
        }
        "book_insurance_nbv" => json!({"reported_nbv": "life_health_new_business_value"}),
        "book_reit_affo" => json!({"reported_affo": "affo_available_to_common_stockholders"}),
        "book_reit_cap_rate" => json!({"reported_cap_rate": "disposition_net_cash_cap_rate"}),
        "book_reit_ffo" => json!({"reported_ffo": "ffo_available_to_common_stockholders"}),
        "book_retail_same_store_sales_growth" => {
            json!({"reported_same_store_sales_growth": "total_company_comparable_sales_growth"})
        }
        "book_biopharma_cash_runway" => {
            json!({"cash_balance": "cash_and_investments", "monthly_cash_burn": "annual_operating_cash_burn / 12"})
        }
        "book_energy_lifting_cost" => json!({"reported_lifting_cost": "reported_lifting_cost"}),
        "book_energy_reserve_life" => json!({"reported_reserve_life": "reported_reserve_life"}),
        "book_gold_aisc" => json!({"reported_gold_aisc": "reported_gold_aisc"}),
        "book_industrial_backlog" => json!({"reported_order_backlog": "order_backlog"}),
        "book_industrial_book_to_bill" => {
            json!({"new_orders": "orders", "recognized_revenue": "revenue"})
        }
        "book_internet_arpu" => json!({"reported_premium_arpu": "premium_arpu"}),
        "book_internet_dau_mau" => {
            json!({"daily_active_people": "family_dap", "monthly_active_people": "family_map"})
        }
        "book_saas_cac_payback" => json!({
            "reported_range_lower_bound": "cac_payback_lower_bound",
            "reported_range_upper_bound": "cac_payback_upper_bound"
        }),
        "book_saas_churn" => json!({"reported_online_monthly_churn": "online_monthly_churn"}),
        "book_saas_nrr" => json!({"reported_nrr": "net_revenue_retention"}),
        "book_semiconductor_asp" => {
            json!({"wafer_revenue": "revenue * wafer_revenue_share", "equivalent_wafer_shipments": "wafer_shipments"})
        }
        "book_semiconductor_utilization" => json!({"reported_utilization": "utilization_rate"}),
        "book_shipping_tce" => json!({"reported_vlcc_spot_tce": "vlcc_spot_tce"}),
        "book_telecom_arpu" => json!({"reported_prepaid_arpu": "prepaid_arpu"}),
        "book_telecom_churn" => json!({"reported_prepaid_churn": "prepaid_churn"}),
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
    let retrieval_note = (cache_status == "verified_archived_original").then_some(
        "Zoom 原站请求未返回已核验 PDF；本案例使用字节数与 SHA-256 完全相同的已核验原文备份。",
    );
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
    let mut values = json!({definition.value_key: value});
    let mut units = json!({definition.value_key: definition.value_unit});
    if id == "book_saas_cac_payback" {
        let upper = case.facts["cac_payback_upper_bound"].value;
        values["cac_payback_upper_bound"] = json!(upper);
        units["cac_payback_upper_bound"] = json!("months upper bound");
    }
    pdf_pages.sort_unstable();
    pdf_pages.dedup();
    Ok(json!({
        "concept_id": id,
        "input_kind": "industry_case",
        "provenance": "verified_original_issuer_disclosure",
        "status": "computed",
        "reason": Value::Null,
        "values": values,
        "units": units,
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
                "matched_bytes": source.bytes,
                "retrieval_note": retrieval_note
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
            &mut registry.moutai,
            &mut registry.delta,
            &mut registry.costco,
            &mut registry.moderna,
            &mut registry.petrobras,
            &mut registry.barrick,
            &mut registry.siemens,
            &mut registry.meta,
            &mut registry.spotify,
            &mut registry.snowflake,
            &mut registry.similarweb,
            &mut registry.zoom,
            &mut registry.smic,
            &mut registry.verizon,
            &mut registry.frontline,
        ] {
            let bytes = format!("%PDF fixture {}", source.url).into_bytes();
            source.bytes = bytes.len();
            source.sha256 = fingerprint(&bytes);
            std::fs::write(directory.join(format!("{}.pdf", source.sha256)), bytes).unwrap();
        }
        registry
    }

    #[tokio::test]
    #[allow(clippy::approx_constant)] // 3.14 is Meta's reported DAP literal, not an approximation of PI.
    async fn supported_cases_match_independent_disclosure_values() {
        let root = std::env::temp_dir().join(format!("axiom-industry-{}", uuid::Uuid::new_v4()));
        let registry = seeded_registry(&root);
        let expected = [
            ("book_share_counts", "restricted_shares_residual", 0.0),
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
            (
                "book_airline_casm",
                "cost_per_available_seat_mile",
                55_648.0 / 288_394.0 * 100.0,
            ),
            (
                "book_airline_load_factor",
                "passenger_load_factor",
                246_145.0 / 288_394.0,
            ),
            (
                "book_airline_rasm",
                "total_revenue_per_available_seat_mile",
                61_643.0 / 288_394.0 * 100.0,
            ),
            (
                "book_bank_cet1_ratio",
                "cet1_capital_adequacy_ratio",
                0.0912,
            ),
            (
                "book_bank_provision_coverage",
                "provision_coverage_ratio",
                2.5071,
            ),
            ("book_insurance_combined_ratio", "combined_ratio", 0.983),
            ("book_insurance_nbv", "new_business_value", 28_534.0),
            (
                "book_reit_affo",
                "affo_available_to_common_stockholders",
                3_621_437.0,
            ),
            ("book_reit_cap_rate", "net_cash_capitalization_rate", 0.072),
            (
                "book_reit_ffo",
                "ffo_available_to_common_stockholders",
                3_467_659.0,
            ),
            (
                "book_retail_same_store_sales_growth",
                "same_store_sales_growth",
                0.053,
            ),
            (
                "book_biopharma_cash_runway",
                "cash_runway_months",
                9_519.0 / 3_004.0 * 12.0,
            ),
            ("book_energy_lifting_cost", "lifting_cost", 6.05),
            ("book_energy_reserve_life", "reserve_life", 13.2),
            ("book_gold_aisc", "gold_aisc", 1_350.0),
            ("book_industrial_backlog", "order_backlog", 113.0),
            (
                "book_industrial_book_to_bill",
                "book_to_bill",
                84_056.0 / 75_930.0,
            ),
            ("book_internet_arpu", "premium_arpu", 4.19),
            (
                "book_internet_dau_mau",
                "daily_monthly_active_ratio",
                3.14 / 3.96,
            ),
            ("book_saas_cac_payback", "cac_payback_lower_bound", 21.0),
            ("book_saas_churn", "monthly_customer_churn", 0.032),
            ("book_saas_nrr", "net_revenue_retention", 1.31),
            (
                "book_semiconductor_asp",
                "implied_revenue_per_equivalent_wafer",
                2_207_281.0 * 0.925 * 1_000.0 / 1_991_761.0,
            ),
            (
                "book_semiconductor_utilization",
                "capacity_utilization",
                0.855,
            ),
            ("book_shipping_tce", "vlcc_spot_tce", 49_600.0),
            ("book_telecom_arpu", "prepaid_arpu", 31.17),
            ("book_telecom_churn", "prepaid_monthly_churn", 0.0426),
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
        assert!(evaluate("not_a_concept", &root, &registry).await.is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn share_structure_rejects_nonreconciling_facts_and_teaching_inputs() {
        let mut case = cases().unwrap().moutai.clone();
        case.facts.get_mut("cancelled_shares").unwrap().value += 1.0;
        assert!(calculate("book_share_counts", &case).is_err());
        case = cases().unwrap().moutai.clone();
        case.facts.get_mut("unrestricted_shares").unwrap().value += 1.0;
        assert!(calculate("book_share_counts", &case).is_err());
        case.facts.remove("closing_total_shares");
        assert!(calculate("book_share_counts", &case).is_err());
        assert!(crate::book::evaluate(
            "book_share_counts",
            &[],
            &json!({"free_float_shares": 6e8, "total_shares": 1e9})
        )
        .is_err());
        let entry = crate::knowledge::all_entries()
            .into_iter()
            .find(|e| e.id == "book_share_counts")
            .unwrap();
        assert!(!entry.formula.contains("自由流通"));
        assert!(entry.summary.contains("2025"));
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
        assert!(is_supported("book_gold_aisc"));
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
