use super::*;
use crate::indicators::{breadth, options, shareholder, statistics as stats};
pub(super) fn defaults(id: &str) -> Option<Value> {
    Some(match id {
        "turnover_rate" => json!({"volume_shares":500000.0,"float_shares":10000000.0}),
        "bid_ask_spread" => json!({"bid":99.9,"ask":100.1}),
        "inside_outside" => {
            json!({"buyer_initiated_volume":6000.0,"seller_initiated_volume":4000.0})
        }
        "eps" => {
            json!({"net_income":3000000.0,"preferred_dividends":0.0,"weighted_average_shares":1000000.0})
        }
        "pe" => json!({"price":60.0,"eps":3.0}),
        "pb" => json!({"market_cap":60000000.0,"equity":30000000.0}),
        "ps" => json!({"market_cap":60000000.0,"revenue":50000000.0}),
        "peg" => json!({"pe":20.0,"earnings_growth_percent":15.0}),
        "ev_ebitda" => {
            json!({"market_cap":60000000.0,"debt":10000000.0,"cash":5000000.0,"ebitda":8000000.0})
        }
        "roe" => json!({"net_income":3000000.0,"average_equity":30000000.0}),
        "roic" => json!({"ebit":5000000.0,"tax_rate":0.25,"invested_capital":25000000.0}),
        "dupont" => json!({"net_margin":0.1,"asset_turnover":0.8,"equity_multiplier":2.0}),
        "fcf" => json!({"operating_cash_flow":5000000.0,"capital_expenditure":2000000.0}),
        "piotroski" => {
            json!({"positive_roa":true,"positive_operating_cf":true,"roa_increasing":true,"cash_roa_above_roa":true,"leverage_decreased":true,"current_ratio_improved":true,"shares_issued":false,"gross_margin_increased":true,"asset_turnover_increased":true})
        }
        "altman_z" => {
            json!({"working_capital":20.0,"retained_earnings":25.0,"ebit":10.0,"market_equity":80.0,"sales":100.0,"total_assets":100.0,"total_liabilities":40.0})
        }
        "ad_line" => {
            json!({"advances":[600.0,550.0,650.0],"declines":[400.0,450.0,350.0],"initial_ad":0.0})
        }
        "trin" => {
            json!({"advances":600.0,"declines":400.0,"up_volume":900000.0,"down_volume":600000.0})
        }
        "mcclellan" => {
            json!({"net_advances":[100.0,120.0,80.0,160.0,150.0,130.0,110.0,180.0,160.0,200.0,170.0,140.0,120.0,220.0,200.0,180.0,210.0,240.0,230.0,250.0,220.0,200.0,240.0,270.0,260.0,280.0,250.0,290.0,280.0,300.0,260.0,250.0,310.0,300.0,320.0,330.0,290.0,340.0,350.0,320.0]})
        }
        "rolling_correlation" => {
            json!({"series_x":[],"series_y":[],"period":3})
        }
        "total_return" | "cagr" | "max_drawdown" | "calmar" => {
            json!({"equity":[10000.0,10500.0,10200.0,11000.0],"elapsed_days":365.0})
        }
        "sharpe" | "sortino" | "var" | "cvar" | "skewness" | "kurtosis" => {
            json!({"returns":[0.01,-0.02,0.03,0.015,-0.01,0.02],"periods_per_year":252.0,"risk_free_annual":0.02,"confidence":0.95})
        }
        "beta" | "alpha" | "information_ratio" | "treynor" | "tracking_error" | "capture_ratio" => {
            json!({"strategy_returns":[0.02,-0.01,0.025,-0.015,0.03,0.01],"benchmark_returns":[0.01,-0.02,0.02,-0.01,0.02,0.005],"periods_per_year":252.0,"risk_free_annual":0.02})
        }
        "delta" | "gamma" | "theta" | "vega" | "iv" => {
            json!({"spot":100.0,"strike":100.0,"time_years":1.0,"rate":0.05,"volatility":0.2,"is_call":true,"option_price":10.4506})
        }
        "iv_rank" => json!({"current_iv":0.3,"min_iv":0.15,"max_iv":0.45}),
        "pitfall_low_pe" | "pitfall_high_pe" => {
            json!({"price":60.0,"reported_eps":6.0,"normalized_eps":3.0,"earnings_growth_percent":15.0})
        }
        "pitfall_main_flow" => {
            json!({"buyer_initiated_volume":6000.0,"seller_initiated_volume":4000.0,"classified_volume":10000.0,"total_volume":16000.0})
        }
        "pitfall_params" => {
            json!({"train_returns":[0.2,0.35,0.24],"test_returns":[0.1,-0.12,0.08],"trials":3})
        }
        "pitfall_industry" | "industry_pe_compare" => {
            json!({"company_pe":20.0,"peer_pe":[12.0,16.0,18.0,24.0,30.0],"same_industry":true})
        }
        "elliott_wave" => {
            json!({"annotated_prices":[100.0,110.0,105.0,125.0,115.0,135.0],"direction":1.0})
        }
        "cvd" => json!({"buy_volume":[100.0,150.0,90.0],"sell_volume":[80.0,100.0,110.0]}),
        "volume_profile" => {
            json!({"prices":[98.0,99.0,100.0,101.0,102.0],"volumes":[50.0,100.0,300.0,150.0,80.0],"value_area_fraction":0.7})
        }
        "tpo" => {
            json!({"prices":[98.0,99.0,100.0,101.0,102.0],"time_counts":[1.0,3.0,8.0,5.0,2.0],"value_area_fraction":0.7})
        }
        "footprint" => {
            json!({"prices":[99.0,100.0,101.0],"ask_volume":[80.0,200.0,120.0],"bid_volume":[100.0,100.0,40.0],"imbalance_ratio":3.0})
        }
        "chip_distribution" => {
            json!({"cost_prices":[80.0,90.0,100.0,110.0],"held_shares":[100.0,200.0,400.0,100.0],"current_price":105.0})
        }
        "ddx_ddy_ddz" => {
            json!({"large_buy_volume":600.0,"large_sell_volume":400.0,"float_shares":100000.0,"buy_orders":200.0,"sell_orders":260.0,"total_orders":1000.0})
        }
        "new_high_low" => json!({"new_highs":100.0,"new_lows":25.0}),
        "tick" => json!({"uptick_count":600.0,"downtick_count":400.0}),
        "breadth_thrust" => {
            json!({"advance_fractions":[0.35,0.38,0.4,0.45,0.5,0.6,0.65,0.7,0.75,0.8,0.8,0.85],"period":10})
        }
        "bullish_percent" => json!({"point_figure_buy_signals":60.0,"universe_size":100.0}),
        "up_down_volume" => json!({"up_volume":900000.0,"down_volume":600000.0}),
        "institution_holding" => json!({"institution_shares":500000.0,"total_shares":1000000.0}),
        "insider_trading" => {
            json!({"shares_bought":10000.0,"shares_sold":3000.0,"total_shares":1000000.0})
        }
        "holder_concentration" => {
            json!({"top_holder_shares":[100000.0,80000.0,60000.0],"total_shares":1000000.0})
        }
        "share_pledge" => json!({"pledged_shares":200000.0,"total_shares":1000000.0}),
        "restricted_shares" => {
            json!({"unlock_shares":50000.0,"price":60.0,"float_shares":800000.0})
        }
        "buyback_rate" => json!({"buyback_shares":20000.0,"total_shares":1000000.0}),
        "short_interest" => {
            json!({"short_shares":80000.0,"float_shares":800000.0,"average_daily_volume":20000.0})
        }
        "goodwill_ratio" => json!({"goodwill":2000000.0,"equity":10000000.0}),
        "accrual_ratio" => {
            json!({"net_income":3000000.0,"operating_cash_flow":5000000.0,"average_assets":30000000.0})
        }
        "beneish_m" => {
            json!({"dsri":1.1,"gmi":1.05,"aqi":1.0,"sgi":1.1,"depi":1.0,"sgai":1.0,"lvgi":1.0,"tata":0.02})
        }
        "consensus" => {
            json!({"ratings":[1.0,1.0,2.0,3.0,2.0],"eps_estimates":[2.0,4.0,4.0,5.0,10.0],"revenue_estimates":[100.0,110.0,120.0,130.0,140.0],"target_prices":[100.0,110.0,120.0,130.0,140.0]})
        }
        "target_upside" => json!({"target_prices":[110.0,120.0,100.0],"current_price":100.0}),
        "forecast_dispersion" => json!({"estimates":[2.0,2.5,3.0,2.5]}),
        "earnings_surprise" => json!({"actual_eps":3.0,"consensus_eps":2.5}),
        "revision" => json!({"upward_revisions":12.0,"downward_revisions":4.0,"unchanged":4.0}),
        "iv_term_structure" => {
            json!({"maturity_years":[0.08333,0.25,0.5,1.0],"implied_volatilities":[0.3,0.28,0.25,0.24]})
        }
        "skew" => json!({"put_25delta_iv":0.3,"call_25delta_iv":0.24}),
        "put_call_ratio" => json!({"put_volume":1200.0,"call_volume":1000.0}),
        "max_pain" => {
            json!({"strikes":[90.0,100.0,110.0],"call_open_interest":[100.0,200.0,150.0],"put_open_interest":[150.0,250.0,100.0],"contract_multiplier":100.0})
        }
        "gex_dex" => {
            json!({"spot":100.0,"gammas":[0.02,0.015],"deltas":[0.6,-0.4],"signed_contracts":[1000.0,-500.0],"contract_multiplier":100.0})
        }
        "bank_nim" => {
            json!({"interest_income":5000000.0,"interest_expense":2000000.0,"average_earning_assets":100000000.0})
        }
        _ => return None,
    })
}
fn positive(v: &Value, k: &str) -> Result<f64, String> {
    let n = num(v, k)?;
    if n <= 0.0 {
        return Err(format!("{k} 必须大于零"));
    }
    Ok(n)
}
fn nonnegative(v: &Value, k: &str) -> Result<f64, String> {
    let n = num(v, k)?;
    if n < 0.0 {
        return Err(format!("{k} 不得为负数"));
    }
    Ok(n)
}
fn paired(a: &[f64], b: &[f64]) -> Result<(), String> {
    if a.len() != b.len() || a.is_empty() {
        Err("配对数组必须非空且等长，并按同一时点对齐".into())
    } else {
        Ok(())
    }
}
fn nonnegative_array(a: &[f64]) -> Result<(), String> {
    if a.iter().any(|x| *x < 0.0) {
        Err("数量/权重数组不得为负".into())
    } else {
        Ok(())
    }
}
pub(super) fn evaluate(id: &str, v: &Value, o: &mut Output) -> Result<(), String> {
    let x = |k| num(v, k);
    let a = |k| arr(v, k);
    macro_rules! ratio {
        ($key:expr,$n:expr,$d:expr,$u:expr) => {
            o.value($key, div($n, $d), $u, "分母为零，该指标无定义")
        };
    }
    match id {
        "turnover_rate" => {
            ratio!(
                id,
                nonnegative(v, "volume_shares")?,
                nonnegative(v, "float_shares")?,
                "fraction"
            );
        }
        "bid_ask_spread" => {
            let bid = positive(v, "bid")?;
            let ask = positive(v, "ask")?;
            if ask < bid {
                return Err("ask 不得小于 bid".into());
            }
            o.number("spread", ask - bid, "price");
            ratio!("relative_spread", ask - bid, (ask + bid) / 2.0, "fraction");
        }
        "inside_outside" => {
            let buy = nonnegative(v, "buyer_initiated_volume")?;
            let sell = nonnegative(v, "seller_initiated_volume")?;
            o.number("outside_buy_volume", buy, "shares/units");
            o.number("inside_sell_volume", sell, "shares/units");
            ratio!("aggressor_imbalance", buy - sell, buy + sell, "fraction");
            o.note("必须使用逐笔主动成交分类，K 线涨跌不能替代内外盘。");
        }
        "eps" => ratio!(
            id,
            x("net_income")? - x("preferred_dividends")?,
            nonnegative(v, "weighted_average_shares")?,
            "currency/share"
        ),
        "pe" => {
            let eps = x("eps")?;
            o.value(
                id,
                if eps > 0.0 {
                    div(nonnegative(v, "price")?, eps)
                } else {
                    None
                },
                "multiple",
                "EPS <= 0 时 PE 无经济可比性",
            );
        }
        "pb" => {
            let eq = x("equity")?;
            o.value(
                id,
                if eq > 0.0 {
                    div(nonnegative(v, "market_cap")?, eq)
                } else {
                    None
                },
                "multiple",
                "净资产非正时 PB 不适用",
            );
        }
        "ps" => ratio!(
            id,
            nonnegative(v, "market_cap")?,
            nonnegative(v, "revenue")?,
            "multiple"
        ),
        "peg" => {
            let g = x("earnings_growth_percent")?;
            o.value(
                id,
                if g > 0.0 { div(x("pe")?, g) } else { None },
                "multiple / growth percentage point",
                "增长率非正时 PEG 不适用",
            );
        }
        "ev_ebitda" => {
            let e = x("ebitda")?;
            o.value(
                id,
                if e > 0.0 {
                    div(x("market_cap")? + x("debt")? - x("cash")?, e)
                } else {
                    None
                },
                "multiple",
                "EBITDA 非正时不可作正常估值比较",
            );
        }
        "roe" => ratio!(id, x("net_income")?, x("average_equity")?, "fraction"),
        "roic" => {
            let tax = x("tax_rate")?;
            if !(0.0..=1.0).contains(&tax) {
                return Err("tax_rate 必须为0..1".into());
            }
            ratio!(
                id,
                x("ebit")? * (1.0 - tax),
                x("invested_capital")?,
                "fraction"
            );
        }
        "dupont" => o.number(
            id,
            x("net_margin")? * x("asset_turnover")? * x("equity_multiplier")?,
            "fraction",
        ),
        "fcf" => o.number(
            id,
            x("operating_cash_flow")? - nonnegative(v, "capital_expenditure")?,
            "currency",
        ),
        "piotroski" => {
            let keys = [
                "positive_roa",
                "positive_operating_cf",
                "roa_increasing",
                "cash_roa_above_roa",
                "leverage_decreased",
                "current_ratio_improved",
                "gross_margin_increased",
                "asset_turnover_increased",
            ];
            let mut score = 0;
            for k in keys {
                if boolean(v, k)? {
                    score += 1;
                }
            }
            if !boolean(v, "shares_issued")? {
                score += 1;
            }
            o.number(id, score as f64, "score 0..9");
            o.note("shares_issued=true 表示发行新股，因此不得分；cash_roa_above_roa 表示 CFO/资产 > ROA。");
        }
        "altman_z" => {
            let assets = positive(v, "total_assets")?;
            let liabilities = positive(v, "total_liabilities")?;
            o.number(
                id,
                1.2 * x("working_capital")? / assets
                    + 1.4 * x("retained_earnings")? / assets
                    + 3.3 * x("ebit")? / assets
                    + 0.6 * x("market_equity")? / liabilities
                    + x("sales")? / assets,
                "Z score",
            );
            o.note("原始上市制造业 Altman Z 系数，不能直接套用银行或所有行业。");
        }
        "ad_line" => {
            let up = a("advances")?;
            let down = a("declines")?;
            paired(&up, &down)?;
            nonnegative_array(&up)?;
            nonnegative_array(&down)?;
            let mut s = x("initial_ad")?;
            o.series(
                id,
                up.iter()
                    .zip(down)
                    .map(|(u, d)| {
                        s += u - d;
                        Some(s)
                    })
                    .collect(),
                "net issues cumulative",
            );
        }
        "trin" => {
            let advances = nonnegative(v, "advances")?;
            let declines = nonnegative(v, "declines")?;
            let uv = nonnegative(v, "up_volume")?;
            let dv = nonnegative(v, "down_volume")?;
            o.value(
                id,
                div(advances, declines).and_then(|q| div(uv, dv).and_then(|r| div(q, r))),
                "ratio",
                "需要非零下跌家数和涨跌成交量",
            );
        }
        "mcclellan" => {
            let net = a("net_advances")?;
            o.series(id, breadth::mcclellan_oscillator(&net), "net issues");
            o.note("传统未标准化净上涨家数 EMA19 - EMA39；不是 A/D 累积线的差值。");
        }
        "rolling_correlation" => {
            let aa = a("series_x")?;
            let bb = a("series_y")?;
            paired(&aa, &bb)?;
            let p = period(v, "period")?;
            if p < 2 || p > aa.len() {
                return Err("correlation period 必须是 2..=已核验收益样本数的整数".into());
            }
            o.note("策略净值逐期收益与同时间戳标的收盘收益的滚动相关性；不是双资产配对交易证据。");
            o.series(id, stats::rolling_correlation(&aa, &bb, p), "correlation");
        }
        "total_return" | "cagr" | "max_drawdown" | "calmar" => {
            let eq = a("equity")?;
            if eq.len() < 2 || eq.iter().any(|x| *x <= 0.0) {
                return Err("equity 需至少两个正值，并包含交易前初始资金".into());
            }
            let days = positive(v, "elapsed_days")?;
            let total = eq[eq.len() - 1] / eq[0] - 1.0;
            let cagr = (1.0 + total).powf(365.0 / days) - 1.0;
            let mut peak = eq[0];
            let dd: Vec<_> = eq
                .iter()
                .map(|z| {
                    peak = peak.max(*z);
                    1.0 - z / peak
                })
                .collect();
            let maxdd = dd.iter().copied().fold(0.0, f64::max);
            match id {
                "total_return" => o.number(id, total, "fraction"),
                "cagr" => o.number(id, cagr, "annualized fraction"),
                "max_drawdown" => {
                    o.number(id, maxdd, "fraction");
                    o.series("drawdown", dd.into_iter().map(Some).collect(), "fraction");
                }
                "calmar" => ratio!(id, cagr, maxdd, "ratio"),
                _ => unreachable!(),
            };
            o.note("equity 是独立输入的净值路径，含初始资金；默认是教学路径，调用模块应覆盖为实际策略净值。");
        }
        "sharpe" | "sortino" | "var" | "cvar" | "skewness" | "kurtosis" => {
            let r = a("returns")?;
            if r.is_empty() || r.iter().any(|x| *x < -1.0) {
                return Err("returns 不能为空且简单收益率不得低于-1".into());
            }
            let periods = positive(v, "periods_per_year")?;
            let rf = x("risk_free_annual")? / periods;
            let excess: Vec<_> = r.iter().map(|x| x - rf).collect();
            match id {
                "sharpe" => o.value(
                    id,
                    stddev(&excess).and_then(|s| div(mean(&excess).unwrap() * periods.sqrt(), s)),
                    "annualized ratio",
                    "收益方差为零或样本不足",
                ),
                "sortino" => {
                    let downside = (excess.iter().map(|x| x.min(0.0).powi(2)).sum::<f64>()
                        / excess.len() as f64)
                        .sqrt();
                    ratio!(
                        id,
                        mean(&excess).unwrap() * periods.sqrt(),
                        downside,
                        "annualized ratio"
                    );
                }
                "var" | "cvar" => {
                    let confidence = x("confidence")?;
                    if !(0.0..1.0).contains(&confidence) {
                        return Err("confidence 必须在0..1内".into());
                    }
                    let mut losses: Vec<_> = r.iter().map(|x| -x).collect();
                    losses.sort_by(f64::total_cmp);
                    let pos = (confidence * losses.len() as f64).ceil() as usize;
                    let q = losses[pos.saturating_sub(1)];
                    let tail: Vec<_> = losses.iter().copied().filter(|x| *x >= q).collect();
                    o.number(
                        id,
                        if id == "var" { q } else { mean(&tail).unwrap() },
                        "one-period loss fraction",
                    );
                    o.note("经验最近秩分位数；CVaR 平均所有损失>=VaR的样本，允许全正收益时负VaR。");
                }
                "skewness" => o.value(
                    id,
                    stats::skewness(&r),
                    "dimensionless",
                    "样本不足或方差为零",
                ),
                "kurtosis" => o.value(
                    id,
                    stats::kurtosis(&r),
                    "excess kurtosis",
                    "样本不足或方差为零",
                ),
                _ => unreachable!(),
            };
        }
        "beta" | "alpha" | "information_ratio" | "treynor" | "tracking_error" | "capture_ratio" => {
            let s = a("strategy_returns")?;
            let b = a("benchmark_returns")?;
            paired(&s, &b)?;
            if s.len() < 2 {
                return Err("至少两组对齐收益率".into());
            }
            let periods = positive(v, "periods_per_year")?;
            let rf = x("risk_free_annual")?;
            let beta = stats::beta(&s, &b);
            let active: Vec<_> = s.iter().zip(&b).map(|(s, b)| s - b).collect();
            let te = stddev(&active).map(|x| x * periods.sqrt());
            match id {
                "beta" => o.value(id, beta, "beta", "基准方差为零"),
                "alpha" => o.value(
                    id,
                    beta.map(|be| {
                        mean(&s).unwrap() * periods - rf - be * (mean(&b).unwrap() * periods - rf)
                    }),
                    "annualized arithmetic fraction",
                    "基准方差为零",
                ),
                "tracking_error" => o.value(id, te, "annualized fraction", "样本不足"),
                "information_ratio" => o.value(
                    id,
                    te.and_then(|te| div(mean(&active).unwrap() * periods, te)),
                    "annualized ratio",
                    "主动收益标准差为零",
                ),
                "treynor" => o.value(
                    id,
                    beta.and_then(|be| div(mean(&s).unwrap() * periods - rf, be)),
                    "ratio",
                    "Beta为零或无定义",
                ),
                "capture_ratio" => {
                    for (k, positive) in [("up_capture", true), ("down_capture", false)] {
                        let pairs: Vec<_> = s
                            .iter()
                            .zip(&b)
                            .filter(|(_, b)| if positive { **b > 0.0 } else { **b < 0.0 })
                            .collect();
                        let sr = pairs.iter().fold(1.0, |acc, (s, _)| acc * (1.0 + **s)) - 1.0;
                        let br = pairs.iter().fold(1.0, |acc, (_, b)| acc * (1.0 + **b)) - 1.0;
                        o.value(
                            k,
                            if pairs.is_empty() { None } else { div(sr, br) },
                            "fraction",
                            "该市场方向没有样本或基准收益为零",
                        );
                    }
                }
                _ => unreachable!(),
            };
            o.note("两组收益必须同频、同日期、同货币；默认是教学收益，不是当前策略的实际绩效。");
        }
        "delta" | "gamma" | "theta" | "vega" | "iv" => option_value(id, v, o)?,
        "iv_rank" => {
            let lo = nonnegative(v, "min_iv")?;
            let hi = nonnegative(v, "max_iv")?;
            let current = nonnegative(v, "current_iv")?;
            o.value(
                id,
                if hi > lo {
                    Some((current - lo) / (hi - lo) * 100.0)
                } else {
                    None
                },
                "percent",
                "历史IV范围必须大于零",
            );
        }
        "pitfall_low_pe" | "pitfall_high_pe" => {
            let price = nonnegative(v, "price")?;
            let reported = x("reported_eps")?;
            let normalized = x("normalized_eps")?;
            o.value(
                "reported_pe",
                if reported > 0.0 {
                    div(price, reported)
                } else {
                    None
                },
                "multiple",
                "EPS非正",
            );
            o.value(
                "normalized_pe",
                if normalized > 0.0 {
                    div(price, normalized)
                } else {
                    None
                },
                "multiple",
                "标准化EPS非正",
            );
            o.number("growth_percent", x("earnings_growth_percent")?, "percent");
            o.note("将一次性/周期盈利与标准化盈利区分；PE高低本身不构成便宜/昂贵结论。");
        }
        "pitfall_main_flow" => {
            let buy = nonnegative(v, "buyer_initiated_volume")?;
            let sell = nonnegative(v, "seller_initiated_volume")?;
            let classified = nonnegative(v, "classified_volume")?;
            let total = nonnegative(v, "total_volume")?;
            if classified > total || buy + sell > classified {
                return Err("主动成交量不能超过已分类量，已分类量不能超过总量".into());
            }
            o.number("classified_net_aggressor_volume", buy - sell, "units");
            ratio!("coverage", classified, total, "fraction");
            o.note("主动买卖净差不是机构身份，也不是市场净流入；缺失分类量不能假设为零。");
        }
        "pitfall_params" => {
            let train = a("train_returns")?;
            let test = a("test_returns")?;
            paired(&train, &test)?;
            let best = (0..train.len())
                .max_by(|i, j| train[*i].total_cmp(&train[*j]))
                .unwrap();
            o.number("train_selected_index", best as f64, "index");
            o.number("selected_train_return", train[best], "fraction");
            o.number("selected_test_return", test[best], "fraction");
            o.number("generalization_gap", train[best] - test[best], "fraction");
            o.number("declared_trials", nonnegative(v, "trials")?, "count");
            o.note("先按训练集选参数再报告其测试集表现；禁止再用该测试集重新选最佳参数，不能据此声称统计显著。");
        }
        "pitfall_industry" | "industry_pe_compare" => {
            let mut peers = a("peer_pe")?;
            peers.retain(|x| *x > 0.0);
            peers.sort_by(f64::total_cmp);
            let med = if peers.is_empty() {
                None
            } else {
                Some(if peers.len() % 2 == 1 {
                    peers[peers.len() / 2]
                } else {
                    (peers[peers.len() / 2 - 1] + peers[peers.len() / 2]) / 2.0
                })
            };
            let comparable = boolean(v, "same_industry")?;
            o.value("peer_median_pe", med, "multiple", "没有正PE同业样本");
            o.value(
                "relative_pe",
                if comparable {
                    med.and_then(|m| div(x("company_pe").unwrap(), m))
                } else {
                    None
                },
                "ratio",
                "不同产业或缺少样本不作直接估值比较",
            );
            o.flag("same_industry", comparable);
        }
        "elliott_wave" => {
            let q = a("annotated_prices")?;
            if q.len() != 6 {
                return Err("需人工提供起点与5浪终点，共6个价格".into());
            }
            let d = x("direction")?;
            if d != 1.0 && d != -1.0 {
                return Err("direction 只能 +1/-1".into());
            }
            let w: Vec<_> = q.iter().map(|x| x * d).collect();
            let lengths = [w[1] - w[0], w[3] - w[2], w[5] - w[4]];
            let alternating =
                w[1] > w[0] && w[2] < w[1] && w[3] > w[2] && w[4] < w[3] && w[5] > w[4];
            o.flag("alternating_impulse", alternating);
            o.flag("wave2_holds_origin", w[2] > w[0]);
            o.flag(
                "wave3_not_shortest",
                lengths[1] >= lengths[0].min(lengths[2]),
            );
            o.flag("wave4_no_wave1_overlap", w[4] > w[1]);
            o.note("只校验用户人工标注的普通推动浪基本约束；不自动识别、不保证唯一数浪，终结楔形等例外不适用。");
        }
        "cvd" => {
            let buy = a("buy_volume")?;
            let sell = a("sell_volume")?;
            paired(&buy, &sell)?;
            nonnegative_array(&buy)?;
            nonnegative_array(&sell)?;
            let mut total = 0.0;
            o.series(
                id,
                buy.iter()
                    .zip(sell)
                    .map(|(b, s)| {
                        total += b - s;
                        Some(total)
                    })
                    .collect(),
                "aggressor volume",
            );
        }
        "volume_profile" | "tpo" => {
            let prices = a("prices")?;
            let weights = a(if id == "tpo" {
                "time_counts"
            } else {
                "volumes"
            })?;
            profile(&prices, &weights, x("value_area_fraction")?, o)?;
            o.note(if id == "tpo" {
                "输入每个价格层在不同时间片出现次数；不能以成交量代替TPO。"
            } else {
                "输入真实逐笔或价位聚合成交量；不将整根K线成交量随意分配到价格层。"
            });
        }
        "footprint" => {
            let prices = a("prices")?;
            let ask = a("ask_volume")?;
            let bid = a("bid_volume")?;
            paired(&prices, &ask)?;
            paired(&ask, &bid)?;
            nonnegative_array(&ask)?;
            nonnegative_array(&bid)?;
            let threshold = positive(v, "imbalance_ratio")?;
            o.series(
                "price_levels",
                prices.into_iter().map(Some).collect(),
                "price",
            );
            o.series(
                "delta",
                ask.iter().zip(&bid).map(|(a, b)| Some(a - b)).collect(),
                "units",
            );
            o.series(
                "same_level_buy_imbalance",
                ask.iter()
                    .zip(bid)
                    .map(|(a, b)| div(*a, b).map(|r| if r >= threshold { 1.0 } else { 0.0 }))
                    .collect(),
                "boolean (1/0)",
            );
            o.note("采用同价位 Ask/Bid 不平衡定义；零Bid为undefined，不冒充对角不平衡。");
        }
        "chip_distribution" => {
            let prices = a("cost_prices")?;
            let shares = a("held_shares")?;
            paired(&prices, &shares)?;
            nonnegative_array(&shares)?;
            let current = positive(v, "current_price")?;
            let total = shares.iter().sum::<f64>();
            ratio!(
                "average_cost",
                prices.iter().zip(&shares).map(|(p, s)| p * s).sum::<f64>(),
                total,
                "currency/share"
            );
            ratio!(
                "profitable_share_fraction",
                prices
                    .iter()
                    .zip(&shares)
                    .filter(|(p, _)| **p < current)
                    .map(|(_, s)| s)
                    .sum::<f64>(),
                total,
                "fraction"
            );
            o.note("输入假设/已知的持仓成本分布；OHLCV 无法识别真实持有人换手与成本。");
        }
        "ddx_ddy_ddz" => {
            ratio!(
                "ddx",
                (nonnegative(v, "large_buy_volume")? - nonnegative(v, "large_sell_volume")?)
                    * 100.0,
                nonnegative(v, "float_shares")?,
                "percent"
            );
            ratio!(
                "ddy",
                (nonnegative(v, "sell_orders")? - nonnegative(v, "buy_orders")?) * 100.0,
                nonnegative(v, "total_orders")?,
                "percent"
            );
            o.value(
                "ddz",
                None,
                "vendor-specific",
                "DDZ 厂商公式/大单分类口径未公开统一定义，不用虚构常数代替",
            );
            o.note("DDX/ DDY 仅采用已注明的教学口径，实际终端可能有平滑和不同分母。");
        }
        "new_high_low" => {
            let hi = nonnegative(v, "new_highs")?;
            let lo = nonnegative(v, "new_lows")?;
            o.number("net_new_highs", hi - lo, "issues");
            ratio!("new_high_fraction", hi, hi + lo, "fraction");
        }
        "tick" => o.number(
            id,
            nonnegative(v, "uptick_count")? - nonnegative(v, "downtick_count")?,
            "issues",
        ),
        "breadth_thrust" => {
            let fractions = a("advance_fractions")?;
            if fractions.iter().any(|x| !(0.0..=1.0).contains(x)) {
                return Err("advance_fractions 范围0..1".into());
            }
            let p = period(v, "period")?;
            if p != 10 {
                return Err("Zweig breadth thrust 固定使用10期EMA".into());
            }
            let sm = crate::indicators::ma::ema(&fractions, 10);
            o.series("advance_fraction_ema", sm.clone(), "fraction");
            o.value(
                "zweig_thrust",
                if sm.last().copied().flatten().is_some() {
                    Some(if breadth::breadth_thrust(&fractions) {
                        1.0
                    } else {
                        0.0
                    })
                } else {
                    None
                },
                "boolean (1/0)",
                "EMA预热不足",
            );
            o.note("Zweig 口径：上涨家数占比的10期EMA，在10期内从<0.4升至>0.615。");
        }
        "bullish_percent" => o.number(
            id,
            breadth::bullish_percent_index(
                nonnegative(v, "point_figure_buy_signals")?,
                nonnegative(v, "universe_size")?,
            ),
            "percent",
        ),
        "up_down_volume" => ratio!(
            id,
            nonnegative(v, "up_volume")?,
            nonnegative(v, "down_volume")?,
            "ratio"
        ),
        "institution_holding" => ratio!(
            id,
            nonnegative(v, "institution_shares")?,
            nonnegative(v, "total_shares")?,
            "fraction"
        ),
        "insider_trading" => {
            let net = nonnegative(v, "shares_bought")? - nonnegative(v, "shares_sold")?;
            o.number("net_shares", net, "shares");
            ratio!(
                "net_fraction",
                net,
                nonnegative(v, "total_shares")?,
                "fraction"
            );
        }
        "holder_concentration" => {
            let shares = a("top_holder_shares")?;
            nonnegative_array(&shares)?;
            let total = nonnegative(v, "total_shares")?;
            ratio!(
                "top_holder_fraction",
                shares.iter().sum::<f64>(),
                total,
                "fraction"
            );
            o.value(
                "listed_holder_hhi",
                if total > 0.0 {
                    Some(shares.iter().map(|s| (s / total).powi(2)).sum())
                } else {
                    None
                },
                "fraction squared",
                "总股本为零",
            );
            o.note("HHI仅含输入所列持有人，不把未披露持有人假设为零。");
        }
        "share_pledge" => ratio!(
            id,
            nonnegative(v, "pledged_shares")?,
            nonnegative(v, "total_shares")?,
            "fraction"
        ),
        "restricted_shares" => {
            let shares = nonnegative(v, "unlock_shares")?;
            o.number(
                "unlock_market_value",
                shares * nonnegative(v, "price")?,
                "currency",
            );
            ratio!(
                "unlock_float_fraction",
                shares,
                nonnegative(v, "float_shares")?,
                "fraction"
            );
        }
        "buyback_rate" => ratio!(
            id,
            nonnegative(v, "buyback_shares")?,
            nonnegative(v, "total_shares")?,
            "fraction"
        ),
        "short_interest" => {
            let shares = nonnegative(v, "short_shares")?;
            ratio!(
                "short_float_fraction",
                shares,
                nonnegative(v, "float_shares")?,
                "fraction"
            );
            ratio!(
                "days_to_cover",
                shares,
                nonnegative(v, "average_daily_volume")?,
                "days"
            );
        }
        "goodwill_ratio" => ratio!(id, nonnegative(v, "goodwill")?, x("equity")?, "fraction"),
        "accrual_ratio" => ratio!(
            id,
            x("net_income")? - x("operating_cash_flow")?,
            nonnegative(v, "average_assets")?,
            "fraction"
        ),
        "beneish_m" => o.number(
            id,
            shareholder::beneish_m_score(
                x("dsri")?,
                x("gmi")?,
                x("aqi")?,
                x("sgi")?,
                x("depi")?,
                x("sgai")?,
                x("lvgi")?,
                x("tata")?,
            ),
            "M score",
        ),
        "consensus" => {
            let r = a("ratings")?;
            if r.iter().any(|x| *x < 1.0 || *x > 5.0) {
                return Err("ratings: 1=强买入..5=强卖出".into());
            }
            o.value("average_rating", mean(&r), "rating 1..5", "没有分析师评级");
            o.number("analyst_count", r.len() as f64, "count");
            let mut eps = a("eps_estimates")?;
            eps.sort_by(f64::total_cmp);
            let median = if eps.is_empty() {
                None
            } else if eps.len() % 2 == 0 {
                Some((eps[eps.len() / 2 - 1] + eps[eps.len() / 2]) / 2.0)
            } else {
                Some(eps[eps.len() / 2])
            };
            o.value("mean_eps", mean(&eps), "currency/share", "没有EPS预测");
            o.value("median_eps", median, "currency/share", "没有EPS预测");
            o.value(
                "eps_dispersion",
                mean(&eps).map(|m| {
                    (eps.iter().map(|x| (x - m).powi(2)).sum::<f64>() / eps.len() as f64).sqrt()
                }),
                "currency/share",
                "没有EPS预测",
            );
            o.value(
                "mean_revenue",
                mean(&a("revenue_estimates")?),
                "currency",
                "没有收入预测",
            );
            o.value(
                "mean_target_price",
                mean(&a("target_prices")?),
                "currency/share",
                "没有目标价预测",
            );
            o.note("均值、中位数和总体标准差描述分析师分布；评级1=强买入、5=强卖出。各项样本可能不同，预测不等于承诺。");
        }
        "target_upside" => {
            let targets = a("target_prices")?;
            let price = nonnegative(v, "current_price")?;
            o.value(
                id,
                mean(&targets).and_then(|t| div(t - price, price)),
                "fraction",
                "目标价缺失或当前价为零",
            );
        }
        "forecast_dispersion" => {
            let estimates = a("estimates")?;
            o.value(
                id,
                stddev(&estimates).and_then(|s| mean(&estimates).and_then(|m| div(s, m.abs()))),
                "coefficient of variation",
                "估计不足或均值为零",
            );
        }
        "earnings_surprise" => ratio!(
            id,
            x("actual_eps")? - x("consensus_eps")?,
            x("consensus_eps")?.abs(),
            "fraction"
        ),
        "revision" => {
            let up = nonnegative(v, "upward_revisions")?;
            let down = nonnegative(v, "downward_revisions")?;
            let unchanged = nonnegative(v, "unchanged")?;
            ratio!(
                "net_revision_ratio",
                up - down,
                up + down + unchanged,
                "fraction"
            );
        }
        "iv_term_structure" => {
            let terms = a("maturity_years")?;
            let ivs = a("implied_volatilities")?;
            paired(&terms, &ivs)?;
            nonnegative_array(&ivs)?;
            if terms.iter().any(|x| *x <= 0.0) || terms.windows(2).any(|x| x[1] <= x[0]) {
                return Err("期限必须为正且严格递增".into());
            }
            o.series("iv", ivs.iter().copied().map(Some).collect(), "fraction");
            o.series(
                "maturity_years",
                terms.iter().copied().map(Some).collect(),
                "years",
            );
            o.value(
                "end_to_end_slope",
                if terms.len() > 1 {
                    div(
                        ivs[ivs.len() - 1] - ivs[0],
                        terms[terms.len() - 1] - terms[0],
                    )
                } else {
                    None
                },
                "IV/year",
                "至少两个期限",
            );
        }
        "skew" => o.number(
            "put_minus_call_25delta_iv",
            nonnegative(v, "put_25delta_iv")? - nonnegative(v, "call_25delta_iv")?,
            "volatility fraction",
        ),
        "put_call_ratio" => ratio!(
            id,
            nonnegative(v, "put_volume")?,
            nonnegative(v, "call_volume")?,
            "ratio"
        ),
        "max_pain" => {
            let strikes = a("strikes")?;
            let calls = a("call_open_interest")?;
            let puts = a("put_open_interest")?;
            paired(&strikes, &calls)?;
            paired(&calls, &puts)?;
            nonnegative_array(&calls)?;
            nonnegative_array(&puts)?;
            let mult = positive(v, "contract_multiplier")?;
            let payouts: Vec<_> = strikes
                .iter()
                .map(|s| {
                    strikes
                        .iter()
                        .zip(&calls)
                        .zip(&puts)
                        .map(|((k, c), p)| ((s - k).max(0.0) * c + (k - s).max(0.0) * p) * mult)
                        .sum::<f64>()
                })
                .collect();
            let i = (0..payouts.len())
                .min_by(|a, b| payouts[*a].total_cmp(&payouts[*b]))
                .unwrap();
            o.number("max_pain_strike", strikes[i], "price");
            o.series(
                "expiry_payout",
                payouts.into_iter().map(Some).collect(),
                "currency",
            );
            o.note("仅计算给定OI下到期内在价值赔付最小的输入行权价，不预测到期价格。");
        }
        "gex_dex" => {
            let gammas = a("gammas")?;
            let deltas = a("deltas")?;
            let contracts = a("signed_contracts")?;
            paired(&gammas, &deltas)?;
            paired(&deltas, &contracts)?;
            let spot = positive(v, "spot")?;
            let mult = positive(v, "contract_multiplier")?;
            o.number(
                "gex_per_1pct_move",
                gammas
                    .iter()
                    .zip(&contracts)
                    .map(|(g, c)| g * c * mult * spot * spot * 0.01)
                    .sum(),
                "currency / 1% spot move",
            );
            o.number(
                "delta_exposure",
                deltas
                    .iter()
                    .zip(&contracts)
                    .map(|(d, c)| d * c * mult * spot)
                    .sum(),
                "currency",
            );
            o.note("signed_contracts 是已知/假设净持仓符号；公开OI不能确定做市商持仓方向。");
        }
        "bank_nim" => ratio!(
            id,
            x("interest_income")? - x("interest_expense")?,
            nonnegative(v, "average_earning_assets")?,
            "annual fraction"
        ),
        _ => return Err(format!("独立概念没有 evaluator: {id}")),
    }
    Ok(())
}
fn option_value(id: &str, v: &Value, o: &mut Output) -> Result<(), String> {
    let s = positive(v, "spot")?;
    let k = positive(v, "strike")?;
    let t = nonnegative(v, "time_years")?;
    let r = num(v, "rate")?;
    let sigma = nonnegative(v, "volatility")?;
    let call = boolean(v, "is_call")?;
    if t == 0.0 || sigma == 0.0 && id != "iv" {
        o.value(
            id,
            None,
            "Greek",
            "到期或零波动率下常规微分公式不适用，不能报告伪零Greeks",
        );
        return Ok(());
    }
    if id == "iv" {
        let price = nonnegative(v, "option_price")?;
        let discounted = k * (-r * t).exp();
        let lower = if call {
            (s - discounted).max(0.0)
        } else {
            (discounted - s).max(0.0)
        };
        let upper = if call { s } else { discounted };
        if price < lower || price >= upper {
            o.value(
                id,
                None,
                "annual volatility fraction",
                "期权价格违反欧式无套利区间或对应无穷大IV",
            );
            return Ok(());
        }
        let mut lo = 0.0;
        let mut hi = 8.0;
        for _ in 0..100 {
            let mid = (lo + hi) / 2.0;
            if options::bs_price(s, k, t, r, mid, call) > price {
                hi = mid
            } else {
                lo = mid
            }
        }
        o.number(id, (lo + hi) / 2.0, "annual volatility fraction");
    } else {
        let val = match id {
            "delta" => options::delta(s, k, t, r, sigma, call),
            "gamma" => options::gamma(s, k, t, r, sigma),
            "theta" => options::theta(s, k, t, r, sigma, call),
            "vega" => options::vega(s, k, t, r, sigma),
            _ => unreachable!(),
        };
        o.number(
            id,
            val,
            match id {
                "delta" => "option/underlying",
                "gamma" => "delta/price",
                "theta" => "currency/day",
                "vega" => "currency / 1 volatility percentage point",
                _ => "",
            },
        );
    }
    o.note("Black–Scholes 欧式、无分红模型；时间以年计，Theta以日计，Vega每1个百分点。");
    Ok(())
}
fn profile(prices: &[f64], weights: &[f64], fraction: f64, o: &mut Output) -> Result<(), String> {
    paired(prices, weights)?;
    nonnegative_array(weights)?;
    if !(0.0..=1.0).contains(&fraction) || fraction == 0.0 {
        return Err("value_area_fraction 必须在 (0,1]".into());
    }
    if prices.windows(2).any(|x| x[1] <= x[0]) {
        return Err("prices 必须严格递增".into());
    }
    let total = weights.iter().sum::<f64>();
    if total == 0.0 {
        o.value("poc", None, "price", "所有价位权重为零");
        return Ok(());
    }
    let poc = (0..weights.len())
        .max_by(|a, b| weights[*a].total_cmp(&weights[*b]))
        .unwrap();
    let (mut lo, mut hi, mut sum) = (poc, poc, weights[poc]);
    while sum < total * fraction && (lo > 0 || hi + 1 < weights.len()) {
        if hi + 1 < weights.len() && (lo == 0 || weights[hi + 1] >= weights[lo - 1]) {
            hi += 1;
            sum += weights[hi];
        } else {
            lo -= 1;
            sum += weights[lo];
        }
    }
    o.number("poc", prices[poc], "price");
    o.number("value_area_low", prices[lo], "price");
    o.number("value_area_high", prices[hi], "price");
    o.number("included_fraction", sum / total, "fraction");
    o.series(
        "price_levels",
        prices.iter().copied().map(Some).collect(),
        "price",
    );
    o.series(
        "weights",
        weights.iter().copied().map(Some).collect(),
        "input weight",
    );
    Ok(())
}
