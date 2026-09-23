//! 第一至十一章的可复算教学练习。coverage.json 保留每一条源记录和其可执行边界。
use crate::knowledge::KnowledgeEntry;
use crate::types::Bar;
use chrono::{Timelike, Utc};
use serde_json::{json, Value};

pub fn entries() -> Vec<KnowledgeEntry> {
    let items = [
        (
            "book_current_ratio",
            "流动比率",
            "流动资产 / 流动负债",
            "短期偿债缓冲",
            "分子和分母必须来自同一报告期；高比率也可能是存货积压。",
        ),
        (
            "book_quick_ratio",
            "速动比率",
            "(现金 + 应收款 + 短期投资) / 流动负债",
            "剔除较难变现存货后的短期偿债能力",
            "应收账款质量和账龄会让速动资产高估。",
        ),
        (
            "book_interest_coverage",
            "利息保障倍数",
            "EBIT / 利息费用",
            "经营利润覆盖利息支出的程度",
            "EBIT 和利息必须采用一致口径，亏损企业不适用。",
        ),
        (
            "book_inventory_turnover",
            "存货周转率",
            "销售成本 / 平均存货",
            "存货在期间内周转的速度",
            "季节性业务应使用平均存货并比较同业。",
        ),
        (
            "book_receivable_turnover",
            "应收账款周转率",
            "营业收入 / 平均应收账款",
            "销售回款效率",
            "收入确认激进会同时抬高收入和应收。",
        ),
        (
            "book_ptbv",
            "有形市净率 P/TBV",
            "市值 / (净资产 - 商誉 - 无形资产)",
            "扣除商誉和无形资产后的账面估值",
            "有形净资产非正时没有经济可比性。",
        ),
        (
            "book_price_cashflow",
            "市现率 P/OCF 与 P/FCF",
            "市值/经营现金流；市值/(经营现金流-资本开支)",
            "以现金流衡量的估值倍数",
            "现金流口径必须同一报告期。",
        ),
        (
            "book_ev",
            "企业价值 EV",
            "市值+有息债务+优先股+少数股东权益-现金",
            "收购整个企业的简化资本结构价值",
            "债务与现金口径须一致。",
        ),
        (
            "book_ev_ebit",
            "EV/EBIT",
            "EV / EBIT",
            "经营利润倍数",
            "EBIT 非正时不适用。",
        ),
        (
            "book_ev_sales",
            "EV/Sales",
            "EV / 营业收入",
            "收入倍数",
            "低利润率公司不可只看收入倍数。",
        ),
        (
            "book_dividend_yield",
            "股息率",
            "每股股息 / 股价",
            "现金分红相对买入价格的比例",
            "不是总回报，也不保证未来分红。",
        ),
        (
            "book_payout_ratio",
            "分红支付率",
            "每股股息 / EPS",
            "利润中分给股东的比例",
            "EPS 非正时不适用。",
        ),
        (
            "book_earnings_yield",
            "盈利收益率",
            "EPS / 股价",
            "PE 的倒数，可为负",
            "负收益率并非债券收益率。",
        ),
        (
            "book_fcf_yield",
            "自由现金流收益率",
            "(经营现金流-资本开支) / 市值",
            "股东可用现金相对市值",
            "资本开支分类会影响结果。",
        ),
        (
            "book_nav_discount",
            "净资产值折溢价",
            "股价 / 每股 NAV - 1",
            "价格相对每股净资产值的溢价或折价",
            "NAV 必须是同一时点的每股口径。",
        ),
        (
            "book_gross_margin",
            "毛利与毛利率",
            "营业收入-销售成本；毛利/营业收入",
            "产品层面的初步盈利能力",
            "成本分类不同会影响同业可比性。",
        ),
        (
            "book_ebit_margin",
            "EBIT Margin",
            "EBIT / 营业收入",
            "经营利润率",
            "一次性项目需单独披露。",
        ),
        (
            "book_roa",
            "ROA 总资产收益率",
            "净利润 / 平均总资产",
            "资产产生利润的效率",
            "平均资产应覆盖同一报告期；高杠杆不会直接改善 ROA。",
        ),
    ];
    let mut out: Vec<KnowledgeEntry> = items
        .into_iter()
        .map(|(id, name, formula, meaning, pitfalls)| KnowledgeEntry {
            id: id.into(),
            summary: format!("{}：{}。", name, meaning),
            example: match id {
                "book_current_ratio" => "流动资产 150、流动负债 100，流动比率为 1.5。".into(),
                "book_quick_ratio" => {
                    "现金 20、应收 30、短投 10、流动负债 40，速动比率为 1.5。".into()
                }
                "book_interest_coverage" => "EBIT 120、利息费用 20，利息保障倍数为 6。".into(),
                "book_inventory_turnover" => {
                    "销售成本 600、平均存货 100，存货周转率为 6 次。".into()
                }
                _ => "收入 360、平均应收 60，应收周转率为 6 次。".into(),
            },
            related: vec![],
            code_url: "https://github.com/Sigma711/axiom/blob/main/src/book.rs".into(),
            code_ref: "src/book.rs::evaluate".into(),
            category: "原书补充".into(),
            name: name.into(),
            formula: formula.into(),
            meaning: meaning.into(),
            signals: "先核对口径、时间点和数据来源，再把结果作为描述性证据。".into(),
            pitfalls: pitfalls.into(),
            implementation: "src/book.rs::evaluate".into(),
            diagram: None,
        })
        .collect();
    for (id, name, formula) in EXTRA {
        out.push(KnowledgeEntry {
            id: (*id).into(),
            summary: format!("{}：使用透明教学输入计算。", name),
            example: "示例输入由实践面板提供；结果只反映输入，不声称来自当前币价。".into(),
            related: vec![],
            code_url: "https://github.com/Sigma711/axiom/blob/main/src/book.rs".into(),
            code_ref: "src/book.rs::evaluate".into(),
            category: "原书行情与财务实践".into(),
            name: (*name).into(),
            formula: (*formula).into(),
            meaning: "需要外部披露、盘口或报告期数据时，调用方必须提供来源与同一时点口径。".into(),
            signals: "作为可复算描述，不构成交易建议。".into(),
            pitfalls: "缺失输入、零分母或混合报告期会导致无定义。".into(),
            implementation: "src/book.rs::evaluate".into(),
            diagram: None,
        });
    }
    for (id, name, formula, meaning, example) in INDUSTRY {
        out.push(KnowledgeEntry {
            id: (*id).into(),
            summary: format!("{}：{}。", name, meaning),
            example: (*example).into(),
            related: vec![],
            code_url: "https://github.com/Sigma711/axiom/blob/main/src/book.rs".into(),
            code_ref: "src/book.rs::evaluate".into(),
            category: "原书行业专属指标".into(),
            name: (*name).into(),
            formula: (*formula).into(),
            meaning: (*meaning).into(),
            signals: "仅与同一行业、同一会计口径和同一报告期的样本比较。".into(),
            pitfalls: "监管口径、一次性项目和报告期不一致时不可直接比较。".into(),
            implementation: "src/book.rs::evaluate".into(),
            diagram: None,
        });
    }
    let lessons: Value = serde_json::from_str(include_str!("../docs/book/foundation-lessons.json"))
        .expect("validated foundation lessons");
    for entry in &mut out {
        if let Some(lesson) = lessons.get(&entry.id) {
            entry.meaning = lesson["meaning"].as_str().expect("lesson meaning").into();
            entry.example = lesson["example"].as_str().expect("lesson example").into();
            entry.pitfalls = lesson["pitfalls"].as_str().expect("lesson pitfalls").into();
            entry.summary = entry
                .meaning
                .split('。')
                .next()
                .unwrap_or(&entry.meaning)
                .into();
        }
    }
    out
}

const INDUSTRY: &[(&str, &str, &str, &str, &str)] = &[
    (
        "book_bank_nim",
        "银行净息差 NIM",
        "净利息收入 / 平均生息资产",
        "每单位生息资产产生的净利息收入",
        "净利息收入 2、平均生息资产 100，NIM 为 2%。",
    ),
    (
        "book_bank_npl_ratio",
        "银行不良贷款率 NPL",
        "不良贷款 / 贷款总额",
        "贷款资产质量",
        "不良贷款 1、贷款总额 100，不良率为 1%。",
    ),
    (
        "book_bank_provision_coverage",
        "银行拨备覆盖率",
        "贷款损失准备 / 不良贷款",
        "拨备对已识别不良贷款的缓冲",
        "准备金 3、不良贷款 1，拨备覆盖率为 300%。",
    ),
    (
        "book_bank_cet1_ratio",
        "银行 CET1 资本充足率",
        "核心一级资本 / 风险加权资产",
        "吸收损失的核心资本底座",
        "CET1 12、风险加权资产 100，资本充足率为 12%。",
    ),
    (
        "book_bank_cost_income",
        "银行成本收入比",
        "经营费用 / 营业收入",
        "银行经营效率",
        "经营费用 40、营业收入 100，成本收入比为 40%。",
    ),
    (
        "book_insurance_combined_ratio",
        "保险综合成本率",
        "(赔付额 + 承保费用) / 已赚保费",
        "承保业务本身的盈亏",
        "赔付 60、费用 30、已赚保费 100，综合成本率为 90%。",
    ),
    (
        "book_insurance_solvency_ratio",
        "保险偿付能力充足率",
        "可用资本 / 监管资本要求",
        "可用资本满足监管要求的程度",
        "可用资本 180、监管要求 100，偿付能力充足率为 180%。",
    ),
    (
        "book_insurance_nbv",
        "保险新业务价值 NBV",
        "第1年利润/(1+r) + 第2年利润/(1+r)²",
        "用两期利润折现演示新业务价值；正式精算还需要完整现金流、费用与资本假设",
        "第1年利润 5.5、第2年 12.1、折现率 10%，现值为 5+10=15 元。",
    ),
    (
        "book_reit_ffo",
        "REIT FFO",
        "净利润 + 房地产折旧 - 出售物业收益",
        "更接近物业经营能力的利润",
        "净利润 50、折旧 20、出售收益 5，FFO 为 65。",
    ),
    (
        "book_reit_affo",
        "REIT AFFO",
        "FFO - 维持性资本开支 - 直线租金调整",
        "更接近可分配现金",
        "FFO 65、维持性开支 10、调整 5，AFFO 为 50。",
    ),
    (
        "book_reit_occupancy",
        "REIT 入住率",
        "已出租面积 / 可出租面积",
        "物业空间的出租程度",
        "已出租 90 平方米、可出租 100 平方米，入住率为 90%。",
    ),
    (
        "book_reit_cap_rate",
        "REIT 资本化率 Cap Rate",
        "物业净经营收入 / 物业价值",
        "物业经营收益率",
        "NOI 6、物业价值 100，Cap Rate 为 6%。",
    ),
    (
        "book_saas_arr",
        "SaaS ARR",
        "月度经常性收入 × 12",
        "年度化经常性订阅收入",
        "月度经常性收入 1000 万元，ARR 为 1.2 亿元。",
    ),
    (
        "book_saas_nrr",
        "SaaS NRR",
        "期末存量客户经常性收入 / 期初存量客户经常性收入",
        "不含新客户的收入留存与扩张",
        "期初存量收入 100、期末 110，NRR 为 110%。",
    ),
    (
        "book_saas_churn",
        "SaaS Churn",
        "流失客户数 / 期初客户数",
        "客户或收入流失比例",
        "流失客户 5、期初客户 100，流失率为 5%。",
    ),
    (
        "book_saas_rule_of_40",
        "SaaS Rule of 40",
        "收入增长率 + FCF 利润率",
        "增长和现金流效率的合计",
        "收入增长 30%、FCF 利润率 12%，Rule of 40 为 42%。",
    ),
    (
        "book_saas_cac_payback",
        "SaaS CAC Payback",
        "获客成本 / (月度 ARPU × 毛利率)",
        "客户毛利收回获客投入所需月数",
        "获客成本 1200、月 ARPU 100、毛利率 80%，回收期为 15 个月。",
    ),
    (
        "book_platform_gmv",
        "平台 GMV",
        "期间平台成交金额",
        "平台撮合的商品或服务总额",
        "期间成交 100 亿元，GMV 为 100 亿元。",
    ),
    (
        "book_platform_take_rate",
        "平台 Take Rate",
        "平台收入 / GMV",
        "平台从交易额获得的抽佣比例",
        "GMV 100 亿元、平台收入 5 亿元，抽佣率为 5%。",
    ),
    (
        "book_internet_dau_mau",
        "互联网 DAU/MAU",
        "日活用户 / 月活用户",
        "月内使用频率",
        "DAU 30 万、MAU 100 万，DAU/MAU 为 30%。",
    ),
    (
        "book_internet_arpu",
        "互联网 ARPU",
        "收入 / 活跃用户数",
        "每名活跃用户的平均收入",
        "收入 100 万、活跃用户 10 万，ARPU 为 10 元。",
    ),
    (
        "book_retail_same_store_sales_growth",
        "零售同店销售增长",
        "本期同店销售 / 上期同店销售 - 1",
        "排除新开和关闭门店后的增长",
        "本期同店销售 110、上期 100，增长为 10%。",
    ),
    (
        "book_semiconductor_utilization",
        "半导体产能利用率",
        "实际产量 / 最大产能",
        "产线被实际使用的程度",
        "实际产量 80、最大产能 100，利用率为 80%。",
    ),
    (
        "book_semiconductor_asp",
        "半导体 ASP",
        "芯片销售收入 / 销售数量",
        "每颗芯片的平均售价",
        "收入 100 万、销售 10 万颗，ASP 为 10 元/颗。",
    ),
    (
        "book_industrial_book_to_bill",
        "工业 Book-to-Bill",
        "新订单 / 已确认收入",
        "订单积累速度",
        "新订单 120、已确认收入 100，Book-to-Bill 为 1.2。",
    ),
    (
        "book_industrial_backlog",
        "工业 Backlog",
        "尚未交付订单金额",
        "未来待交付订单储备",
        "待交付订单 300 万元，Backlog 为 300 万元。",
    ),
    (
        "book_energy_reserve_life",
        "能源储量寿命",
        "可采储量 / 年度产量",
        "以当前产量估计的可采年限",
        "可采储量 1000、年度产量 100，储量寿命为 10 年。",
    ),
    (
        "book_energy_lifting_cost",
        "能源 Lifting Cost",
        "油气开采运营成本 / 产量",
        "单位油气开采成本",
        "运营成本 200、产量 100，Lifting Cost 为 2 元/单位。",
    ),
    (
        "book_gold_aisc",
        "黄金矿业 AISC",
        "维持性总成本 / 黄金产量",
        "包含维持性资本开支的单位综合成本",
        "维持性总成本 1200、黄金产量 1 盎司，AISC 为 1200 元/盎司。",
    ),
    (
        "book_airline_load_factor",
        "航空客座率",
        "实际旅客公里 / 可用座位公里",
        "已提供运力的实际使用程度",
        "实际旅客公里 85、可用座位公里 100，客座率为 85%。",
    ),
    (
        "book_airline_rasm",
        "航空 RASM",
        "客运收入 / 可用座位公里",
        "单位可用座位公里收入",
        "客运收入 8、可用座位公里 100，RASM 为 0.08 元。",
    ),
    (
        "book_airline_casm",
        "航空 CASM",
        "运营成本 / 可用座位公里",
        "单位可用座位公里成本",
        "运营成本 7、可用座位公里 100，CASM 为 0.07 元。",
    ),
    (
        "book_telecom_arpu",
        "电信 ARPU",
        "服务收入 / 平均用户数",
        "每名用户平均服务收入",
        "服务收入 500、平均用户 100，ARPU 为 5 元。",
    ),
    (
        "book_telecom_churn",
        "电信流失率",
        "流失用户数 / 期初用户数",
        "电信用户离网比例",
        "流失用户 2、期初用户 100，流失率为 2%。",
    ),
    (
        "book_biopharma_cash_runway",
        "生物医药 Cash Runway",
        "现有现金 / 月度现金消耗",
        "按当前消耗速度可维持的月数",
        "现金 120、月度消耗 10，现金跑道为 12 个月。",
    ),
    (
        "book_shipping_tce",
        "航运 TCE",
        "(航次收入 - 航次费用) / 可用营运天数",
        "每日等效租金收入",
        "航次收入 120、费用 20、可用天数 10，TCE 为 10 元/天。",
    ),
];

const EXTRA: &[(&str, &str, &str)] = &[
    ("book_trade_volume", "成交量与成交额", "成交额=成交量×价格"),
    (
        "book_share_counts",
        "股本结构",
        "自由流通比例=自由流通股/总股本",
    ),
    ("book_float_market_cap", "流通市值", "流通股×价格"),
    ("book_52w_range", "52周区间", "距高点=价格/52周高点-1"),
    (
        "book_order_imbalance",
        "委比与委差",
        "委比=(买量-卖量)/(买量+卖量)",
    ),
    ("book_order_flow", "订单流分类", "净流入=主动买入-主动卖出"),
    (
        "book_period",
        "K线周期",
        "由已验证来源约定：Binance 1小时=3600秒；A股/美股日线=86400秒",
    ),
    ("book_adjustment", "复权价格", "复权价=原价×复权因子"),
    ("book_log_return", "对数坐标", "log return=ln(P1/P0)"),
    (
        "book_nonstandard_bar",
        "非标准图表与成交价格",
        "HA收盘=(O+H+L+C)/4；图形价格不等于真实收盘",
    ),
    ("book_dcf", "DCF/目标价", "公允价值=未来现金流折现和"),
    ("book_revenue", "营业收入", "报告期收入原值"),
    ("book_ebitda_margin", "EBITDA利润率", "EBITDA/营业收入"),
    ("book_net_margin", "净利率", "净利润/营业收入"),
    ("book_roce", "ROCE", "EBIT/(总资产-流动负债)"),
    ("book_yoy", "同比", "本期/去年同期-1"),
    ("book_qoq", "环比", "本期/上期-1"),
    ("book_debt_ratio", "资产负债率", "总负债/总资产"),
    ("book_de_ratio", "负债权益比", "总负债/权益"),
    ("book_net_debt", "净债务", "有息债务-现金"),
    ("book_net_debt_ebitda", "净债务/EBITDA", "净债务/EBITDA"),
    ("book_cash_ratio", "现金比率", "现金/流动负债"),
    ("book_cfo", "经营现金流", "报告期CFO原值"),
    ("book_capex", "资本开支", "报告期Capex原值"),
    ("book_fcf", "自由现金流", "CFO-Capex"),
    ("book_cfo_income", "CFO/净利润", "CFO/净利润"),
    ("book_asset_turnover", "资产周转率", "收入/平均资产"),
    ("book_dpo", "应付账款天数", "平均应付/销售成本×365"),
    ("book_ccc", "现金转换周期", "DIO+DSO-DPO"),
    (
        "book_intangibles_ratio",
        "商誉无形占比",
        "(商誉+无形资产)/权益",
    ),
    (
        "book_diluted_shares",
        "稀释股数与每股收益",
        "稀释EPS=净利润/稀释股数",
    ),
    ("book_free_float", "自由流通比例", "自由流通股/流通股"),
];

fn extra_inputs(id: &str) -> Option<Vec<(&'static str, &'static str, f64)>> {
    Some(match id {
        "book_trade_volume" => vec![
            ("shares", "成交数量（股）", 1000.),
            ("price", "成交价格（元/股）", 10.),
        ],
        "book_share_counts" => vec![
            ("free_float_shares", "自由流通股（股）", 6e8),
            ("total_shares", "总股本（股）", 1e9),
        ],
        "book_float_market_cap" => vec![
            ("price", "股价（元/股）", 20.),
            ("total_shares", "总股本（股）", 1e9),
            ("float_shares", "流通股（股）", 6e8),
        ],
        "book_52w_range" => vec![
            ("price", "当前价格（元）", 80.),
            ("high_52w", "52周最高价（元）", 100.),
            ("low_52w", "52周最低价（元）", 50.),
        ],
        "book_order_imbalance" => vec![
            ("bid_size", "委买数量（手）", 8000.),
            ("ask_size", "委卖数量（手）", 5000.),
        ],
        "book_order_flow" => vec![
            ("aggressive_buy_value", "主动买入额（元）", 60000.),
            ("aggressive_sell_value", "主动卖出额（元）", 40000.),
        ],
        "book_period" => vec![],
        "book_adjustment" => vec![
            ("raw_price", "原始价格（元）", 10.),
            ("adjustment_factor", "复权因子", 1.2),
        ],
        "book_log_return" => vec![],
        "book_nonstandard_bar" => vec![],
        "book_dcf" => vec![
            ("year1_fcf", "第1年自由现金流（元）", 100.),
            ("year2_fcf", "第2年自由现金流（元）", 110.),
            ("terminal_value", "第2年末终值（元）", 2000.),
            ("discount_rate", "折现率（小数）", 0.1),
        ],
        "book_revenue" => vec![("revenue", "营业收入（元）", 1000.)],
        "book_ebitda_margin" => vec![
            ("ebitda", "EBITDA（元）", 150.),
            ("revenue", "营业收入（元）", 1000.),
        ],
        "book_net_margin" => vec![
            ("net_income", "净利润（元）", 100.),
            ("revenue", "营业收入（元）", 1000.),
        ],
        "book_roce" => vec![
            ("ebit", "EBIT（元）", 120.),
            ("total_assets", "总资产（元）", 1000.),
            ("current_liabilities", "流动负债（元）", 400.),
        ],
        "book_yoy" => vec![
            ("current", "本期数值", 120.),
            ("prior_year_same_period", "去年同期数值", 100.),
        ],
        "book_qoq" => vec![
            ("current", "本期数值", 110.),
            ("previous_period", "上期数值", 100.),
        ],
        "book_debt_ratio" => vec![
            ("total_liabilities", "总负债（元）", 60.),
            ("total_assets", "总资产（元）", 100.),
        ],
        "book_de_ratio" => vec![
            ("debt", "债务金额（元）", 60.),
            ("equity", "股东权益（元）", 40.),
        ],
        "book_net_debt" => vec![
            ("interest_bearing_debt", "有息债务（元）", 30.),
            ("cash", "现金及等价物（元）", 20.),
        ],
        "book_net_debt_ebitda" => vec![
            ("interest_bearing_debt", "有息债务（元）", 30.),
            ("cash", "现金及等价物（元）", 20.),
            ("ebitda", "EBITDA（元）", 10.),
        ],
        "book_cash_ratio" => vec![
            (
                "cash_and_liquid_investments",
                "现金及高流动性投资（元）",
                30.,
            ),
            ("current_liabilities", "流动负债（元）", 100.),
        ],
        "book_cfo" => vec![("operating_cash_flow", "经营现金流（元）", 150.)],
        "book_capex" => vec![("capital_expenditure", "资本开支（元）", 50.)],
        "book_fcf" => vec![
            ("operating_cash_flow", "经营现金流（元）", 150.),
            ("capital_expenditure", "资本开支（元）", 50.),
        ],
        "book_cfo_income" => vec![
            ("operating_cash_flow", "经营现金流（元）", 120.),
            ("net_income", "净利润（元）", 100.),
        ],
        "book_asset_turnover" => vec![
            ("revenue", "营业收入（元）", 120.),
            ("average_assets", "平均总资产（元）", 100.),
        ],
        "book_dpo" => vec![
            ("average_payables", "平均应付账款（元）", 20.),
            ("cost_of_sales", "营业成本（元）", 120.),
        ],
        "book_ccc" => vec![
            ("dio_days", "存货周转天数（天）", 60.),
            ("dso_days", "应收账款天数（天）", 30.),
            ("dpo_days", "应付账款天数（天）", 45.),
        ],
        "book_intangibles_ratio" => vec![
            ("goodwill", "商誉（元）", 70.),
            ("intangibles", "无形资产（元）", 10.),
            ("equity", "股东权益（元）", 100.),
        ],
        "book_diluted_shares" => vec![
            ("net_income", "归母净利润（元）", 100.),
            ("diluted_weighted_shares", "稀释加权平均股数（股）", 20.),
        ],
        "book_free_float" => vec![
            ("free_float_shares", "自由流通股（股）", 40.),
            ("float_shares", "流通股（股）", 60.),
        ],
        _ => return None,
    })
}

fn industry_inputs(id: &str) -> Option<Vec<(&'static str, &'static str, f64)>> {
    macro_rules! f {($($k:literal,$l:literal,$v:expr);+)=>{vec![$(($k,$l,$v)),+]};}
    Some(match id {
        "book_roa" => {
            f!("net_income", "净利润（元）", 10.; "average_assets", "平均总资产（元）", 100.)
        }
        "book_bank_nim" => {
            f!("net_interest_income","净利息收入（元）",2.;"average_earning_assets","平均生息资产（元）",100.)
        }
        "book_bank_npl_ratio" => {
            f!("nonperforming_loans","不良贷款（元）",1.;"gross_loans","贷款总额（元）",100.)
        }
        "book_bank_provision_coverage" => {
            f!("loan_loss_reserves","贷款损失准备（元）",3.;"nonperforming_loans","不良贷款（元）",1.)
        }
        "book_bank_cet1_ratio" => {
            f!("cet1_capital","核心一级资本（元）",12.;"risk_weighted_assets","风险加权资产（元）",100.)
        }
        "book_bank_cost_income" => {
            f!("operating_expenses","经营费用（元）",40.;"operating_income","营业收入（元）",100.)
        }
        "book_insurance_combined_ratio" => {
            f!("claims_incurred","赔付额（元）",60.;"underwriting_expenses","承保费用（元）",30.;"net_earned_premiums","已赚保费（元）",100.)
        }
        "book_insurance_solvency_ratio" => {
            f!("available_capital","可用资本（元）",180.;"required_capital","监管资本要求（元）",100.)
        }
        "book_insurance_nbv" => {
            f!("year1_profit","第1年新业务利润（元）",5.5;"year2_profit","第2年新业务利润（元）",12.1;"discount_rate","折现率（小数）",0.1)
        }
        "book_reit_ffo" => {
            f!("net_income","净利润（元）",50.;"real_estate_depreciation","房地产折旧（元）",20.;"property_sale_gains","出售物业收益（元）",5.)
        }
        "book_reit_affo" => {
            f!("ffo","FFO（元）",65.;"maintenance_capex","维持性资本开支（元）",10.;"straight_line_rent_adjustment","直线租金调整（元）",5.)
        }
        "book_reit_occupancy" => {
            f!("leased_area","已出租面积（平方米）",90.;"lettable_area","可出租面积（平方米）",100.)
        }
        "book_reit_cap_rate" => {
            f!("net_operating_income","物业净经营收入（元）",6.;"property_value","物业价值（元）",100.)
        }
        "book_saas_arr" => f!(
            "monthly_recurring_revenue",
            "月度经常性收入（元）",
            10000000.
        ),
        "book_saas_nrr" => {
            f!("ending_cohort_revenue","期末存量客户经常性收入（元）",110.;"beginning_cohort_revenue","期初存量客户经常性收入（元）",100.)
        }
        "book_saas_churn" => {
            f!("churned_customers","流失客户数（户）",5.;"beginning_customers","期初客户数（户）",100.)
        }
        "book_saas_rule_of_40" => {
            f!("revenue_growth_rate","收入增长率（小数）",0.3;"fcf_margin","自由现金流利润率（小数）",0.12)
        }
        "book_saas_cac_payback" => {
            f!("cac","获客成本（元/客户）",1200.;"monthly_arpu","月度 ARPU（元/客户）",100.;"gross_margin","毛利率（小数）",0.8)
        }
        "book_platform_gmv" => f!(
            "gross_merchandise_value",
            "平台成交总额（元）",
            10000000000.
        ),
        "book_platform_take_rate" => {
            f!("platform_revenue","平台收入（元）",500000000.;"gross_merchandise_value","平台成交总额（元）",10000000000.)
        }
        "book_internet_dau_mau" => {
            f!("daily_active_users","日活用户（人）",300000.;"monthly_active_users","月活用户（人）",1000000.)
        }
        "book_internet_arpu" => {
            f!("revenue","收入（元）",1000000.;"active_users","活跃用户数（人）",100000.)
        }
        "book_retail_same_store_sales_growth" => {
            f!("current_same_store_sales","本期同店销售（元）",110.;"prior_same_store_sales","上期同店销售（元）",100.)
        }
        "book_semiconductor_utilization" => {
            f!("actual_output","实际产量（片）",80.;"maximum_capacity","最大产能（片）",100.)
        }
        "book_semiconductor_asp" => {
            f!("chip_revenue","芯片销售收入（元）",1000000.;"units_sold","销售数量（颗）",100000.)
        }
        "book_industrial_book_to_bill" => {
            f!("new_orders","新订单（元）",120.;"recognized_revenue","已确认收入（元）",100.)
        }
        "book_industrial_backlog" => f!("undelivered_orders", "尚未交付订单（元）", 3000000.),
        "book_energy_reserve_life" => {
            f!("recoverable_reserves","可采储量（单位）",1000.;"annual_production","年度产量（单位/年）",100.)
        }
        "book_energy_lifting_cost" => {
            f!("lifting_operating_cost","油气开采运营成本（元）",200.;"production_volume","产量（单位）",100.)
        }
        "book_gold_aisc" => {
            f!("sustaining_total_cost","维持性总成本（元）",1200.;"gold_ounces_produced","黄金产量（盎司）",1.)
        }
        "book_airline_load_factor" => {
            f!("revenue_passenger_kilometers","实际旅客公里（人公里）",85.;"available_seat_kilometers","可用座位公里（座位公里）",100.)
        }
        "book_airline_rasm" => {
            f!("passenger_revenue","客运收入（元）",8.;"available_seat_kilometers","可用座位公里（座位公里）",100.)
        }
        "book_airline_casm" => {
            f!("operating_cost","运营成本（元）",7.;"available_seat_kilometers","可用座位公里（座位公里）",100.)
        }
        "book_telecom_arpu" => {
            f!("service_revenue","服务收入（元）",500.;"average_subscribers","平均用户数（户）",100.)
        }
        "book_telecom_churn" => {
            f!("churned_subscribers","流失用户数（户）",2.;"beginning_subscribers","期初用户数（户）",100.)
        }
        "book_biopharma_cash_runway" => {
            f!("cash_balance","现有现金（元）",120.;"monthly_cash_burn","月度现金消耗（元）",10.)
        }
        "book_shipping_tce" => {
            f!("voyage_revenue","航次收入（元）",120.;"voyage_expenses","航次费用（元）",20.;"available_operating_days","可用营运天数（天）",10.)
        }
        _ => return None,
    })
}

pub fn catalog() -> Vec<crate::practice::PracticeConcept> {
    let fields: Vec<(&str, Vec<(&str, f64)>)> = vec![
        (
            "book_current_ratio",
            vec![("current_assets", 150.0), ("current_liabilities", 100.0)],
        ),
        (
            "book_quick_ratio",
            vec![
                ("cash", 20.0),
                ("receivables", 30.0),
                ("short_term_investments", 10.0),
                ("current_liabilities", 40.0),
            ],
        ),
        (
            "book_interest_coverage",
            vec![("ebit", 120.0), ("interest_expense", 20.0)],
        ),
        (
            "book_inventory_turnover",
            vec![("cost_of_sales", 600.0), ("average_inventory", 100.0)],
        ),
        (
            "book_receivable_turnover",
            vec![("revenue", 360.0), ("average_receivables", 60.0)],
        ),
        (
            "book_ptbv",
            vec![
                ("market_cap", 1000.0),
                ("equity", 800.0),
                ("goodwill", 100.0),
                ("intangibles", 200.0),
            ],
        ),
        (
            "book_price_cashflow",
            vec![
                ("market_cap", 1000.0),
                ("operating_cf", 200.0),
                ("capex", 50.0),
            ],
        ),
        (
            "book_ev",
            vec![
                ("market_cap", 1000.0),
                ("debt", 300.0),
                ("preferred_equity", 20.0),
                ("minority_interest", 10.0),
                ("cash", 50.0),
            ],
        ),
        ("book_ev_ebit", vec![("ev", 1280.0), ("ebit", 160.0)]),
        ("book_ev_sales", vec![("ev", 1280.0), ("revenue", 640.0)]),
        (
            "book_dividend_yield",
            vec![("dividend_per_share", 2.0), ("price", 50.0)],
        ),
        (
            "book_payout_ratio",
            vec![("dividend_per_share", 2.0), ("eps", 5.0)],
        ),
        ("book_earnings_yield", vec![("eps", 5.0), ("price", 50.0)]),
        (
            "book_fcf_yield",
            vec![
                ("operating_cf", 200.0),
                ("capex", 50.0),
                ("market_cap", 1000.0),
            ],
        ),
        (
            "book_nav_discount",
            vec![("price", 90.0), ("nav_per_share", 100.0)],
        ),
        (
            "book_gross_margin",
            vec![("revenue", 500.0), ("cost_of_sales", 300.0)],
        ),
        ("book_ebit_margin", vec![("ebit", 80.0), ("revenue", 500.0)]),
        (
            "book_roa",
            vec![("net_income", 10.0), ("average_assets", 100.0)],
        ),
    ];
    let mut out: Vec<crate::practice::PracticeConcept> = fields
        .into_iter()
        .map(|(id, fs)| crate::practice::PracticeConcept {
            id: id.into(),
            name: entries().into_iter().find(|e| e.id == id).unwrap().name,
            category: "原书财务计算".into(),
            input_kind: "independent_inputs".into(),
            inputs: fs
                .into_iter()
                .map(|(key, value)| crate::practice::PracticeInput {
                    key: key.into(),
                    label: key.into(),
                    default: json!(value),
                })
                .collect(),
            notes: "同一报告期的显式教学输入；分母为零或经济口径不适用时返回错误。".into(),
        })
        .collect();
    for (id, name, formula) in EXTRA {
        let market_bars = matches!(
            *id,
            "book_log_return" | "book_nonstandard_bar" | "book_period"
        );
        let fs = extra_inputs(id)
            .expect("registered extra inputs")
            .into_iter()
            .map(|(key, label, value)| crate::practice::PracticeInput {
                key: key.into(),
                label: label.into(),
                default: json!(value),
            })
            .collect();
        out.push(crate::practice::PracticeConcept {
            id: (*id).into(),
            name: (*name).into(),
            category: "原书教学输入".into(),
            input_kind: if market_bars {
                "market_bars".into()
            } else {
                "independent_inputs".into()
            },
            inputs: fs,
            notes: if *id == "book_nonstandard_bar" {
                format!("{}；使用最近一根已收盘真实K线的OHLC计算合成展示价；实际收盘价单独给出，合成价不可作为成交价。", formula)
            } else if market_bars {
                format!(
                    "{}；使用最近两根有序、已收盘OHLCV K线的收盘价计算。",
                    formula
                )
            } else {
                format!(
                    "{}；全部字段是可编辑教学输入，财报和盘口数据须注明来源与时点。",
                    formula
                )
            },
        });
    }
    for (id, name, formula, _, _) in INDUSTRY {
        let fs = industry_inputs(id)
            .expect("registered industry inputs")
            .into_iter()
            .map(|(key, label, value)| crate::practice::PracticeInput {
                key: key.into(),
                label: label.into(),
                default: json!(value),
            })
            .collect();
        out.push(crate::practice::PracticeConcept {
            id: (*id).into(),
            name: (*name).into(),
            category: "原书行业专属指标".into(),
            input_kind: "independent_inputs".into(),
            inputs: fs,
            notes: format!(
                "{}；全部字段是可编辑教学输入，只有同业同口径数据才可比较。",
                formula
            ),
        });
    }
    out
}
fn log_return_bars(bars: &[Bar]) -> Result<&[Bar], String> {
    crate::practice::validate_bars(bars)?;
    if bars.len() < 2 {
        return Err("对数收益率至少需要两根有序、已收盘的OHLCV K线".into());
    }
    if bars.last().unwrap().timestamp > Utc::now() {
        return Err("最后一根K线时间在未来，不能视为已收盘".into());
    }
    Ok(bars)
}
/// Computes the OHLC4 synthetic display price used by non-standard charts.
///
/// The result is `(open + high + low + close) / 4`. It is not a tradable
/// quote or a replacement for the bar's actual close. Callers that need a
/// market observation must first choose a completed source bar.
pub fn nonstandard_bar_ohlc4(open: f64, high: f64, low: f64, close: f64) -> Result<f64, String> {
    if ![open, high, low, close]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("OHLC 必须都是有限数值".into());
    }
    if open <= 0.0 || low <= 0.0 || close <= 0.0 {
        return Err("OHLC 的开盘、最低和收盘价必须为正数".into());
    }
    if high < open.max(close) || low > open.min(close) {
        return Err("OHLC 不满足 low ≤ open/close ≤ high".into());
    }
    let value = (open + high + low + close) / 4.0;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| "OHLC4 合成值超出可表示范围".into())
}

/// Summarizes the source-declared K-line period and observed timestamp gaps.
///
/// A stock's daily bar has a nominal 86,400-second calendar period even when a
/// weekend, holiday, or suspension creates a larger gap between observations.
/// Binance is expected to provide continuous one-hour bars; missing hours are
/// reported as data gaps and never relabel the bars as multi-hour candles.
pub fn market_period_summary(bars: &[Bar], source: &str) -> Result<Value, String> {
    let (source_label, nominal_seconds, stock_daily) = match source {
        "binance" | "real" => ("Binance 1小时", 3_600_i64, false),
        "a_share" => ("A股日线", 86_400_i64, true),
        "us_stock" => ("美股日线", 86_400_i64, true),
        _ => return Err("K线周期只接受 Binance、A股或美股的真实来源".into()),
    };
    crate::practice::validate_bars(bars)?;
    let first = bars.first().ok_or("K线周期至少需要一根已收盘OHLCV K线")?;
    let last = bars.last().expect("checked non-empty bars");
    if last.timestamp > Utc::now() {
        return Err("最后一根K线时间在未来，不能视为已收盘".into());
    }
    if !stock_daily
        && bars
            .iter()
            .any(|bar| bar.timestamp.timestamp().rem_euclid(nominal_seconds) != 0)
    {
        return Err("Binance 1小时K线时间戳必须位于UTC整点".into());
    }
    if stock_daily
        && bars.iter().any(|bar| {
            bar.timestamp.hour() != 0 || bar.timestamp.minute() != 0 || bar.timestamp.second() != 0
        })
    {
        return Err("A股和美股日线必须使用UTC 00:00的交易日期标签".into());
    }
    let intervals: Vec<i64> = bars
        .windows(2)
        .map(|pair| (pair[1].timestamp - pair[0].timestamp).num_seconds())
        .collect();
    let last_interval = intervals.last().copied();
    let (missing_expected_intervals, calendar_gap_count) = if stock_daily {
        (
            0_i64,
            intervals
                .iter()
                .filter(|&&seconds| seconds > nominal_seconds)
                .count() as i64,
        )
    } else {
        (
            intervals
                .iter()
                .map(|&seconds| (seconds / nominal_seconds - 1).max(0))
                .sum(),
            0_i64,
        )
    };
    let range = format!(
        "{} 至 {}",
        first.timestamp.to_rfc3339(),
        last.timestamp.to_rfc3339()
    );
    let mut notes = vec![format!(
        "来源：{source_label}；名义周期：{nominal_seconds} 秒{}。本次含 {} 根已收盘K线，范围：{range}。",
        if stock_daily { "（日线的日历长度，不是开市时长）" } else { "（1小时）" },
        bars.len()
    )];
    if let Some(seconds) = last_interval {
        notes.push(format!("最近相邻观测间隔：{seconds} 秒。"));
    } else {
        notes.push("只有一根K线，无法核对相邻观测间隔。".into());
    }
    if stock_daily && calendar_gap_count > 0 {
        notes.push(format!("发现 {calendar_gap_count} 个大于86400秒的日期空档（周末、节假日或停牌均可能造成）；它们不表示多日K线。"));
    }
    if !stock_daily && missing_expected_intervals > 0 {
        notes.push(format!("发现 {missing_expected_intervals} 个缺小时空档；名义周期仍为1小时，缺口应作为数据完整性问题处理。"));
    }
    Ok(json!({
        "concept_id":"book_period", "input_kind":"market_bars", "provenance":"provided_market_bars",
        "status":"computed", "reason":null,
        "values":{"book_period":nominal_seconds,"last_observed_interval_seconds":last_interval,"bar_count":bars.len(),"missing_expected_intervals":missing_expected_intervals,"calendar_gap_count":calendar_gap_count},
        "units":{"book_period":"秒","last_observed_interval_seconds":"秒","bar_count":"根 K 线","missing_expected_intervals":"个预期周期","calendar_gap_count":"个日期空档"},
        "series":[],"notes":notes,"inputs":{}
    }))
}

/// Evaluates two fixed horizons from one completed, source-bound series.
/// The source only supplies one native cadence, so this intentionally does not
/// pretend to compare a five-minute feed with a daily feed or infer independent
/// confirmation from the two observations.
pub fn market_timeframe_summary(bars: &[Bar], source: &str) -> Result<Value, String> {
    const SHORT_BARS: usize = 5;
    const LONG_BARS: usize = 20;
    let mut result = market_period_summary(bars, source)?;
    result["concept_id"] = json!("book_pitfall_timeframe");
    let values = result["values"]
        .as_object_mut()
        .expect("period values are an object");
    let nominal = values
        .remove("book_period")
        .expect("period summary has a nominal cadence");
    values.insert("source_bar_seconds".into(), nominal);
    values.insert("same_completed_asof".into(), json!(1.0));
    values.insert("short_horizon_bars".into(), json!(SHORT_BARS as f64));
    values.insert("long_horizon_bars".into(), json!(LONG_BARS as f64));
    let horizon_return = |periods: usize| {
        if bars.len() <= periods {
            None
        } else {
            let last = bars.last().expect("nonempty bars were checked");
            let first = &bars[bars.len() - periods - 1];
            (first.close > 0.0 && last.close > 0.0).then(|| last.close / first.close - 1.0)
        }
    };
    let short = horizon_return(SHORT_BARS);
    let long = horizon_return(LONG_BARS);
    let marker = |periods: usize| {
        if bars.len() > periods {
            bars.iter()
                .enumerate()
                .map(|(index, bar)| (index == bars.len() - periods - 1).then_some(bar.close))
                .collect::<Vec<_>>()
        } else {
            vec![None; bars.len()]
        }
    };
    values.insert("short_horizon_return".into(), json!(short));
    values.insert("long_horizon_return".into(), json!(long));
    values.insert("horizon_direction_differs".into(), json!(matches!((short, long), (Some(a), Some(b)) if a.signum() != b.signum() && a != 0.0 && b != 0.0) as u8 as f64));
    let units = result["units"]
        .as_object_mut()
        .expect("period units are an object");
    units.remove("book_period");
    for (key, unit) in [
        ("source_bar_seconds", "秒"),
        ("same_completed_asof", "布尔值（1/0）"),
        ("short_horizon_bars", "根 K 线"),
        ("long_horizon_bars", "根 K 线"),
        ("short_horizon_return", "比例（小数）"),
        ("long_horizon_return", "比例（小数）"),
        ("horizon_direction_differs", "布尔值（1/0）"),
        ("close_price", "price"),
        ("short_horizon_start", "price"),
        ("long_horizon_start", "price"),
    ] {
        units.insert(key.into(), json!(unit));
    }
    let notes = result["notes"]
        .as_array_mut()
        .expect("period notes are an array");
    notes.push(json!(format!(
        "短期取最近 {SHORT_BARS} 根K线、长期取最近 {LONG_BARS} 根K线的收盘价变化；两者使用同一标的、同一来源和最后一根已收盘K线作为截止。原书提示不同时间尺度可同时成立：方向不同是可观察结果，不是错误，也不能据此宣称独立确认。"
    )));
    result["series"] = json!([
        {"name":"close_price","values":bars.iter().map(|bar| Some(bar.close)).collect::<Vec<_>>()},
        {"name":"short_horizon_start","values":marker(SHORT_BARS)},
        {"name":"long_horizon_start","values":marker(LONG_BARS)}
    ]);
    Ok(result)
}

/// Shows a documented MACD histogram scaling convention from one completed,
/// source-bound market series. The two lines are conventions, not provider feeds.
pub fn market_formula_variant_summary(bars: &[Bar], source: &str) -> Result<Value, String> {
    const FAST: usize = 12;
    const SLOW: usize = 26;
    const SIGNAL: usize = 9;
    let mut result = market_period_summary(bars, source)?;
    result["concept_id"] = json!("book_pitfall_formula_variant");
    result["source_ids"] = json!(["book_28_16"]);
    let macd = crate::indicators::trend::macd(
        &bars.iter().map(|bar| bar.close).collect::<Vec<_>>(),
        FAST,
        SLOW,
        SIGNAL,
    );
    let twice = macd
        .hist
        .iter()
        .map(|value| value.map(|value| value * 2.0))
        .collect::<Vec<_>>();
    let values = result["values"]
        .as_object_mut()
        .expect("period values are an object");
    let nominal = values
        .remove("book_period")
        .expect("period summary has a nominal cadence");
    values.insert("source_bar_seconds".into(), nominal);
    let latest_histogram = macd.hist.iter().rev().flatten().next().copied();
    for (key, value) in [
        ("fast_period", FAST as f64),
        ("slow_period", SLOW as f64),
        ("signal_period", SIGNAL as f64),
        ("first_histogram_scale", 1.0),
        ("second_histogram_scale", 2.0),
    ] {
        values.insert(key.into(), json!(value));
    }
    values.insert("latest_histogram_x1".into(), json!(latest_histogram));
    values.insert(
        "latest_histogram_x2".into(),
        json!(latest_histogram.map(|value| value * 2.0)),
    );
    values.insert(
        "latest_histogram_difference".into(),
        json!(latest_histogram),
    );
    let units = result["units"]
        .as_object_mut()
        .expect("period units are an object");
    units.remove("book_period");
    for (key, unit) in [
        ("source_bar_seconds", "秒"),
        ("fast_period", "根 K 线"),
        ("slow_period", "根 K 线"),
        ("signal_period", "根 K 线"),
        ("first_histogram_scale", "倍"),
        ("second_histogram_scale", "倍"),
        ("latest_histogram_x1", "macd_price"),
        ("latest_histogram_x2", "macd_price"),
        ("latest_histogram_difference", "macd_price"),
        ("macd_histogram_x1", "macd_price"),
        ("macd_histogram_x2", "macd_price"),
    ] {
        units.insert(key.into(), json!(unit));
    }
    result["series"] = json!([
        {"name":"macd_histogram_x1","values":macd.hist},
        {"name":"macd_histogram_x2","values":twice}
    ]);
    result["notes"].as_array_mut().expect("period notes are an array").push(json!(
        "MACD 使用本项目公开实现：DIF=EMA(12)-EMA(26)，DEA=signal 的 EMA(9)，柱体= DIF-DEA。图中 x1 与 x2 仅是同一柱体的两种倍数展示约定；它们不是两家供应商的实测输出，也不能用来判断某家供应商对错。"
    ));
    Ok(result)
}

pub fn evaluate(id: &str, bars: &[Bar], inputs: &Value) -> Result<Value, String> {
    let supplied = inputs.as_object().ok_or("inputs 必须是对象")?;
    let definition = catalog()
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("未知原书补充概念: {id}"))?;
    let allowed: std::collections::BTreeSet<String> =
        definition.inputs.iter().map(|x| x.key.clone()).collect();
    if let Some(key) = supplied.keys().find(|key| !allowed.contains(*key)) {
        return Err(format!("不支持输入{key}"));
    }
    let mut merged = serde_json::Map::new();
    for input in definition.inputs {
        merged.insert(input.key, input.default);
    }
    for (key, value) in supplied {
        merged.insert(key.clone(), value.clone());
    }
    let merged = Value::Object(merged);
    let n = |k: &str| {
        merged
            .get(k)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{k} 必须是有限数值"))
    };
    let ratio = |a: f64, b: f64| -> Result<f64, String> {
        if b.abs() > 1e-12 {
            Ok(a / b)
        } else {
            Err("分母为零，指标无定义".into())
        }
    };
    let value = match id {
        "book_current_ratio" => ratio(n("current_assets")?, n("current_liabilities")?)?,
        "book_quick_ratio" => ratio(
            n("cash")? + n("receivables")? + n("short_term_investments")?,
            n("current_liabilities")?,
        )?,
        "book_interest_coverage" => ratio(n("ebit")?, n("interest_expense")?)?,
        "book_inventory_turnover" => ratio(n("cost_of_sales")?, n("average_inventory")?)?,
        "book_receivable_turnover" => ratio(n("revenue")?, n("average_receivables")?)?,
        "book_ptbv" => ratio(
            n("market_cap")?,
            n("equity")? - n("goodwill")? - n("intangibles")?,
        )?,
        "book_price_cashflow" => {
            let ocf = n("operating_cf")?;
            let fcf = ocf - n("capex")?;
            if ocf <= 0.0 || fcf <= 0.0 {
                return Err("OCF 与 FCF 必须大于零才可计算市现率".into());
            };
            return Ok(
                json!({"concept_id":id,"input_kind":"independent_inputs","provenance":"explicit_inputs","status":"computed","reason":null,"values":{"p_ocf":n("market_cap")?/ocf,"p_fcf":n("market_cap")?/fcf},"units":{"p_ocf":"multiple","p_fcf":"multiple"},"series":[],"notes":[],"inputs":merged}),
            );
        }
        "book_ev" => {
            n("market_cap")? + n("debt")? + n("preferred_equity")? + n("minority_interest")?
                - n("cash")?
        }
        "book_ev_ebit" => {
            let e = n("ebit")?;
            if e <= 0.0 {
                return Err("EBIT 必须大于零".into());
            };
            n("ev")? / e
        }
        "book_ev_sales" => ratio(n("ev")?, n("revenue")?)?,
        "book_dividend_yield" => ratio(n("dividend_per_share")?, n("price")?)?,
        "book_payout_ratio" => {
            let e = n("eps")?;
            if e <= 0.0 {
                return Err("EPS 必须大于零".into());
            };
            n("dividend_per_share")? / e
        }
        "book_earnings_yield" => ratio(n("eps")?, n("price")?)?,
        "book_fcf_yield" => ratio(n("operating_cf")? - n("capex")?, n("market_cap")?)?,
        "book_nav_discount" => {
            let nav = n("nav_per_share")?;
            if nav <= 0.0 {
                return Err("nav_per_share 必须大于零".into());
            };
            n("price")? / nav - 1.0
        }
        "book_gross_margin" => {
            let r = n("revenue")?;
            let gross = r - n("cost_of_sales")?;
            return Ok(
                json!({"concept_id":id,"input_kind":"independent_inputs","provenance":"explicit_inputs","status":"computed","reason":null,"values":{"gross_profit":gross,"gross_margin":ratio(gross,r)?},"units":{"gross_profit":"currency","gross_margin":"fraction"},"series":[],"notes":[],"inputs":merged}),
            );
        }
        "book_ebit_margin" => ratio(n("ebit")?, n("revenue")?)?,
        "book_roa" => ratio(n("net_income")?, n("average_assets")?)?,
        "book_bank_nim" => ratio(n("net_interest_income")?, n("average_earning_assets")?)?,
        "book_bank_npl_ratio" => ratio(n("nonperforming_loans")?, n("gross_loans")?)?,
        "book_bank_provision_coverage" => {
            ratio(n("loan_loss_reserves")?, n("nonperforming_loans")?)?
        }
        "book_bank_cet1_ratio" => ratio(n("cet1_capital")?, n("risk_weighted_assets")?)?,
        "book_bank_cost_income" => ratio(n("operating_expenses")?, n("operating_income")?)?,
        "book_insurance_combined_ratio" => ratio(
            n("claims_incurred")? + n("underwriting_expenses")?,
            n("net_earned_premiums")?,
        )?,
        "book_insurance_solvency_ratio" => ratio(n("available_capital")?, n("required_capital")?)?,
        "book_insurance_nbv" => {
            let rate = n("discount_rate")?;
            if rate <= -1.0 {
                return Err("折现率必须大于 -100%".into());
            }
            n("year1_profit")? / (1.0 + rate) + n("year2_profit")? / (1.0 + rate).powi(2)
        }
        "book_reit_ffo" => {
            n("net_income")? + n("real_estate_depreciation")? - n("property_sale_gains")?
        }
        "book_reit_affo" => {
            n("ffo")? - n("maintenance_capex")? - n("straight_line_rent_adjustment")?
        }
        "book_reit_occupancy" => ratio(n("leased_area")?, n("lettable_area")?)?,
        "book_reit_cap_rate" => ratio(n("net_operating_income")?, n("property_value")?)?,
        "book_saas_arr" => n("monthly_recurring_revenue")? * 12.0,
        "book_saas_nrr" => ratio(n("ending_cohort_revenue")?, n("beginning_cohort_revenue")?)?,
        "book_saas_churn" => ratio(n("churned_customers")?, n("beginning_customers")?)?,
        "book_saas_rule_of_40" => n("revenue_growth_rate")? + n("fcf_margin")?,
        "book_saas_cac_payback" => ratio(n("cac")?, n("monthly_arpu")? * n("gross_margin")?)?,
        "book_platform_gmv" => n("gross_merchandise_value")?,
        "book_platform_take_rate" => ratio(n("platform_revenue")?, n("gross_merchandise_value")?)?,
        "book_internet_dau_mau" => ratio(n("daily_active_users")?, n("monthly_active_users")?)?,
        "book_internet_arpu" => ratio(n("revenue")?, n("active_users")?)?,
        "book_retail_same_store_sales_growth" => {
            ratio(n("current_same_store_sales")?, n("prior_same_store_sales")?)? - 1.0
        }
        "book_semiconductor_utilization" => ratio(n("actual_output")?, n("maximum_capacity")?)?,
        "book_semiconductor_asp" => ratio(n("chip_revenue")?, n("units_sold")?)?,
        "book_industrial_book_to_bill" => ratio(n("new_orders")?, n("recognized_revenue")?)?,
        "book_industrial_backlog" => n("undelivered_orders")?,
        "book_energy_reserve_life" => ratio(n("recoverable_reserves")?, n("annual_production")?)?,
        "book_energy_lifting_cost" => ratio(n("lifting_operating_cost")?, n("production_volume")?)?,
        "book_gold_aisc" => ratio(n("sustaining_total_cost")?, n("gold_ounces_produced")?)?,
        "book_airline_load_factor" => ratio(
            n("revenue_passenger_kilometers")?,
            n("available_seat_kilometers")?,
        )?,
        "book_airline_rasm" => ratio(n("passenger_revenue")?, n("available_seat_kilometers")?)?,
        "book_airline_casm" => ratio(n("operating_cost")?, n("available_seat_kilometers")?)?,
        "book_telecom_arpu" => ratio(n("service_revenue")?, n("average_subscribers")?)?,
        "book_telecom_churn" => ratio(n("churned_subscribers")?, n("beginning_subscribers")?)?,
        "book_biopharma_cash_runway" => ratio(n("cash_balance")?, n("monthly_cash_burn")?)?,
        "book_shipping_tce" => ratio(
            n("voyage_revenue")? - n("voyage_expenses")?,
            n("available_operating_days")?,
        )?,
        "book_trade_volume" => n("shares")? * n("price")?,
        "book_share_counts" => ratio(n("free_float_shares")?, n("total_shares")?)?,
        "book_float_market_cap" => n("price")? * n("float_shares")?,
        "book_52w_range" => ratio(n("price")?, n("high_52w")?)? - 1.0,
        "book_order_imbalance" => ratio(
            n("bid_size")? - n("ask_size")?,
            n("bid_size")? + n("ask_size")?,
        )?,
        "book_order_flow" => n("aggressive_buy_value")? - n("aggressive_sell_value")?,
        "book_period" => return Err("K线周期必须由 API 根据已验证真实来源和已收盘K线计算".into()),
        "book_adjustment" => n("raw_price")? * n("adjustment_factor")?,
        "book_log_return" => {
            let bars = log_return_bars(bars)?;
            (bars[bars.len() - 1].close / bars[bars.len() - 2].close).ln()
        }
        "book_nonstandard_bar" => {
            crate::practice::validate_bars(bars)?;
            let bar = bars.last().ok_or("需要至少一根已收盘的真实OHLCV K线")?;
            if bar.timestamp > Utc::now() {
                return Err("K线时间在未来，不能视为已收盘".into());
            }
            nonstandard_bar_ohlc4(bar.open, bar.high, bar.low, bar.close)?
        }
        "book_dcf" => {
            let r = n("discount_rate")?;
            if r <= -1.0 {
                return Err("折现率必须大于 -100%".into());
            };
            n("year1_fcf")? / (1.0 + r)
                + (n("year2_fcf")? + n("terminal_value")?) / (1.0 + r).powi(2)
        }
        "book_revenue" => n("revenue")?,
        "book_ebitda_margin" => ratio(n("ebitda")?, n("revenue")?)?,
        "book_net_margin" => ratio(n("net_income")?, n("revenue")?)?,
        "book_roce" => ratio(n("ebit")?, n("total_assets")? - n("current_liabilities")?)?,
        "book_yoy" => ratio(n("current")?, n("prior_year_same_period")?)? - 1.0,
        "book_qoq" => ratio(n("current")?, n("previous_period")?)? - 1.0,
        "book_debt_ratio" => ratio(n("total_liabilities")?, n("total_assets")?)?,
        "book_de_ratio" => ratio(n("debt")?, n("equity")?)?,
        "book_net_debt" => n("interest_bearing_debt")? - n("cash")?,
        "book_net_debt_ebitda" => ratio(n("interest_bearing_debt")? - n("cash")?, n("ebitda")?)?,
        "book_cash_ratio" => ratio(n("cash_and_liquid_investments")?, n("current_liabilities")?)?,
        "book_cfo" => n("operating_cash_flow")?,
        "book_capex" => n("capital_expenditure")?,
        "book_fcf" => n("operating_cash_flow")? - n("capital_expenditure")?,
        "book_cfo_income" => ratio(n("operating_cash_flow")?, n("net_income")?)?,
        "book_asset_turnover" => ratio(n("revenue")?, n("average_assets")?)?,
        "book_dpo" => (ratio(n("average_payables")?, n("cost_of_sales")?)?) * 365.0,
        "book_ccc" => n("dio_days")? + n("dso_days")? - n("dpo_days")?,
        "book_intangibles_ratio" => ratio(n("goodwill")? + n("intangibles")?, n("equity")?)?,
        "book_diluted_shares" => ratio(n("net_income")?, n("diluted_weighted_shares")?)?,
        "book_free_float" => ratio(n("free_float_shares")?, n("float_shares")?)?,
        _ => return Err(format!("未知原书补充概念: {id}")),
    };
    if !value.is_finite() {
        return Err("结果超出有限数值范围，请检查输入量级".into());
    }
    let market_bars = matches!(
        id,
        "book_log_return" | "book_nonstandard_bar" | "book_period"
    );
    let mut result = json!({"concept_id":id,"input_kind":if market_bars {"market_bars"} else {"independent_inputs"},"provenance":if market_bars {"provided_market_bars"} else {"explicit_inputs"},"status":"computed","reason":null,"values":{id:value},"units":{id:unit(id)},"series":[],"notes":[if id == "book_log_return" {"仅使用最近两根有序、已收盘OHLCV K线的收盘价；不使用手填价格或未来K线。"} else if id == "book_nonstandard_bar" {"OHLC4 是已收盘真实K线的合成展示价；同时列出实际收盘价，合成价不可作为成交价。"} else {"同一报告期口径；分母为零时拒绝计算。"}],"inputs":merged});
    if id == "book_nonstandard_bar" {
        let actual_close = bars.last().unwrap().close;
        result["values"]["actual_close"] = json!(actual_close);
        result["values"]["synthetic_minus_close"] = json!(value - actual_close);
        result["units"]["actual_close"] = json!(unit(id));
        result["units"]["synthetic_minus_close"] = json!(unit(id));
    }
    Ok(result)
}

fn unit(id: &str) -> &'static str {
    match id {
        "book_current_ratio"
        | "book_quick_ratio"
        | "book_interest_coverage"
        | "book_inventory_turnover"
        | "book_receivable_turnover"
        | "book_ptbv"
        | "book_ps"
        | "book_ev_ebitda"
        | "book_ev_ebit"
        | "book_ev_sales"
        | "book_asset_turnover"
        | "book_cfo_income"
        | "book_net_debt_ebitda"
        | "book_de_ratio"
        | "book_industrial_book_to_bill" => "倍",
        "book_ev"
        | "book_net_debt"
        | "book_cfo"
        | "book_capex"
        | "book_fcf"
        | "book_trade_volume"
        | "book_float_market_cap"
        | "book_order_flow"
        | "book_dcf"
        | "book_revenue"
        | "book_insurance_nbv"
        | "book_reit_ffo"
        | "book_reit_affo"
        | "book_saas_arr"
        | "book_platform_gmv"
        | "book_industrial_backlog" => "元",
        "book_period" => "秒",
        "book_nonstandard_bar" => "price",
        "book_adjustment" | "book_diluted_shares" => "元/股",
        "book_internet_arpu" | "book_telecom_arpu" => "元/用户",
        "book_semiconductor_asp" => "元/颗",
        "book_energy_lifting_cost" => "元/产量单位",
        "book_gold_aisc" => "元/盎司",
        "book_airline_rasm" | "book_airline_casm" => "元/座位公里",
        "book_shipping_tce" => "元/天",
        "book_dpo" | "book_ccc" => "天",
        "book_energy_reserve_life" => "年",
        "book_saas_cac_payback" | "book_biopharma_cash_runway" => "月",
        _ => "fraction",
    }
}
