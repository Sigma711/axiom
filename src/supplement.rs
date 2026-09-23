//! Source-backed exercises for external derivatives and blockchain measurements.
//! No exchange OHLCV is substituted for unavailable on-chain or position data.
use crate::{
    knowledge::KnowledgeEntry,
    practice::{arr, boolean, div, num, Output, PracticeConcept, PracticeInput},
    types::Bar,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use std::{collections::BTreeSet, sync::OnceLock};

fn uses_hourly_market_bars(id: &str) -> bool {
    id == "book_volume_24h"
}
fn uses_bitcoin_block_snapshot(id: &str) -> bool {
    matches!(id, "book_block_height" | "book_block_size")
}

pub fn definitions() -> &'static Vec<Value> {
    static DATA: OnceLock<Vec<Value>> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str::<Value>(include_str!("../docs/book/supplement.json"))
            .expect("checked book supplement")["concepts"]
            .as_array()
            .unwrap()
            .clone()
    })
}
fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().into()
}
pub fn entries() -> Vec<KnowledgeEntry> {
    definitions()
        .iter()
        .map(|d| KnowledgeEntry {
            id: text(d, "id"),
            name: text(d, "name"),
            category: text(d, "category"),
            summary: text(d, "summary"),
            meaning: text(d, "meaning"),
            formula: text(d, "formula"),
            example: text(d, "example"),
            pitfalls: text(d, "pitfalls"),
            related: vec![],
            code_url: "https://github.com/Sigma711/axiom/blob/main/src/supplement.rs".into(),
            code_ref: "src/supplement.rs::evaluate".into(),
            implementation: "src/supplement.rs::evaluate".into(),
            diagram: None,
            signals:
                "调整已标注的教学输入，观察数值变化；用于来源核对和风险分析，不自动生成买卖建议。"
                    .into(),
        })
        .collect()
}
pub fn catalog() -> Vec<PracticeConcept> {
    definitions()
        .iter()
        .map(|d| PracticeConcept {
            id: text(d, "id"),
            name: text(d, "name"),
            category: text(d, "category"),
            input_kind: if uses_hourly_market_bars(d["id"].as_str().unwrap_or_default()) || uses_bitcoin_block_snapshot(d["id"].as_str().unwrap_or_default()) {
                "market_bars".into()
            } else {
                "independent_inputs".into()
            },
            inputs: if uses_hourly_market_bars(d["id"].as_str().unwrap_or_default()) || uses_bitcoin_block_snapshot(d["id"].as_str().unwrap_or_default()) {
                json!({})
            } else {
                d["defaults"].clone()
            }
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| PracticeInput {
                key: k.clone(),
                label: d["labels"][k].as_str().unwrap_or(k).to_owned(),
                default: v.clone(),
            })
            .collect(),
            notes: if uses_bitcoin_block_snapshot(d["id"].as_str().unwrap_or_default()) {
                format!("原书 PDF 第{}页；服务器直接取得未缓存的10个连续Bitcoin主网区块，不接受客户端输入。{}", d["pdf_page"], text(d, "pitfalls"))
            } else if uses_hourly_market_bars(d["id"].as_str().unwrap_or_default()) {
                format!(
                    "原书 PDF 第{}页；仅从连续24根完整1小时加密市场OHLCV K线计算，不接受手填成交量。{}",
                    d["pdf_page"],
                    text(d, "pitfalls")
                )
            } else {
                format!(
                    "原书 PDF 第{}页；需独立外部观测，默认值是可编辑教学示例。{}",
                    d["pdf_page"],
                    text(d, "pitfalls")
                )
            },
        })
        .collect()
}
fn positive(v: &Value, k: &str) -> Result<f64, String> {
    let n = num(v, k)?;
    if n <= 0.0 {
        return Err(format!("{k} 必须>0"));
    }
    Ok(n)
}
fn nonnegative(v: &Value, k: &str) -> Result<f64, String> {
    let n = num(v, k)?;
    if n < 0.0 {
        return Err(format!("{k} 不得为负"));
    }
    Ok(n)
}
fn samples(v: &Value, k: &str) -> Result<Vec<f64>, String> {
    let a = arr(v, k)?;
    if a.is_empty() || a.iter().any(|x| *x < 0.0) {
        return Err(format!("{k} 需要非空、非负数值数组"));
    }
    Ok(a)
}
fn same(a: &[f64], b: &[f64]) -> Result<(), String> {
    if a.len() != b.len() {
        Err("输入数组必须逐项对齐".into())
    } else {
        Ok(())
    }
}
fn addresses(v: &Value, k: &str) -> Result<BTreeSet<String>, String> {
    v[k].as_array()
        .ok_or_else(|| format!("{k} 必须是地址数组"))?
        .iter()
        .map(|a| {
            a.as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| "地址不能为空且必须是字符串".into())
        })
        .collect()
}
fn ratio(o: &mut Output, key: &str, a: f64, b: f64, unit: &str) {
    o.value(key, div(a, b), unit, "分母为零，结果没有定义");
}
fn continuous_24_hour_bars(bars: &[Bar]) -> Result<&[Bar], String> {
    if bars.len() < 24 {
        return Err("需要至少24根连续、完整的1小时加密市场OHLCV K线".into());
    }
    let window = &bars[bars.len() - 24..];
    crate::practice::validate_bars(window)?;
    if window
        .windows(2)
        .any(|pair| (pair[1].timestamp - pair[0].timestamp).num_seconds() != 3600)
    {
        return Err("24小时成交量要求相邻K线严格相隔1小时；拒绝缺口、非小时和日线数据".into());
    }
    if window.last().unwrap().timestamp + Duration::hours(1) > Utc::now() {
        return Err("最后一根1小时K线尚未完整收盘或时间在未来".into());
    }
    Ok(window)
}
pub fn evaluate(id: &str, bars: &[Bar], inputs: &Value) -> Result<Value, String> {
    if uses_bitcoin_block_snapshot(id) {
        return Err(
            "Bitcoin block snapshot practice requires the server-fetched Bitcoin mainnet endpoint"
                .into(),
        );
    }
    let definition = definitions()
        .iter()
        .find(|d| d["id"] == id)
        .ok_or_else(|| format!("未知外部概念: {id}"))?;
    let mut v = if uses_hourly_market_bars(id) {
        json!({})
    } else {
        definition["defaults"].clone()
    };
    for (k, value) in inputs.as_object().ok_or("inputs 必须是对象")? {
        if v.get(k).is_none() {
            return Err(format!("不支持输入{k}"));
        }
        v[k] = value.clone();
    }
    let unit = definition["unit"].as_str().unwrap();
    let mut o = Output::default();
    match definition["op"].as_str().unwrap() {
        "cape" => {
            let earnings = arr(&v, "annual_eps")?;
            let cpi = samples(&v, "annual_cpi")?;
            if earnings.len() != 10 || cpi.len() != 10 || cpi.iter().any(|v| *v <= 0.0) {
                return Err("CAPE需要十个完整年度EPS与十个正值CPI，逐年对齐".into());
            }
            let current = positive(&v, "current_cpi")?;
            let price = positive(&v, "price")?;
            let adjusted: Vec<_> = earnings
                .iter()
                .zip(&cpi)
                .map(|(eps, cpi)| eps * current / cpi)
                .collect();
            let average = adjusted.iter().sum::<f64>() / 10.0;
            o.number("inflation_adjusted_mean_eps", average, "元/股");
            o.value(
                "cape",
                (average > 0.0).then(|| div(price, average)).flatten(),
                "倍",
                "通胀调整后平均利润非正，CAPE不适用于正常估值比较",
            );
            o.series(
                "inflation_adjusted_eps",
                adjusted.into_iter().map(Some).collect(),
                "元/股",
            );
        }
        "sum24" => {
            let bars = continuous_24_hour_bars(bars)?;
            let volumes = bars.iter().map(|bar| bar.volume).collect::<Vec<_>>();
            o.number("rolling_24h_volume", volumes.iter().sum(), unit);
            o.series(
                "hourly_volume",
                volumes.into_iter().map(Some).collect(),
                unit,
            );
        }
        "basis" => {
            let spot = positive(&v, "spot")?;
            let future = positive(&v, "future")?;
            o.number("basis", future - spot, unit);
            o.number("premium_fraction", future / spot - 1.0, "fraction");
        }
        "funding" => {
            let size = nonnegative(&v, "notional")?;
            let rate = num(&v, "rate_per_settlement")?;
            let intervals = positive(&v, "settlements_per_day")?;
            let sign = if boolean(&v, "is_long")? { 1.0 } else { -1.0 };
            o.number("payment", sign * size * rate, unit);
            o.number(
                "daily_payment_if_rate_constant",
                sign * size * rate * intervals,
                unit,
            );
            o.note("正payment表示支付，负值表示收取。日费用仅为费率恒定假设，不是未来费率预测。");
        }
        "weighted" => {
            let p = samples(&v, "prices")?;
            let w = samples(&v, "weights")?;
            same(&p, &w)?;
            ratio(
                &mut o,
                "weighted_index",
                p.iter().zip(&w).map(|(p, w)| p * w).sum(),
                w.iter().sum(),
                unit,
            );
        }
        "mark" => {
            let mark = positive(&v, "mark_price")?;
            o.number(
                "unrealized_pnl",
                (mark - positive(&v, "entry_price")?) * num(&v, "signed_size")?,
                unit,
            );
            o.number(
                "last_mark_difference",
                positive(&v, "last_price")? - mark,
                unit,
            );
            o.note("输入的标记价须来自交易所；此处不编造跨交易所通用标记价公式或强平价格。");
        }
        "open_interest" => {
            let previous = nonnegative(&v, "previous_contracts")?;
            let opened = nonnegative(&v, "new_contracts")?;
            let closed = nonnegative(&v, "closed_contracts")?;
            if closed > previous + opened {
                return Err("关闭数量不可超过现有+新增数量".into());
            }
            o.number("current_count", previous + opened - closed, unit);
            o.number("net_change", opened - closed, unit);
        }
        "liquidations" => {
            let long = samples(&v, "long_liquidations")?;
            let short = samples(&v, "short_liquidations")?;
            same(&long, &short)?;
            let a: f64 = long.iter().sum();
            let b: f64 = short.iter().sum();
            o.number("long_liquidations", a, unit);
            o.number("short_liquidations", b, unit);
            o.number("total_liquidations", a + b, unit);
            ratio(&mut o, "long_share", a, a + b, "fraction");
        }
        "long_short" => {
            let a = nonnegative(&v, "long")?;
            let b = nonnegative(&v, "short")?;
            ratio(&mut o, "long_short_ratio", a, b, "ratio");
            ratio(&mut o, "long_fraction", a, a + b, "fraction");
        }
        "balances" => {
            let old = nonnegative(&v, "previous_units")?;
            let current = nonnegative(&v, "current_units")?;
            o.number("balance", current, unit);
            o.number("unit_change", current - old, unit);
            o.number("market_value", current * positive(&v, "price")?, "currency");
        }
        "flows" => {
            let c = samples(&v, "creations")?;
            let r = samples(&v, "redemptions")?;
            same(&c, &r)?;
            let net: Vec<f64> = c.iter().zip(r).map(|(c, r)| c - r).collect();
            o.number("total_net_flows", net.iter().sum(), unit);
            o.series("net_flows", net.into_iter().map(Some).collect(), unit);
        }
        "ratio" => {
            let active = nonnegative(&v, "active_supply")?;
            let total = nonnegative(&v, "total_supply")?;
            if active > total {
                return Err("活跃供应不可超过总供应".into());
            }
            ratio(&mut o, "active_supply_fraction", active, total, unit);
        }
        "unique" => o.number(
            "unique_addresses",
            addresses(&v, "addresses")?.len() as f64,
            unit,
        ),
        "threshold" => {
            let b = samples(&v, "balances")?;
            let threshold = nonnegative(&v, "threshold")?;
            let total = positive(&v, "total_supply")?;
            if b.iter().sum::<f64>() > total {
                return Err("样本地址总余额超过供应量".into());
            }
            let selected: Vec<f64> = b.into_iter().filter(|x| *x >= threshold).collect();
            let held: f64 = selected.iter().sum();
            o.number("address_count", selected.len() as f64, "addresses");
            o.number("held_tokens", held, "tokens");
            o.number("supply_fraction", held / total, "fraction");
            o.number("held_value", held * positive(&v, "price")?, "currency");
        }
        "large_volume" => {
            let a = samples(&v, "volumes")?;
            let threshold = nonnegative(&v, "large_threshold")?;
            o.number("mean_volume", a.iter().sum::<f64>() / a.len() as f64, unit);
            o.number(
                "large_volume",
                a.iter().filter(|x| **x > threshold).sum(),
                unit,
            );
        }
        "blocks" => {
            let start = nonnegative(&v, "start_height")?;
            let end = nonnegative(&v, "end_height")?;
            if end < start || start.fract() != 0.0 || end.fract() != 0.0 {
                return Err("高度必须为递增非负整数".into());
            }
            o.number("block_height", end, unit);
            o.number("blocks_mined", end - start, unit);
        }
        "hashrate" => {
            o.number(
                "estimated_hashrate",
                positive(&v, "bitcoin_difficulty")? * 4294967296.0
                    / positive(&v, "mean_block_seconds")?,
                unit,
            );
            o.note("这是Bitcoin工作量证明难度定义下的估计。不能跨PoW算法比较难度或用于PoS链。");
        }
        "statistics" => {
            let mut a = samples(&v, "observations")?;
            o.number("count", a.len() as f64, "observations");
            o.number("total", a.iter().sum(), unit);
            o.number("mean", a.iter().sum::<f64>() / a.len() as f64, unit);
            o.series("observations", a.iter().copied().map(Some).collect(), unit);
            a.sort_by(f64::total_cmp);
            let m = a.len() / 2;
            o.number(
                "median",
                if a.len() % 2 == 0 {
                    (a[m - 1] + a[m]) / 2.0
                } else {
                    a[m]
                },
                unit,
            );
        }
        "utxo_totals" => {
            o.number(
                "created_value",
                samples(&v, "created_values")?.iter().sum(),
                unit,
            );
            o.number(
                "spent_value",
                samples(&v, "spent_values")?.iter().sum(),
                unit,
            );
        }
        "rate" => {
            o.number("count", nonnegative(&v, "count")?, "count");
            o.number(
                "rate",
                nonnegative(&v, "count")? / positive(&v, "elapsed_seconds")?,
                unit,
            );
        }
        "address_sets" => {
            let s = addresses(&v, "senders")?;
            let r = addresses(&v, "receivers")?;
            o.number("sending_addresses", s.len() as f64, unit);
            o.number("receiving_addresses", r.len() as f64, unit);
            o.number("both_directions", s.intersection(&r).count() as f64, unit);
        }
        "deposits" => {
            let a = samples(&v, "new_values")?;
            o.number("new_deposits", a.len() as f64, unit);
            o.number(
                "total_deposits",
                nonnegative(&v, "previous_total")? + a.len() as f64,
                unit,
            );
            o.number("new_tokens", a.iter().sum(), "tokens");
        }
        "new_addresses" => {
            let old = addresses(&v, "previous_addresses")?;
            let new = addresses(&v, "new_addresses")?;
            o.number(
                "new_unique_addresses",
                new.difference(&old).count() as f64,
                unit,
            );
            o.number(
                "total_unique_addresses",
                old.union(&new).count() as f64,
                unit,
            );
        }
        "staked" => {
            let added: f64 = samples(&v, "new_stakes")?.iter().sum();
            let withdrawn: f64 = samples(&v, "withdrawals")?.iter().sum();
            let total = nonnegative(&v, "previous_tokens")? + added - withdrawn;
            if total < 0.0 {
                return Err("提取量超过质押量".into());
            }
            o.number("new_staked_tokens", added, unit);
            o.number("net_new_tokens", added - withdrawn, unit);
            o.number("total_staked_tokens", total, unit);
            o.number("staked_value", total * positive(&v, "price")?, "currency");
        }
        "realized" => {
            let a = samples(&v, "token_amounts")?;
            let p = samples(&v, "last_moved_prices")?;
            same(&a, &p)?;
            o.number(
                "realized_market_cap",
                a.iter().zip(p).map(|(a, p)| a * p).sum(),
                unit,
            );
        }
        "sopr" => {
            let a = samples(&v, "amounts")?;
            let p = samples(&v, "creation_prices")?;
            let q = samples(&v, "spent_prices")?;
            same(&a, &p)?;
            same(&a, &q)?;
            ratio(
                &mut o,
                "sopr",
                a.iter().zip(q).map(|(a, p)| a * p).sum(),
                a.iter().zip(p).map(|(a, p)| a * p).sum(),
                unit,
            );
        }
        "stock_flow" => {
            let stock = nonnegative(&v, "stock_tokens")?;
            ratio(
                &mut o,
                "stock_to_flow",
                stock,
                nonnegative(&v, "annual_new_tokens")?,
                unit,
            );
            o.number(
                "stock_market_value",
                stock * positive(&v, "price")?,
                "currency",
            );
        }
        "power_law" => {
            let days = positive(&v, "days_since_origin")?;
            let model = positive(&v, "coefficient")? * days.powf(num(&v, "exponent")?);
            o.number("historical_model_price", model, unit);
            ratio(
                &mut o,
                "relative_to_model",
                positive(&v, "observed_price")? - model,
                model,
                "fraction",
            );
            o.note("a与b是用户提供的历史拟合参数。这里没有拟合优度、因果识别或未来预测保证。");
        }
        "equality" => ratio(
            &mut o,
            "supply_equality_ratio",
            nonnegative(&v, "small_holder_tokens")?,
            nonnegative(&v, "top_one_percent_tokens")?,
            unit,
        ),
        "rvt" => {
            let a = samples(&v, "daily_adjusted_transfer_values")?;
            if a.len() != 90 {
                return Err("RVT90需要90个完整日的对齐样本".into());
            }
            ratio(
                &mut o,
                "rvt_90",
                nonnegative(&v, "realized_market_cap")?,
                a.iter().sum::<f64>() / 90.0,
                unit,
            );
        }
        op => return Err(format!("unregistered supplement operation: {op}")),
    }
    o.note(&text(definition, "pitfalls"));
    if uses_hourly_market_bars(id) {
        o.note("仅使用调用方提供的连续24根完整1小时加密市场OHLCV K线；不使用手填成交量数组或未来数据。");
    } else {
        o.note(
            "这是独立可编辑教学输入，不是当前币种真实数据；用于历史分析时必须核对数据发布时点。",
        );
    }
    let computed = o.values.values().any(|v| !v.is_null());
    Ok(
        json!({"concept_id":id,"input_kind":if uses_hourly_market_bars(id) {"market_bars"} else {"independent_inputs"},"provenance":if uses_hourly_market_bars(id) {"provided_market_bars"} else {"editable_teaching_inputs"},"status":if computed{"computed"}else{"undefined"},"reason":if computed{Value::Null}else{json!(o.reasons.join("；"))},"values":o.values,"units":o.units,"series":o.series,"notes":o.reasons,"inputs":v}),
    )
}
