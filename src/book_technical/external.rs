use super::*;
pub(super) fn evaluate(id: &str, v: &Value, o: &mut Output) -> Result<(), String> {
    let x = |k| num(v, k);
    let a = |k| arr(v, k);
    macro_rules! ratio {
        ($k:expr,$a:expr,$b:expr,$u:expr) => {
            o.value($k, div($a, $b), $u, "零分母或不足输入，无定义")
        };
    }
    match id {
        "gann_angle" => o.number(
            "line_price",
            x("anchor_price")?
                + nonnegative(v, "bars_elapsed")? * x("price_per_bar")? * x("angle_ratio")?,
            "price",
        ),
        "chart_pattern_validation" => {
            let first = positive(v, "first_extreme")?;
            let second = positive(v, "second_extreme")?;
            let neck = x("neckline")?;
            let current = x("current_price")?;
            let tol = nonnegative(v, "tolerance_fraction")?;
            let similar = (second / first - 1.0).abs() <= tol;
            let top = boolean(v, "is_double_top")?;
            let structure = if top {
                neck < first.min(second)
            } else {
                neck > first.max(second)
            };
            o.flag("similar_extremes", similar);
            o.flag("neckline_structure", structure);
            o.flag(
                "confirmed_break",
                similar && structure && if top { current < neck } else { current > neck },
            );
        }
        "advances_declines" => {
            let cur = a("current_prices")?;
            let prev = a("previous_prices")?;
            pair(&cur, &prev)?;
            let up = cur.iter().zip(&prev).filter(|(a, b)| a > b).count();
            let down = cur.iter().zip(&prev).filter(|(a, b)| a < b).count();
            o.number("advances", up as f64, "issues");
            o.number("declines", down as f64, "issues");
            o.number("unchanged", (cur.len() - up - down) as f64, "issues");
        }
        "advance_decline_ratio" => ratio!(
            id,
            nonnegative(v, "advances")?,
            nonnegative(v, "declines")?,
            "ratio"
        ),
        "mcclellan_sum" => {
            let mut sum = x("initial_value")?;
            o.series(
                id,
                a("oscillator")?
                    .iter()
                    .map(|v| {
                        sum += v;
                        Some(sum)
                    })
                    .collect(),
                "cumulative oscillator",
            );
        }
        "percent_above_ma" => {
            let p = a("prices")?;
            for k in ["ma50", "ma200"] {
                let m = a(k)?;
                pair(&p, &m)?;
                o.number(
                    k,
                    p.iter().zip(m).filter(|(p, m)| **p > *m).count() as f64 / p.len() as f64,
                    "fraction",
                );
            }
        }
        "relative_strength_line" | "mansfield_rs" => {
            let asset = a("asset_prices")?;
            let b = a("benchmark_prices")?;
            pair(&asset, &b)?;
            let rs: Vec<_> = asset.iter().zip(b).map(|(a, b)| div(*a, b)).collect();
            if id == "relative_strength_line" {
                o.series(id, rs, "price ratio");
            } else {
                let m = ma::optional_smooth(&rs, period(v, "period")?, ma::sma);
                o.series(
                    id,
                    rs.iter()
                        .zip(m)
                        .map(|(r, m)| div((*r)?, m?).map(|x| (x - 1.0) * 100.0))
                        .collect(),
                    "percent",
                );
            }
        }
        "rrg_coordinates" => {
            let r = x("rs_ratio")?;
            let m = x("rs_momentum")?;
            o.number("rs_ratio", r, "provided normalized index");
            o.number("rs_momentum", m, "provided normalized index");
            o.number(
                "quadrant",
                if r >= 100.0 && m >= 100.0 {
                    1.0
                } else if r >= 100.0 {
                    2.0
                } else if m < 100.0 {
                    3.0
                } else {
                    4.0
                },
                "1 leading / 2 weakening / 3 lagging / 4 improving",
            );
            o.note("采用用户/供应商提供坐标，不反向捏造RRG专有计算。");
        }
        "pair_spread" => {
            let aa = a("asset_a")?;
            let bb = a("asset_b")?;
            pair(&aa, &bb)?;
            let hedge = x("hedge_ratio")?;
            let spread: Vec<_> = aa.iter().zip(bb).map(|(a, b)| a - hedge * b).collect();
            o.series(
                "spread",
                spread.iter().copied().map(Some).collect(),
                "price",
            );
            o.series(
                "zscore",
                stats::zscore(&spread, period(v, "period")?),
                "standard deviations",
            );
        }
        "cointegration_diagnostic" => {
            let asset_x = a("asset_x")?;
            let asset_y = a("asset_y")?;
            pair(&asset_x, &asset_y)?;
            if asset_x.len() < 4 {
                return Err("至少四组按相同时间对齐的两资产观测".into());
            }
            let n = asset_x.len();
            let mx = mean(&asset_x).unwrap_or(0.0);
            let my = mean(&asset_y).unwrap_or(0.0);
            let sxx = asset_x.iter().map(|z| (z - mx).powi(2)).sum::<f64>();
            let sxy = asset_x
                .iter()
                .zip(&asset_y)
                .map(|(x, y)| (x - mx) * (y - my))
                .sum::<f64>();
            let hedge = div(sxy, sxx);
            let intercept = hedge.map(|b| my - b * mx);
            o.value(
                "hedge_ratio",
                hedge,
                "y units / x unit",
                "自变量方差为零，OLS无定义",
            );
            o.value(
                "intercept",
                intercept,
                "y units",
                "自变量方差为零，OLS无定义",
            );
            let residual: Vec<Option<f64>> = asset_x
                .iter()
                .zip(&asset_y)
                .map(|(x, y)| hedge.zip(intercept).map(|(b, c)| y - c - b * x))
                .collect();
            o.series(
                "in_sample_ols_residual",
                residual.clone(),
                "y units; full-sample fit",
            );
            let fit = residual.into_iter().collect::<Option<Vec<f64>>>();
            let mut beta = None;
            let mut statistic = None;
            if let Some(e) = fit {
                let energy = e.iter().map(|z| z * z).sum::<f64>();
                let scale = asset_y.iter().map(|z| z * z).sum::<f64>().max(1.0);
                // Numerical collinearity is not a statistically estimable residual process.
                if energy > f64::EPSILON * f64::EPSILON * scale * n as f64 {
                    let lag = &e[..n - 1];
                    let delta: Vec<f64> = e.windows(2).map(|w| w[1] - w[0]).collect();
                    let denominator = lag.iter().map(|z| z * z).sum::<f64>();
                    beta = div(
                        lag.iter().zip(&delta).map(|(a, b)| a * b).sum(),
                        denominator,
                    );
                    if let Some(b) = beta {
                        let sse = lag
                            .iter()
                            .zip(&delta)
                            .map(|(a, d)| (d - b * a).powi(2))
                            .sum::<f64>();
                        let variance = sse / (n - 2) as f64;
                        statistic = div(variance, denominator).and_then(|v| div(b, v.sqrt()));
                    }
                }
            }
            o.value(
                "delta_on_lag_slope",
                beta,
                "DF delta coefficient",
                "OLS残差为零或滞后残差没有有效变动",
            );
            o.value(
                "residual_unit_root_t",
                statistic,
                "residual DF t statistic, not Student t",
                "OLS/残差过程退化或回归标准误为零",
            );
            o.number("df_lags", 0.0, "lagged differences; fixed zero");
            o.number("df_observations", (n - 1) as f64, "paired differences");
            o.value(
                "half_life",
                beta.and_then(|b| {
                    let phi = 1.0 + b;
                    if phi > 0.0 && phi < 1.0 {
                        Some(-2f64.ln() / phi.ln())
                    } else {
                        None
                    }
                }),
                "periods",
                "AR系数不在(0,1)，不报告单调均值回复半衰期",
            );
            o.value("cointegration_p_value",None,"probability","未实现两资产、含截距协整回归对应的MacKinnon分布；不能使用普通t分布或单序列ADF的p值");
            o.value(
                "cointegration_critical_value_5pct",
                None,
                "residual DF threshold",
                "临界值依赖资产数、确定项和样本量；未校准，不能按负系数或固定-1.96判断协整",
            );
            o.note("先拟合 y=截距+对冲比例×x，再对残差拟合 Δe_t=γ e_(t-1)，第二步不另加截距/趋势，固定零个差分滞后。这是Engle–Granger的统计量练习，不提供显著性结论；正式应用还需验证两序列为I(1)、选择滞后并检查残差自相关。");
            o.note("输入必须同一时间对齐，不会自动对齐或补缺。残差使用整段样本估计参数，是回顾性诊断；回测必须只用当时训练窗口拟合并冻结参数，不能把这些残差当作过去已知信号。");
            o.note("方法边界参考 https://www.statsmodels.org/stable/generated/statsmodels.tsa.stattools.coint.html 。默认两资产是可编辑教学数据，不是当前交易币种实测数据。");
        }
        "factor_score" => {
            let s = a("standardized_scores")?;
            let w = a("weights")?;
            pair(&s, &w)?;
            o.number(
                id,
                s.iter().zip(w).map(|(s, w)| s * w).sum(),
                "weighted z score",
            );
        }
        "annualized_volatility" => {
            let periods = positive(v, "periods_per_year")?;
            o.value(
                id,
                stddev(&a("returns")?).map(|z| z * periods.sqrt()),
                "annual fraction",
                "至少两个收益观测",
            );
        }
        "r_squared" => {
            let a = a("strategy_returns")?;
            let b = arr(v, "benchmark_returns")?;
            pair(&a, &b)?;
            o.value(
                id,
                stats::correlation(&a, &b).map(|r| r * r),
                "fraction",
                "至少两组且非零方差",
            );
        }
        "option_value_components" => {
            let s = positive(v, "spot")?;
            let k = positive(v, "strike")?;
            let p = nonnegative(v, "premium")?;
            let intrinsic = if boolean(v, "is_call")? {
                (s - k).max(0.0)
            } else {
                (k - s).max(0.0)
            };
            o.number("intrinsic_value", intrinsic, "currency");
            o.number("premium_minus_intrinsic", p - intrinsic, "currency");
            o.note("负剩余价值需要考虑报价质量、分红、欧式提前行权限制与折现，不能机械裁为0。");
        }
        "option_moneyness" => {
            let s = positive(v, "spot")?;
            let k = positive(v, "strike")?;
            let gap = s / k - 1.0;
            let tol = nonnegative(v, "atm_tolerance_fraction")?;
            let sign = if boolean(v, "is_call")? { 1.0 } else { -1.0 };
            o.number("signed_moneyness", gap * sign, "fraction");
            o.number(
                "classification",
                if gap.abs() <= tol {
                    0.0
                } else if gap * sign > 0.0 {
                    1.0
                } else {
                    -1.0
                },
                "1 ITM / 0 ATM / -1 OTM",
            );
        }
        "option_dte" => o.number(
            "dte",
            ((x("expiry_seconds")? - x("evaluation_seconds")?) / 86400.0).max(0.0),
            "calendar days",
        ),
        "option_volume_oi" => {
            let oi = nonnegative(v, "open_interest")?;
            let prev = nonnegative(v, "previous_open_interest")?;
            o.number("oi_change", oi - prev, "contracts");
            o.number("volume", nonnegative(v, "volume")?, "contracts");
            ratio!("volume_to_oi", x("volume")?, oi, "ratio");
        }
        "iv_percentile" => {
            let hist = a("historical_iv")?;
            let current = nonnegative(v, "current_iv")?;
            if hist.iter().any(|x| *x < 0.0) {
                return Err("IV不得负".into());
            }
            ratio!(
                id,
                hist.iter().filter(|x| **x < current).count() as f64 * 100.0,
                hist.len() as f64,
                "percent"
            );
        }
        "iv_smile" => {
            let strikes = a("strikes")?;
            let iv = a("ivs")?;
            pair(&strikes, &iv)?;
            let spot = positive(v, "spot")?;
            let at = (0..strikes.len())
                .min_by(|a, b| {
                    (strikes[*a] - spot)
                        .abs()
                        .total_cmp(&(strikes[*b] - spot).abs())
                })
                .unwrap();
            o.number("atm_iv", iv[at], "annual fraction");
            o.number("left_wing_minus_atm", iv[0] - iv[at], "fraction");
            o.number(
                "right_wing_minus_atm",
                iv[iv.len() - 1] - iv[at],
                "fraction",
            );
            o.series(
                "smile",
                iv.into_iter().map(Some).collect(),
                "annual fraction",
            );
        }
        "implied_move" => {
            let s = positive(v, "spot")?;
            let move_ = s * nonnegative(v, "iv")? * nonnegative(v, "time_years")?.sqrt();
            o.number("one_sigma_move", move_, "price");
            o.number("linear_lower", s - move_, "price");
            o.number("linear_upper", s + move_, "price");
        }
        "rho" | "second_order_greeks" => {
            let s = positive(v, "spot")?;
            let k = positive(v, "strike")?;
            let t = nonnegative(v, "time_years")?;
            let rate = x("rate")?;
            let sigma = nonnegative(v, "volatility")?;
            let call = boolean(v, "is_call")?;
            if t == 0.0 || sigma == 0.0 {
                o.value(id, None, "Greek", "到期或零波动率时常规导数不适用");
            } else if id == "rho" {
                o.number(
                    id,
                    options::rho(s, k, t, rate, sigma, call),
                    "currency / 1 rate percentage point",
                );
            } else {
                let ds = (sigma * 0.001).min(0.0001);
                let dt = (t * 0.001).min(1.0 / 36500.0);
                let vanna = (options::delta(s, k, t, rate, sigma + ds, call)
                    - options::delta(s, k, t, rate, sigma - ds, call))
                    / (2.0 * ds);
                let charm = (options::delta(s, k, t - dt, rate, sigma, call)
                    - options::delta(s, k, t + dt, rate, sigma, call))
                    / (2.0 * dt * 365.0);
                let vomma = (options::vega(s, k, t, rate, sigma + ds)
                    - options::vega(s, k, t, rate, sigma - ds))
                    / (2.0 * ds)
                    * 100.0;
                o.number("vanna", vanna, "delta / 1.0 volatility");
                o.number("charm_per_day", charm, "delta / calendar day");
                o.number("vomma", vomma, "currency / volatility squared");
                o.note("中央有限差分，Black-Scholes无分红欧式模型；显示原始σ单位而非百分点。");
            }
        }
        "news_sentiment" => {
            let s = a("scores")?;
            let w = a("weights")?;
            pair(&s, &w)?;
            if s.iter().any(|x| x.abs() > 1.0) || w.iter().any(|x| *x < 0.0) {
                return Err("情绪分数须[-1,1]且权重非负".into());
            }
            ratio!(
                "weighted_sentiment",
                s.iter().zip(&w).map(|(s, w)| s * w).sum::<f64>(),
                w.iter().sum::<f64>(),
                "score -1..1"
            );
            o.number("sample_count", s.len() as f64, "count");
        }
        "social_attention" => {
            let hist = a("historical_mentions")?;
            let current = nonnegative(v, "current_mentions")?;
            o.value(
                "attention_ratio",
                mean(&hist).and_then(|m| div(current, m)),
                "multiple",
                "历史平均次数为零",
            );
            o.value(
                "attention_zscore",
                mean(&hist).and_then(|m| stddev(&hist).and_then(|s| div(current - m, s))),
                "standard deviations",
                "样本不足或方差为零",
            );
        }
        "pitfall_order_imbalance" => {
            let b = nonnegative(v, "bid_size")?;
            let a = nonnegative(v, "ask_size")?;
            let cancelled = nonnegative(v, "cancelled_bid_size")?;
            if cancelled > b {
                return Err("撤单量不能超过原买盘".into());
            }
            ratio!("before", b - a, b + a, "fraction");
            ratio!(
                "after_cancellation",
                b - cancelled - a,
                b - cancelled + a,
                "fraction"
            );
        }
        "pitfall_timeframe" => {
            o.flag(
                "same_interval",
                positive(v, "first_interval_seconds")? == positive(v, "second_interval_seconds")?,
            );
            o.flag(
                "same_asof",
                x("first_asof_seconds")? == x("second_asof_seconds")?,
            );
        }
        "pitfall_open_candle" => {
            let p = positive(v, "previous_close")?;
            o.number(
                "provisional_return",
                x("provisional_close")? / p - 1.0,
                "fraction",
            );
            o.number("final_return", x("final_close")? / p - 1.0, "fraction");
            o.flag(
                "eligible_for_close_based_strategy",
                boolean(v, "is_closed")?,
            );
        }
        "pitfall_adjustment" => {
            let p = positive(v, "previous_price")?;
            let c = positive(v, "current_price")?;
            o.number("unadjusted_return", c / p - 1.0, "fraction");
            o.number(
                "economic_return",
                (c * positive(v, "new_shares_per_old_share")? + x("cash_dividend_per_old_share")?)
                    / p
                    - 1.0,
                "fraction",
            );
        }
        "pitfall_formula_variant" => {
            let a = a("provider_a")?;
            let b = arr(v, "provider_b")?;
            pair(&a, &b)?;
            o.series(
                "provider_difference",
                a.iter().zip(b).map(|(a, b)| Some(b - a)).collect(),
                "provider units",
            );
        }
        "backtest_data_audit" => {
            let obs = a("observation_times")?;
            let avail = a("available_times")?;
            let decisions = a("decision_times")?;
            pair(&obs, &avail)?;
            pair(&avail, &decisions)?;
            o.number(
                "future_information_violations",
                avail
                    .iter()
                    .zip(decisions)
                    .filter(|(a, d)| **a > *d)
                    .count() as f64,
                "count",
            );
            o.flag("includes_delisted", boolean(v, "includes_delisted")?);
            o.flag(
                "positive_execution_lag",
                nonnegative(v, "execution_lag_bars")? > 0.0,
            );
            o.flag("positive_costs", nonnegative(v, "fee_fraction")? > 0.0);
        }
        "pitchfork" => {
            let x = a("x")?;
            let y = a("y")?;
            if x.len() != 3 || y.len() != 3 || x[1] <= x[0] || x[2] <= x[1] {
                return Err("音叉需要按时间递增的三个锚点".into());
            }
            let mid_x = (x[1] + x[2]) / 2.0;
            let mid_y = (y[1] + y[2]) / 2.0;
            let slope = (mid_y - y[0]) / (mid_x - x[0]);
            let eval = num(v, "evaluate_x")?;
            o.number("median", y[0] + slope * (eval - x[0]), "price");
            o.number("parallel_b", y[1] + slope * (eval - x[1]), "price");
            o.number("parallel_c", y[2] + slope * (eval - x[2]), "price");
        }
        "cumulative_volume_index" | "net_volume" => {
            let (a_key, b_key) = if id == "cumulative_volume_index" {
                ("up_volume", "down_volume")
            } else {
                ("uptick_volume", "downtick_volume")
            };
            let up = a(a_key)?;
            let down = a(b_key)?;
            pair(&up, &down)?;
            if up.iter().chain(&down).any(|x| *x < 0.0) {
                return Err("成交量不能负".into());
            }
            let mut total = 0.0;
            o.series(
                id,
                up.iter()
                    .zip(down)
                    .map(|(u, d)| {
                        let delta = u - d;
                        total += delta;
                        Some(if id == "cumulative_volume_index" {
                            total
                        } else {
                            delta
                        })
                    })
                    .collect(),
                "volume",
            );
        }
        "rob_booker_adx" | "knoxville" | "missed_pivots" | "booker_reversal" | "ghost_pivots" => {
            let signal = a("signal_indices")?;
            let confirm = a("confirmed_indices")?;
            let entry = a("entry_indices")?;
            let prices = a("entry_prices")?;
            let exits = a("exit_prices")?;
            let sides = a("sides")?;
            for array in [&confirm, &entry, &prices, &exits, &sides] {
                pair(&signal, array)?;
            }
            if sides.iter().any(|x| *x != 1.0 && *x != -1.0) || prices.iter().any(|x| *x <= 0.0) {
                return Err("sides必须+1/-1，入场价格须为正".into());
            }
            let cost = nonnegative(v, "cost_fraction")?;
            let mut violations = 0;
            let mut returns = Vec::new();
            for i in 0..signal.len() {
                if confirm[i] < signal[i] || entry[i] < confirm[i] {
                    violations += 1;
                    returns.push(None);
                } else {
                    returns.push(Some((exits[i] / prices[i] - 1.0) * sides[i] - cost));
                }
            }
            o.number("timing_violations", violations as f64, "count");
            o.series(
                "valid_imported_signal_net_returns",
                returns,
                "fraction / entry notional",
            );
            let checked_array = |key: &str| -> Result<Vec<f64>, String> {
                let data = arr(v, key)?;
                pair(&signal, &data)?;
                Ok(data)
            };
            let evidence: Vec<Option<f64>> = match id {
                "rob_booker_adx" => {
                    let before = checked_array("adx_before")?;
                    let now = checked_array("adx_now")?;
                    let breakout = checked_array("price_breakout_flags")?;
                    let low = x("compression_threshold")?;
                    let high = x("trend_threshold")?;
                    (0..signal.len())
                        .map(|i| {
                            Some(if before[i] < low && now[i] >= high && breakout[i] == 1.0 {
                                1.0
                            } else {
                                0.0
                            })
                        })
                        .collect()
                }
                "knoxville" => {
                    let price = checked_array("price_extreme_changes")?;
                    let momentum = checked_array("momentum_extreme_changes")?;
                    let rsi = checked_array("rsi_at_signal")?;
                    (0..signal.len())
                        .map(|i| {
                            Some(
                                if sides[i] < 0.0
                                    && price[i] > 0.0
                                    && momentum[i] < 0.0
                                    && rsi[i] >= 70.0
                                    || sides[i] > 0.0
                                        && price[i] < 0.0
                                        && momentum[i] > 0.0
                                        && rsi[i] <= 30.0
                                {
                                    1.0
                                } else {
                                    0.0
                                },
                            )
                        })
                        .collect()
                }
                "missed_pivots" => {
                    let levels = checked_array("pivot_prices")?;
                    let touches = checked_array("prior_touch_counts")?;
                    (0..signal.len())
                        .map(|i| {
                            Some(if levels[i] > 0.0 && touches[i] == 0.0 {
                                1.0
                            } else {
                                0.0
                            })
                        })
                        .collect()
                }
                "booker_reversal" => {
                    let before = checked_array("prior_trend_signs")?;
                    let now = checked_array("current_trend_signs")?;
                    let m0 = checked_array("prior_momentum")?;
                    let m1 = checked_array("current_momentum")?;
                    (0..signal.len())
                        .map(|i| {
                            Some(
                                if before[i] == -sides[i]
                                    && now[i] == sides[i]
                                    && m0[i] * sides[i] < 0.0
                                    && m1[i] * sides[i] > 0.0
                                {
                                    1.0
                                } else {
                                    0.0
                                },
                            )
                        })
                        .collect()
                }
                "ghost_pivots" => {
                    let levels = checked_array("provided_levels")?;
                    let available = checked_array("level_available_indices")?;
                    let lo = checked_array("low_since_available")?;
                    let hi = checked_array("high_since_available")?;
                    (0..signal.len())
                        .map(|i| {
                            Some(
                                if available[i] <= entry[i]
                                    && lo[i] <= hi[i]
                                    && (levels[i] < lo[i] || levels[i] > hi[i])
                                {
                                    1.0
                                } else {
                                    0.0
                                },
                            )
                        })
                        .collect()
                }
                _ => unreachable!(),
            };
            o.series(
                "declared_concept_evidence",
                evidence,
                "boolean 1/0 under stated teaching rule",
            );
            o.note("这里只核验用户导入信号与明示条件，不产生平台原版自动信号；不能将本练习的示例阈值冒称Rob Booker专有公式。空头收益按入场名义金额归一，不是融资账户收益率。");
        }
        "seasonality" => {
            let r = a("same_season_returns")?;
            o.value("mean_return", mean(&r), "fraction", "无历史样本");
            o.value("sample_std", stddev(&r), "fraction", "样本不足");
            o.number("sample_count", r.len() as f64, "count");
        }
        "technical_ratings" => {
            let a = a("ma_votes")?;
            let b = arr(v, "oscillator_votes")?;
            if a.iter().chain(&b).any(|x| ![-1.0, 0.0, 1.0].contains(x)) {
                return Err("投票只能-1,0,1".into());
            }
            o.value(
                "combined_rating",
                mean(&a).zip(mean(&b)).map(|(a, b)| (a + b) / 2.0),
                "score -1..1",
                "各组至少一个投票",
            );
        }
        "moon_phases" => {
            o.number(
                "days_since_provided_new_moon",
                (x("current_seconds")? - x("new_moon_seconds")?) / 86400.0,
                "days",
            );
            o.number(
                "days_until_provided_full_moon",
                (x("full_moon_seconds")? - x("current_seconds")?) / 86400.0,
                "days",
            );
            o.note("输入事件时刻是教学坐标，实际天文事件必须从星历输入；此处不预测金融收益。");
        }
        _ => return Err(format!("没有独立技术 evaluator: {id}")),
    }
    Ok(())
}
