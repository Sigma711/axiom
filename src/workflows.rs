//! The book's combined example, market selection and reproducibility exercises.
use crate::{
    knowledge::KnowledgeEntry,
    practice::{boolean, num, Output, PracticeConcept, PracticeInput},
    types::Bar,
};
use serde_json::{json, Value};

fn definitions() -> Vec<Value> {
    vec![
        json!({"id":"book_combined_analysis","name":"组合读图：质量、估值、趋势与仓位","page":67,
"formula":"PE=价格/EPS；PB=价格/BVPS；风险距离=ATR×倍数；股数=min(风险预算/风险距离,现金预算/价格)",
"summary":"把公司质量、价格状态和可承受损失分开判断，再决定可承担的仓位。",
"meaning":"原书完整例子：股价100、EPS5、每股净资产40；ROIC14%高于资本成本8%。价格高于MA20，MA20高于MA60且位于云上，说明趋势偏强；RSI74和RVOL2.2说明短期延伸与参与度，不能直接推出必须买卖。",
"example":"ATR为4，2倍ATR的风险距离是8元。最多承受800元损失，对应100股；还须受到现金、交易单位与实际成交价格限制。",
"pitfalls":"ROIC、财报、均线及云层必须来自同一决策时点。这里是预算示例，跳空、滑点和无法成交可能使实际损失超过预算。",
"defaults":{"price":100,"eps":5,"bvps":40,"roic":0.14,"capital_cost":0.08,"fcf_yield":0.04,"net_debt_ebitda":1,"ma20":96,"ma60":90,"adx":28,"rsi":74,"atr":4,"rvol":2.2,"cloud_low":88,"cloud_high":94,"atr_multiple":2,"loss_budget":800,"cash_budget":20000}}),
        json!({"id":"book_market_regimes","name":"按市场环境选择指标，避免重复证据","page":68,
"formula":"冗余数=max(趋势指标数−2,0)+max(动量指标数−1,0)；证据按问题分组，不按指标个数计票",
"summary":"先确定问题，再选指标；多个同源指标不是多份独立证据。",
"meaning":"长期投资看收入、FCF、ROIC、负债和估值；上升趋势看价格结构、MA、ADX、ATR和量；震荡看支撑阻力、RSI/Stochastic和布林带；突破看Donchian、带宽、RVOL和ATR。日内看价差、深度、VWAP、逐笔和CVD；轮动看相对强弱、RRG和宽度；期权看IV、期限、Skew、Greeks和OI；组合看Beta、相关性、Sharpe、回撤和VaR。",
"example":"选3个趋势指标、4个动量指标，会超出示例建议共4个。补上波动、量价与执行，仍只有5类问题，不是10份独立证明。",
"pitfalls":"数量只是教学提示，不是策略最优参数。ATR和布林带回答不同问题，可同时保留；下单前必须检查价差和深度。",
"defaults":{"trend_indicators":2,"momentum_indicators":1,"volatility_checked":true,"participation_checked":true,"execution_checked":true}}),
        json!({"id":"book_reading_order","name":"看盘十步：从公司到成交","page":70,
"formula":"完成度=已核对步骤/10；任何未核对项都显示为待补证据",
"summary":"依次回答公司、估值、趋势、风险和成交问题，避免先见金叉就下单。",
"meaning":"1理解业务与利润现金真实性；2看收入利润ROIC负债变化；3比较同业及历史估值；4看日周线结构；5用均线或云确认趋势；6看ADX强度；7选一个动量证据；8用ATR预算仓位；9看成交量RVOL/VWAP成本重心；10核对价差深度与流动性。",
"example":"前9步都完成，但未检查成交条件，完成度是90%，仍有1项待核对。勾选表示你已提供证据，并不表示市场结论正确。",
"pitfalls":"清单不自动验证人工提供的证据真实性，也不产生投资建议；完成度100%不等于零风险。",
"defaults":{"business_checked":true,"quality_checked":true,"valuation_checked":true,"timeframes_checked":true,"trend_checked":true,"strength_checked":true,"momentum_checked":true,"sizing_checked":true,"participation_checked":true,"execution_checked":true}}),
        json!({"id":"book_data_conventions","name":"数据口径与可复现性核对","page":83,
"formula":"可复现=所有口径已记录且数据在信号时点已公开",
"summary":"同名指标数值不同，先查价格源、平滑、时段、股本、财务周期、成交方向和历史修订。",
"meaning":"记录收盘/典型/复权价格与初始化算法；交易时区和盘前盘后；总股本/流通/稀释股数；年报/TTM/预期；Bid/Ask、Tick Rule或低周期近似；重述和供应商回填版本。再记录信号确认时点和数据发布时间。",
"example":"其余口径都相同，但拿事后发布的财报计算历史信号，published_before_signal为否，可复现核对不通过。",
"pitfalls":"技术指标不保证未来表现；未收盘、枢轴、分形和ZigZag可变或延迟确认。Greeks/VaR依赖模型；还需计入税费、停牌、涨跌停、退市和无法成交。来源优先交易所、监管及平台公式文档。",
"defaults":{"price_source_recorded":true,"smoothing_recorded":true,"sessions_recorded":true,"share_basis_recorded":true,"financial_period_recorded":true,"trade_classification_recorded":true,"revision_recorded":true,"confirmation_recorded":true,"published_before_signal":true}}),
    ]
}
pub fn entries() -> Vec<KnowledgeEntry> {
    definitions()
        .into_iter()
        .map(|d| {
            let s = |k: &str| d[k].as_str().unwrap().to_owned();
            KnowledgeEntry {
                id: s("id"),
                name: s("name"),
                summary: s("summary"),
                formula: s("formula"),
                meaning: s("meaning"),
                example: s("example"),
                pitfalls: s("pitfalls"),
                signals: "逐项改动教学输入，观察结果，并填写自己的证据与失效条件。".into(),
                category: "组合分析与数据口径".into(),
                related: vec![],
                code_ref: "src/workflows.rs::evaluate".into(),
                code_url: "https://github.com/Sigma711/axiom/blob/main/src/workflows.rs".into(),
                implementation: "src/workflows.rs::evaluate".into(),
                diagram: None,
            }
        })
        .collect()
}
pub fn catalog() -> Vec<PracticeConcept> {
    definitions()
        .into_iter()
        .map(|d| PracticeConcept {
            id: d["id"].as_str().unwrap().into(),
            name: d["name"].as_str().unwrap().into(),
            category: "组合分析与数据口径".into(),
            input_kind: "independent_inputs".into(),
            inputs: d["defaults"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| PracticeInput {
                    key: k.clone(),
                    label: k.clone(),
                    default: v.clone(),
                })
                .collect(),
            notes: format!(
                "原书PDF第{}页；默认值为教学输入，核对不代表预测或交易建议。",
                d["page"]
            ),
        })
        .collect()
}
pub fn evaluate(id: &str, _bars: &[Bar], inputs: &Value) -> Result<Value, String> {
    let d = definitions()
        .into_iter()
        .find(|d| d["id"] == id)
        .ok_or("未知组合概念")?;
    let mut v = d["defaults"].clone();
    for (k, x) in inputs.as_object().ok_or("inputs必须为对象")? {
        if v.get(k).is_none() {
            return Err(format!("未知输入{k}"));
        }
        v[k] = x.clone();
    }
    let mut o = Output::default();
    match id {
        "book_combined_analysis" => {
            let positive = |k: &str| -> Result<f64, String> {
                let n = num(&v, k)?;
                if n <= 0.0 {
                    Err(format!("{k}必须大于0"))
                } else {
                    Ok(n)
                }
            };
            let price = positive("price")?;
            let distance = positive("atr")? * positive("atr_multiple")?;
            let risk_shares = (positive("loss_budget")? / distance).floor();
            let cash_shares = (positive("cash_budget")? / price).floor();
            let low = positive("cloud_low")?;
            let high = positive("cloud_high")?;
            if low > high {
                return Err("云层下界不得高于上界".into());
            }
            let adx = num(&v, "adx")?;
            let rsi = num(&v, "rsi")?;
            if !(0.0..=100.0).contains(&adx) || !(0.0..=100.0).contains(&rsi) {
                return Err("ADX与RSI范围为0到100".into());
            }
            o.number("pe", price / positive("eps")?, "multiple");
            o.number("pb", price / positive("bvps")?, "multiple");
            o.number(
                "value_spread",
                num(&v, "roic")? - num(&v, "capital_cost")?,
                "fraction",
            );
            o.number("fcf_yield", num(&v, "fcf_yield")?, "fraction");
            o.number("net_debt_ebitda", num(&v, "net_debt_ebitda")?, "multiple");
            o.flag(
                "trend_up",
                price > positive("ma20")? && num(&v, "ma20")? > positive("ma60")? && price > high,
            );
            o.flag("adx_above_25", adx > 25.0);
            o.flag("rsi_above_70", rsi > 70.0);
            o.number("relative_volume", positive("rvol")?, "multiple");
            o.number("atr_fraction", num(&v, "atr")? / price, "fraction");
            o.number("risk_distance", distance, "currency/share");
            o.number("risk_budget_shares", risk_shares, "shares");
            o.number("affordable_shares", cash_shares, "shares");
            o.number("position_shares", risk_shares.min(cash_shares), "shares");
        }
        "book_market_regimes" => {
            let t = num(&v, "trend_indicators")?;
            let m = num(&v, "momentum_indicators")?;
            if t < 0.0 || m < 0.0 || t.fract() != 0.0 || m.fract() != 0.0 {
                return Err("指标数量必须为非负整数".into());
            }
            o.number(
                "redundant_indicators",
                (t - 2.0).max(0.0) + (m - 1.0).max(0.0),
                "count",
            );
            let mut groups = usize::from(t > 0.0) + usize::from(m > 0.0);
            for k in [
                "volatility_checked",
                "participation_checked",
                "execution_checked",
            ] {
                groups += usize::from(boolean(&v, k)?);
            }
            o.number("evidence_groups", groups as f64, "groups");
            o.number("missing_groups", (5 - groups) as f64, "groups");
        }
        "book_reading_order" | "book_data_conventions" => {
            let mut complete = 0;
            let fields = v.as_object().unwrap();
            for k in fields.keys() {
                let checked = boolean(&v, k)?;
                o.flag(k, checked);
                complete += usize::from(checked);
            }
            o.number("completed_steps", complete as f64, "steps");
            o.number("missing_steps", (fields.len() - complete) as f64, "steps");
            o.number(
                "completion_fraction",
                complete as f64 / fields.len() as f64,
                "fraction",
            );
            o.flag(
                if id == "book_reading_order" {
                    "review_complete"
                } else {
                    "reproducible"
                },
                complete == fields.len(),
            );
        }
        _ => unreachable!(),
    }
    o.note(d["pitfalls"].as_str().unwrap());
    Ok(
        json!({"concept_id":id,"input_kind":"independent_inputs","provenance":"editable_teaching_inputs","status":"computed","reason":null,"values":o.values,"units":o.units,"series":o.series,"notes":o.reasons,"inputs":v}),
    )
}
