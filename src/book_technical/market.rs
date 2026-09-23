use super::*;
use chrono::Timelike;
pub(super) fn evaluate(
    id: &str,
    b: &[Bar],
    v: &Value,
    o: &mut Output,
    annualization: Option<AnnualizationBasis>,
) -> Result<(), String> {
    let c: Vec<_> = b.iter().map(|x| x.close).collect();
    let n = b.len();
    let per = |k| period(v, k);
    let x = |k| num(v, k);
    let p = if v.get("period").is_some() {
        per("period")?
    } else {
        14
    };
    macro_rules! ser {
        ($k:expr,$z:expr,$u:expr) => {
            o.series($k, $z, $u)
        };
    }
    match id {
        "relative_volume_at_time" => {
            let end = b.last().ok_or("需要已收盘1小时K线")?;
            let cutoff = end.timestamp.hour();
            let day = end.timestamp.date_naive();
            let mut sums = std::collections::BTreeMap::<chrono::NaiveDate, f64>::new();
            for bar in b
                .iter()
                .filter(|bar| bar.timestamp.date_naive() <= day && bar.timestamp.hour() <= cutoff)
            {
                *sums.entry(bar.timestamp.date_naive()).or_default() += bar.volume;
            }
            let current = sums.remove(&day).unwrap_or(0.0);
            let history: Vec<_> = sums.into_iter().filter(|(date, _)| *date < day).collect();
            if history.len() < 7 {
                o.value(id, None, "ratio", "需要过去7个完整UTC日的同截止小时累计量");
                return Ok(());
            }
            let samples = &history[history.len() - 7..];
            let average = samples.iter().map(|(_, sum)| sum).sum::<f64>() / samples.len() as f64;
            let ratio = div(current, average);
            o.value(id, ratio, "ratio", "历史同期累计量均值为零");
            if ratio.is_some() {
                o.number("current_cumulative_volume", current, "volume");
                o.number("historical_sample_count", samples.len() as f64, "days");
            }
            let dates = samples
                .iter()
                .map(|(date, _)| date.to_string())
                .collect::<Vec<_>>()
                .join(",");
            o.note(&format!("Binance 1小时已收盘K线；累计窗口UTC {} 00:00至{}（含最后一根 {:02}:00 开盘小时K线）；当前累计量 ÷ 历史完整日 [{}] 同窗口累计量均值（7日）。", day, end.timestamp + chrono::Duration::hours(1), cutoff, dates));
        }
        "annualized_volatility" => {
            let Some(basis) = annualization else {
                o.value(id, None, "annual fraction", "需要已验证市场来源的年化口径");
                return Ok(());
            };
            let (periods_per_year, label): (f64, &str) = match basis {
                AnnualizationBasis::CryptoHourly => {
                    (8760.0, "加密市场连续1小时K线，按365×24=8760小时/年")
                }
                AnnualizationBasis::EquityDaily => {
                    (252.0, "交易所日线，按252个交易日/年的年化惯例")
                }
            };
            let returns: Vec<_> = b
                .windows(2)
                .map(|pair| pair[1].close / pair[0].close - 1.0)
                .collect();
            let annualized = stddev(&returns).map(|value| value * periods_per_year.sqrt());
            o.value(
                id,
                annualized,
                "annual fraction",
                "至少需要三根已收盘K线（两项相邻收盘价简单收益率）",
            );
            if annualized.is_some() {
                o.number("periods_per_year", periods_per_year, "periods per year");
            }
            o.note(&format!(
                "{label}；收益率定义为相邻收盘价简单收益率 C_t/C_(t-1)-1，使用样本标准差。"
            ));
        }
        "cdp" => {
            let Some(previous) = n.checked_sub(2).and_then(|index| b.get(index)) else {
                for key in ["cdp", "ah", "al", "nh", "nl"] {
                    o.value(key, None, "price", "需要上一根已收盘日线的高低收");
                }
                return Ok(());
            };
            if b.windows(2)
                .any(|pair| pair[1].timestamp - pair[0].timestamp < chrono::Duration::hours(20))
            {
                return Err("CDP 需要日线；小时K线不是上一交易时段".into());
            }
            let p = (previous.high + previous.low + 2.0 * previous.close) / 4.0;
            for (key, value) in [
                ("cdp", p),
                ("ah", p + previous.high - previous.low),
                ("al", p - previous.high + previous.low),
                ("nh", 2.0 * p - previous.low),
                ("nl", 2.0 * p - previous.high),
            ] {
                o.number(key, value, "price");
            }
            o.note(&format!(
                "取倒数第二根已收盘A股日线的高、低、收；数值适用于最后一根已收盘日线所属交易时段（{}），不把它称为当前自然日。",
                b[n - 1].timestamp.to_rfc3339(),
            ));
        }
        "tema" => ser!(id, ma::tema(&c, p), "price"),
        "kama" => {
            let fast = per("fast")?;
            let slow = per("slow")?;
            if fast >= slow {
                return Err("KAMA fast须小于slow".into());
            }
            let mut out = vec![None; n];
            if n >= p {
                let mut prev = c[..p].iter().sum::<f64>() / p as f64;
                out[p - 1] = Some(prev);
                for i in p..n {
                    let movement = c[i + 1 - p..=i]
                        .iter()
                        .zip(c[i - p..i].iter())
                        .map(|(a, b)| (a - b).abs())
                        .sum::<f64>();
                    let er = if movement == 0.0 {
                        0.0
                    } else {
                        (c[i] - c[i - p]).abs() / movement
                    };
                    let sc = (er * (2.0 / (fast + 1) as f64 - 2.0 / (slow + 1) as f64)
                        + 2.0 / (slow + 1) as f64)
                        .powi(2);
                    prev += sc * (c[i] - prev);
                    out[i] = Some(prev);
                }
            }
            ser!(id, out, "price");
        }
        "alma" => {
            let offset = x("offset")?;
            if !(0.0..=1.0).contains(&offset) {
                return Err("ALMA offset须0..1".into());
            }
            let sigma = positive(v, "sigma")?;
            let m = offset * (p - 1) as f64;
            let s = p as f64 / sigma;
            let w: Vec<_> = (0..p)
                .map(|i| (-((i as f64 - m).powi(2)) / (2.0 * s * s)).exp())
                .collect();
            let total = w.iter().sum::<f64>();
            ser!(
                id,
                rolling(&c, p, |a| div(
                    a.iter().zip(&w).map(|(a, w)| a * w).sum(),
                    total
                )),
                "price"
            );
        }
        "gmma" => {
            for k in ["short_periods", "long_periods"] {
                let ps = periods(v, k)?;
                if ps.is_empty() {
                    return Err("GMMA周期组不能为空".into());
                }
                for p in ps {
                    ser!(&format!("{k}_{p}"), ma::ema(&c, p), "price");
                }
            }
        }
        "macd_zero" | "macd_cross" | "macd_histogram" | "macd_divergence" | "macd_regime" => {
            let m = trend::macd(&c, per("fast")?, per("slow")?, per("signal")?);
            match id {
                "macd_zero" => {
                    ser!("dif", m.dif.clone(), "price");
                    ser!(
                        "above_zero",
                        m.dif
                            .into_iter()
                            .map(|x| x.map(|z| if z > 0.0 {
                                1.0
                            } else if z < 0.0 {
                                -1.0
                            } else {
                                0.0
                            }))
                            .collect(),
                        "sign"
                    );
                }
                "macd_histogram" => ser!("dif_minus_dea", m.hist, "price"),
                "macd_cross" => {
                    let mut out = vec![None; n];
                    for (i, value) in out.iter_mut().enumerate().skip(1) {
                        *value = match (m.hist[i - 1], m.hist[i]) {
                            (Some(a), Some(b)) => Some(if a <= 0.0 && b > 0.0 {
                                1.0
                            } else if a >= 0.0 && b < 0.0 {
                                -1.0
                            } else {
                                0.0
                            }),
                            _ => None,
                        };
                    }
                    ser!("cross", out, "+1 bullish / -1 bearish / 0 none");
                }
                "macd_regime" => {
                    ser!("dif", m.dif, "price");
                    ser!("adx", trend::dmi(b, p).adx, "index 0..100");
                }
                "macd_divergence" => divergence(b, &m.dif, per("right_bars")?, o),
                _ => unreachable!(),
            };
        }
        "vortex" => {
            let r = trend::vortex(b, p);
            ser!("vi_plus", r.plus, "ratio");
            ser!("vi_minus", r.minus, "ratio");
        }
        "chandelier_exit" => {
            let atr = vol::atr(b, p);
            let m = positive(v, "multiplier")?;
            let mut long = vec![None; n];
            let mut short = vec![None; n];
            for i in p.saturating_sub(1)..n {
                if let Some(a) = atr[i] {
                    long[i] = Some(
                        b[i + 1 - p..=i]
                            .iter()
                            .map(|b| b.high)
                            .fold(f64::NEG_INFINITY, f64::max)
                            - m * a,
                    );
                    short[i] = Some(
                        b[i + 1 - p..=i]
                            .iter()
                            .map(|b| b.low)
                            .fold(f64::INFINITY, f64::min)
                            + m * a,
                    );
                }
            }
            ser!("long_exit", long, "price");
            ser!("short_exit", short, "price");
        }
        "ma_envelope" => {
            let f = nonnegative(v, "fraction")?;
            let ma = ma::sma(&c, p);
            ser!(
                "upper",
                ma.iter().map(|x| x.map(|x| x * (1.0 + f))).collect(),
                "price"
            );
            ser!(
                "lower",
                ma.iter().map(|x| x.map(|x| x * (1.0 - f))).collect(),
                "price"
            );
        }
        "regression_channel" => {
            let mult = nonnegative(v, "multiplier")?;
            let mut center = vec![None; n];
            let mut upper = center.clone();
            let mut lower = center.clone();
            for i in p.saturating_sub(1)..n {
                let w = &c[i + 1 - p..=i];
                if let Some((s, k)) = stats::linear_regression(w) {
                    let y = s * (p - 1) as f64 + k;
                    let rms = (w
                        .iter()
                        .enumerate()
                        .map(|(j, z)| (z - (s * j as f64 + k)).powi(2))
                        .sum::<f64>()
                        / p as f64)
                        .sqrt();
                    center[i] = Some(y);
                    upper[i] = Some(y + mult * rms);
                    lower[i] = Some(y - mult * rms);
                }
            }
            ser!("lsma", center, "price");
            ser!("upper", upper, "price");
            ser!("lower", lower, "price");
        }
        "ppo_apo" => {
            let f = ma::ema(&c, per("fast")?);
            let s = ma::ema(&c, per("slow")?);
            ser!(
                "apo",
                f.iter().zip(&s).map(|(f, s)| Some((*f)? - (*s)?)).collect(),
                "price"
            );
            ser!(
                "ppo",
                f.iter()
                    .zip(&s)
                    .map(|(f, s)| div(((*f)? - (*s)?) * 100.0, (*s)?))
                    .collect(),
                "percent"
            );
        }
        "dma" => {
            let f = ma::sma(&c, per("fast")?);
            let s = ma::sma(&c, per("slow")?);
            let d: Vec<_> = f.iter().zip(s).map(|(f, s)| Some((*f)? - s?)).collect();
            ser!(
                "signal",
                ma::optional_smooth(&d, per("signal")?, ma::sma),
                "price"
            );
            ser!("dma", d, "price");
        }
        "trix" => {
            let e1 = ma::ema(&c, p);
            let e2 = smooth(&e1, p);
            let e3 = smooth(&e2, p);
            let mut r = vec![None; n];
            for i in 1..n {
                r[i] = e3[i]
                    .zip(e3[i - 1])
                    .and_then(|(a, b)| div((a - b) * 100.0, b));
            }
            ser!("signal", smooth(&r, per("signal")?), "percent");
            ser!("trix", r, "percent");
        }
        "ichimoku_chikou" => {
            let displacement = per("displacement")?;
            o.number("known_at_bar", (n - 1) as f64, "index");
            o.value(
                "visual_plot_index",
                (n > displacement).then_some((n as f64) - 1.0 - displacement as f64),
                "index",
                "历史长度小于位移",
            );
            o.number("current_close", c[n - 1], "price");
            o.note("不输出回填历史价格序列；显示位置并不改变信息可得时点。");
        }
        "accelerator" => {
            let ao = mom::awesome_oscillator(b, per("fast")?, per("slow")?);
            let sm = ma::optional_smooth(&ao, per("signal")?, ma::sma);
            ser!(
                id,
                ao.iter().zip(sm).map(|(a, b)| Some((*a)? - b?)).collect(),
                "price"
            );
        }
        "osc" => {
            let ma = ma::sma(&c, p);
            ser!(
                id,
                c.iter().zip(ma).map(|(c, m)| Some(c - m?)).collect(),
                "price"
            );
        }
        "true_range" => ser!(
            id,
            vol::true_range(b).into_iter().map(Some).collect(),
            "price"
        ),
        "adr" => ser!(id, vol::adr(b, p), "price"),
        "ohlc_volatility" => ohlc_vol(b, p, positive(v, "periods_per_year")?, o)?,
        "volume_price_quadrant" => {
            let mut out = vec![None; n];
            for i in 1..n {
                out[i] = Some(match (c[i] >= c[i - 1], b[i].volume >= b[i - 1].volume) {
                    (true, true) => 1.0,
                    (true, false) => 2.0,
                    (false, true) => 3.0,
                    (false, false) => 4.0,
                });
            }
            ser!(id, out, "1 up/up;2 up/down;3 down/up;4 down/down");
        }
        "emv" => ser!(
            id,
            ma::optional_smooth(&volume::emv(b), p, ma::sma),
            "price squared / volume"
        ),
        "pvi" => ser!(id, volume::pvi(b), "index base 1000"),
        "aroon_oscillator" => {
            let r = trend::aroon(b, p);
            ser!(
                id,
                r.up.iter()
                    .zip(r.down)
                    .map(|(a, b)| Some((*a)? - b?))
                    .collect(),
                "index -100..100"
            );
        }
        "auto_key_levels" | "pivot_high_low" => {
            let r = per("right_bars")?;
            let piv = confirmed_pivots(b, r);
            let mut highs = vec![None; n];
            let mut lows = vec![None; n];
            for (when, pivot, sign) in piv {
                if sign > 0 {
                    highs[when] = Some(b[pivot].high);
                } else {
                    lows[when] = Some(b[pivot].low);
                }
            }
            ser!("confirmed_high", highs, "price at confirmation");
            ser!("confirmed_low", lows, "price at confirmation");
        }
        "bbtrend" => {
            let a = vol::bollinger_bands(&c, per("short")?, positive(v, "multiplier")?);
            let z = vol::bollinger_bands(&c, per("long")?, positive(v, "multiplier")?);
            ser!(
                id,
                (0..n)
                    .map(|i| div(
                        ((a.lower[i]? - z.lower[i]?).abs() - (a.upper[i]? - z.upper[i]?).abs())
                            * 100.0,
                        a.middle[i]?
                    ))
                    .collect(),
                "percent"
            );
        }
        "bollinger_bars" => {
            let b = &b[n - 1];
            o.number("body", (b.close - b.open).abs(), "price");
            o.number("upper_shadow", b.high - b.open.max(b.close), "price");
            o.number("lower_shadow", b.open.min(b.close) - b.low, "price");
            o.number("range", b.high - b.low, "price");
            o.number("direction", (b.close - b.open).signum(), "sign");
            o.note("固定像素宽度属于图形渲染参数；没有把Bollinger Bars误当Bollinger Bands。");
        }
        "chande_kroll" => {
            let q = per("stop_period")?;
            let atr = vol::atr(b, p);
            let m = positive(v, "multiplier")?;
            let mut l = vec![None; n];
            let mut s = l.clone();
            for i in p - 1..n {
                if let Some(a) = atr[i] {
                    l[i] = Some(
                        b[i + 1 - p..=i]
                            .iter()
                            .map(|b| b.high)
                            .fold(f64::NEG_INFINITY, f64::max)
                            - a * m,
                    );
                    s[i] = Some(
                        b[i + 1 - p..=i]
                            .iter()
                            .map(|b| b.low)
                            .fold(f64::INFINITY, f64::min)
                            + a * m,
                    );
                }
            }
            let mut ll = vec![None; n];
            let mut ss = ll.clone();
            for i in q - 1..n {
                let a: Option<Vec<_>> = l[i + 1 - q..=i].iter().copied().collect();
                let z: Option<Vec<_>> = s[i + 1 - q..=i].iter().copied().collect();
                ll[i] = a.map(|v| v.into_iter().fold(f64::NEG_INFINITY, f64::max));
                ss[i] = z.map(|v| v.into_iter().fold(f64::INFINITY, f64::min));
            }
            ser!("long_stop", ll, "price");
            ser!("short_stop", ss, "price");
        }
        "chop_zone" => {
            let e = ma::ema(&c, p);
            let rp = per("range_period")?;
            let mut r = vec![None; n];
            for i in rp.max(1)..n {
                if let (Some(a), Some(z)) = (e[i], e[i - 1]) {
                    let hi = b[i + 1 - rp..=i]
                        .iter()
                        .map(|x| x.high)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let lo = b[i + 1 - rp..=i]
                        .iter()
                        .map(|x| x.low)
                        .fold(f64::INFINITY, f64::min);
                    r[i] = div(a - z, hi - lo).map(|x| x.atan().to_degrees());
                }
            }
            ser!("normalized_ema_angle", r, "degrees under stated scaling");
            o.note("本练习固定范围归一化尺度；不宣称匹配TradingView默认九色阈值。");
        }
        "choppiness" => {
            if p < 2 {
                return Err("CHOP周期至少2".into());
            }
            let tr = vol::true_range(b);
            let mut r = vec![None; n];
            for i in p - 1..n {
                let hi = b[i + 1 - p..=i]
                    .iter()
                    .map(|x| x.high)
                    .fold(f64::NEG_INFINITY, f64::max);
                let lo = b[i + 1 - p..=i]
                    .iter()
                    .map(|x| x.low)
                    .fold(f64::INFINITY, f64::min);
                r[i] = div(tr[i + 1 - p..=i].iter().sum(), hi - lo)
                    .filter(|x| *x > 0.0)
                    .map(|x| 100.0 * x.log10() / (p as f64).log10());
            }
            ser!(id, r, "index");
        }
        "connors_rsi" => {
            let rsi = mom::rsi(&c, per("price_period")?);
            let mut streak = vec![0.0; n];
            let mut roc = vec![None; n];
            for i in 1..n {
                streak[i] = if c[i] > c[i - 1] {
                    if streak[i - 1] > 0.0 {
                        streak[i - 1] + 1.0
                    } else {
                        1.0
                    }
                } else if c[i] < c[i - 1] {
                    if streak[i - 1] < 0.0 {
                        streak[i - 1] - 1.0
                    } else {
                        -1.0
                    }
                } else {
                    0.0
                };
                roc[i] = Some(c[i] / c[i - 1] - 1.0);
            }
            let sr = mom::rsi(&streak, per("streak_period")?);
            let rp = per("rank_period")?;
            let mut out = vec![None; n];
            for i in rp + 1..n {
                if let (Some(a), Some(z), Some(cur)) = (rsi[i], sr[i], roc[i]) {
                    let rank = roc[i - rp..i]
                        .iter()
                        .filter(|x| x.is_some_and(|x| x < cur))
                        .count() as f64
                        / rp as f64
                        * 100.0;
                    out[i] = Some((a + z + rank) / 3.0);
                }
            }
            ser!(id, out, "index 0..100");
        }
        "mcginley" => {
            let k = positive(v, "k")?;
            let mut out = vec![None; n];
            let mut prev = c[0];
            out[0] = Some(prev);
            for i in 1..n {
                let denominator = k * p as f64 * (c[i] / prev).powi(4);
                if denominator <= 0.0 || !denominator.is_finite() {
                    continue;
                }
                prev += (c[i] - prev) / denominator;
                out[i] = Some(prev);
            }
            ser!(id, out, "price");
        }
        "rolling_median" => ser!(
            id,
            rolling(&c, p, |w| {
                let mut x = w.to_vec();
                x.sort_by(f64::total_cmp);
                Some(if x.len() % 2 == 1 {
                    x[x.len() / 2]
                } else {
                    (x[x.len() / 2 - 1] + x[x.len() / 2]) / 2.0
                })
            }),
            "price"
        ),
        "multi_timeframe" => {
            let g = per("bars_per_group")?;
            let mut close = vec![None; n];
            let mut high = close.clone();
            let mut low = close.clone();
            for end in (g - 1..n).step_by(g) {
                let group = &b[end + 1 - g..=end];
                close[end] = Some(b[end].close);
                high[end] = Some(
                    group
                        .iter()
                        .map(|x| x.high)
                        .fold(f64::NEG_INFINITY, f64::max),
                );
                low[end] = Some(group.iter().map(|x| x.low).fold(f64::INFINITY, f64::min));
            }
            ser!("completed_group_close", close, "price");
            ser!("completed_group_high", high, "price");
            ser!("completed_group_low", low, "price");
            o.note("从输入首根对齐分组，仅演示合成；实际交易时段/时区需调用方先对齐。");
        }
        "pvo" => {
            let volume: Vec<_> = b.iter().map(|b| b.volume).collect();
            let f = ma::ema(&volume, per("fast")?);
            let s = ma::ema(&volume, per("slow")?);
            let r: Vec<_> = f
                .iter()
                .zip(s)
                .map(|(f, s)| {
                    let s = s?;
                    div(((*f)? - s) * 100.0, s)
                })
                .collect();
            let signal = smooth(&r, per("signal")?);
            ser!(
                "histogram",
                r.iter()
                    .zip(&signal)
                    .map(|(r, s)| Some((*r)? - (*s)?))
                    .collect(),
                "percentage points"
            );
            ser!("pvo", r, "percent");
            ser!("signal", signal, "percent");
        }
        "performance" => {
            let a = index(v, "anchor_index")?;
            ser!(
                id,
                (0..n)
                    .map(|i| if a < n && i >= a {
                        Some((c[i] / c[a] - 1.0) * 100.0)
                    } else {
                        None
                    })
                    .collect(),
                "percent"
            );
        }
        "pmo" => {
            let mut roc = vec![None; n];
            for i in 1..n {
                roc[i] = Some((c[i] / c[i - 1] - 1.0) * 100.0);
            }
            let a = alpha_smooth(&roc, per("first_period")?);
            let r: Vec<_> = alpha_smooth(&a, per("second_period")?)
                .into_iter()
                .map(|x| x.map(|x| x * 10.0))
                .collect();
            ser!("signal", smooth(&r, per("signal")?), "PMO units");
            ser!("pmo", r, "PMO units");
        }
        "special_k" => {
            let roc = periods(v, "roc_periods")?;
            let smoothp = periods(v, "smooth_periods")?;
            let weights = arr(v, "weights")?;
            if roc.is_empty() || roc.len() != smoothp.len() || roc.len() != weights.len() {
                return Err("Special K 三组参数须非空等长".into());
            }
            let mut out = vec![Some(0.0); n];
            for j in 0..roc.len() {
                let r = mom::roc(&c, roc[j]);
                let r = ma::optional_smooth(&r, smoothp[j], ma::sma);
                for i in 0..n {
                    out[i] = out[i].zip(r[i]).map(|(a, b)| a + b * weights[j]);
                }
            }
            ser!(id, out, "weighted ROC percentage points");
        }
        "rci" => ser!(id, rci(&c, p), "index -100..100"),
        "rci_ribbon" => {
            for p in periods(v, "periods")? {
                ser!(&format!("rci_{p}"), rci(&c, p), "index -100..100");
            }
        }
        "relative_volatility_index" => {
            let sd = rolling(&c, p, stddev);
            let mut up = vec![None; n];
            let mut down = up.clone();
            for i in 1..n {
                up[i] = sd[i].map(|s| if c[i] > c[i - 1] { s } else { 0.0 });
                down[i] = sd[i].map(|s| if c[i] < c[i - 1] { s } else { 0.0 });
            }
            let up = ma::optional_smooth(&up, per("smooth_period")?, ma::rma);
            let down = ma::optional_smooth(&down, per("smooth_period")?, ma::rma);
            ser!(
                id,
                up.iter()
                    .zip(down)
                    .map(|(a, b)| {
                        let a = (*a)?;
                        div(a * 100.0, a + b?)
                    })
                    .collect(),
                "index 0..100"
            );
        }
        "smi_ergodic_oscillator" => {
            let tsi = mom::tsi(&c, per("long")?, per("short")?);
            let signal = smooth(&tsi, per("signal")?);
            ser!(
                id,
                tsi.iter()
                    .zip(signal)
                    .map(|(a, b)| Some((*a)? - b?))
                    .collect(),
                "index difference"
            );
        }
        "stochastic_momentum_index" => {
            let mut dist = vec![None; n];
            let mut range = dist.clone();
            for i in p - 1..n {
                let w = &b[i + 1 - p..=i];
                let h = w.iter().map(|x| x.high).fold(f64::NEG_INFINITY, f64::max);
                let l = w.iter().map(|x| x.low).fold(f64::INFINITY, f64::min);
                dist[i] = Some(c[i] - (h + l) / 2.0);
                range[i] = Some((h - l) / 2.0);
            }
            let ds = smooth(&smooth(&dist, per("first_smooth")?), per("second_smooth")?);
            let rs = smooth(&smooth(&range, per("first_smooth")?), per("second_smooth")?);
            ser!(
                id,
                ds.iter()
                    .zip(rs)
                    .map(|(a, b)| div((*a)? * 100.0, b?))
                    .collect(),
                "index -100..100"
            );
        }
        "twap" => {
            let a = index(v, "anchor_index")?;
            let mut sum = 0.0;
            ser!(
                id,
                (0..n)
                    .map(|i| if i < a {
                        None
                    } else {
                        sum += b[i].typical_price();
                        Some(sum / (i - a + 1) as f64)
                    })
                    .collect(),
                "price"
            );
            o.note("等长K线按典型价等权，不使用成交量权重。");
        }
        "trading_sessions" => {
            let start = index(v, "start_utc_hour")?;
            let end = index(v, "end_utc_hour")?;
            if start > 23 || end > 24 || start >= end {
                return Err("示例时段需0<=start<end<=24；跨午夜请先切分".into());
            }
            let day = b[n - 1].timestamp.timestamp().div_euclid(86400);
            let chosen: Vec<_> = b
                .iter()
                .filter(|b| b.timestamp.timestamp().div_euclid(86400) == day)
                .filter(|b| {
                    let h = b.timestamp.timestamp().rem_euclid(86400) / 3600;
                    h >= start as i64 && h < end as i64
                })
                .collect();
            o.value(
                "session_high",
                chosen.iter().map(|b| b.high).reduce(f64::max),
                "price",
                "最新UTC日期的时段没有K线",
            );
            o.value(
                "session_low",
                chosen.iter().map(|b| b.low).reduce(f64::min),
                "price",
                "最新UTC日期的时段没有K线",
            );
            o.number("bars_in_session", chosen.len() as f64, "count");
        }
        "trend_strength_index" => ser!(
            id,
            rolling(&c, p, |w| stats::correlation(
                w,
                &(0..w.len()).map(|i| i as f64).collect::<Vec<_>>()
            )),
            "correlation -1..1"
        ),
        "lower_timeframe_volume" => {
            let up = b
                .iter()
                .filter(|b| b.close > b.open)
                .map(|b| b.volume)
                .sum::<f64>();
            let down = b
                .iter()
                .filter(|b| b.close < b.open)
                .map(|b| b.volume)
                .sum::<f64>();
            let flat = b
                .iter()
                .filter(|b| b.close == b.open)
                .map(|b| b.volume)
                .sum::<f64>();
            o.number("up_bar_volume", up, "volume");
            o.number("down_bar_volume", down, "volume");
            o.number("unclassified_flat_volume", flat, "volume");
            o.number("estimated_delta", up - down, "volume");
            o.note(
                "对传入K线按涨跌代理分类，不是真实逐笔主动买卖，也不会假设未获取的更低周期数据。",
            );
        }
        "visible_average_price" => {
            let start = index(v, "start_index")?;
            let end = index(v, "end_index")?;
            if start > end {
                return Err("start_index不能大于end_index".into());
            }
            o.value(
                id,
                if start < n {
                    mean(&c[start..=end.min(n - 1)])
                } else {
                    None
                },
                "price",
                "可见窗口没有数据",
            );
        }
        "volatility_stop" => {
            let m = nonnegative(v, "multiplier")?;
            ser!(
                "long_reference",
                rolling(&c, p, |w| stddev(w).map(|s| w
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
                    - m * s)),
                "price"
            );
            ser!(
                "short_reference",
                rolling(&c, p, |w| stddev(w).map(|s| w
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min)
                    + m * s)),
                "price"
            );
        }
        "woodies_cci" => {
            ser!("short_cci", mom::cci(b, per("fast")?), "CCI units");
            ser!("long_cci", mom::cci(b, per("slow")?), "CCI units");
        }
        "auto_fib" | "auto_trendlines" => {
            let pivots = confirmed_pivots(b, per("right_bars")?);
            if id == "auto_fib" {
                let pair: Vec<_> = pivots.iter().rev().take(2).collect();
                if pair.len() == 2 {
                    let price =
                        |p: &&(usize, usize, i8)| if p.2 > 0 { b[p.1].high } else { b[p.1].low };
                    let latest = price(&pair[0]);
                    let previous = price(&pair[1]);
                    o.number("candidate_start", previous, "price");
                    o.number("candidate_end", latest, "price");
                    for (k, r) in [
                        ("retrace_382", 0.382),
                        ("retrace_618", 0.618),
                        ("extension_1618", -0.618),
                    ] {
                        o.number(k, latest - (latest - previous) * r, "price");
                    }
                    o.number("anchor_confirmed_at", pair[0].0 as f64, "bar index");
                } else {
                    o.value(id, None, "price", "需要两个已确认枢轴");
                }
            } else {
                for (sign, name) in [(1, "resistance"), (-1, "support")] {
                    let points: Vec<_> = pivots
                        .iter()
                        .filter(|p| p.2 == sign)
                        .rev()
                        .take(2)
                        .collect();
                    if points.len() == 2 {
                        let (a, z) = (points[1], points[0]);
                        let price = |p: &(usize, usize, i8)| {
                            if sign > 0 {
                                b[p.1].high
                            } else {
                                b[p.1].low
                            }
                        };
                        let slope = (price(z) - price(a)) / (z.1 - a.1) as f64;
                        o.number(name, price(z) + slope * (n - 1 - z.1) as f64, "price");
                    } else {
                        o.value(name, None, "price", "需要同方向两个已确认枢轴");
                    }
                }
            }
        }
        "rsi_divergence" => divergence(b, &mom::rsi(&c, p), per("right_bars")?, o),
        "smi_ergodic_indicator" => {
            let tsi = mom::tsi(&c, per("long")?, per("short")?);
            ser!("signal", smooth(&tsi, per("signal")?), "index");
            ser!("main", tsi, "index");
        }
        "auto_anchored_vwap" => {
            let a = (0..n)
                .max_by(|a, z| b[*a].volume.total_cmp(&b[*z].volume))
                .unwrap();
            o.number("anchor_index", a as f64, "bar index");
            o.value(
                "current_auto_avwap",
                volume::vwap(&b[a..]).last().copied().flatten(),
                "price",
                "锚点后成交量为零",
            );
            o.number("known_at_index", (n - 1) as f64, "bar index");
            o.note("锚点随新增数据可能变化，因此仅报告当前时点结果，不把当前选择回填历史信号。");
        }
        _ => return Err(format!("没有市场技术 evaluator: {id}")),
    }
    Ok(())
}
fn rci(c: &[f64], p: usize) -> Vec<Option<f64>> {
    rolling(c, p, |w| {
        let ranks = ranks(w);
        let times: Vec<_> = (0..w.len()).map(|i| i as f64).collect();
        stats::correlation(&ranks, &times).map(|r| r * 100.0)
    })
}
fn alpha_smooth(v: &[Option<f64>], p: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; v.len()];
    let mut values = Vec::new();
    let mut prev = None;
    for (i, v) in v.iter().enumerate() {
        if let Some(v) = v {
            values.push(*v);
            if let Some(z) = prev {
                let next = z + (2.0 / p as f64) * (*v - z);
                prev = Some(next);
                out[i] = prev;
            } else if values.len() >= p {
                prev = Some(values[values.len() - p..].iter().sum::<f64>() / p as f64);
                out[i] = prev;
            }
        } else {
            values.clear();
            prev = None;
        }
    }
    out
}
fn confirmed_pivots(b: &[Bar], r: usize) -> Vec<(usize, usize, i8)> {
    let mut out = vec![];
    if b.len() < r * 2 + 1 {
        return out;
    }
    for i in r..b.len() - r {
        if (i - r..=i + r).all(|j| j == i || b[i].high > b[j].high) {
            out.push((i + r, i, 1));
        }
        if (i - r..=i + r).all(|j| j == i || b[i].low < b[j].low) {
            out.push((i + r, i, -1));
        }
    }
    out
}
fn divergence(b: &[Bar], indicator: &[Option<f64>], r: usize, o: &mut Output) {
    let mut out = vec![None; b.len()];
    let mut last_high = None;
    let mut last_low = None;
    for (when, i, sign) in confirmed_pivots(b, r) {
        if let Some(value) = indicator[i] {
            let prior = if sign > 0 { last_high } else { last_low };
            if let Some((old_price, old_value)) = prior {
                out[when] = Some(if sign > 0 && b[i].high > old_price && value < old_value {
                    1.0
                } else if sign < 0 && b[i].low < old_price && value > old_value {
                    -1.0
                } else {
                    0.0
                });
            }
            if sign > 0 {
                last_high = Some((b[i].high, value));
            } else {
                last_low = Some((b[i].low, value));
            }
        }
    }
    o.series("confirmed_divergence", out, "bear +1 / bull -1");
}
fn ohlc_vol(b: &[Bar], p: usize, annual: f64, o: &mut Output) -> Result<(), String> {
    if p < 2 {
        return Err("OHLC波动窗口至少2".into());
    }
    let n = b.len();
    let mut pk = vec![None; n];
    let mut gk = pk.clone();
    let mut yz = pk.clone();
    for i in p..n {
        let w = &b[i + 1 - p..=i];
        let par = w.iter().map(|b| (b.high / b.low).ln().powi(2)).sum::<f64>()
            / (4.0 * 2f64.ln() * p as f64);
        let g = w
            .iter()
            .map(|b| {
                0.5 * (b.high / b.low).ln().powi(2)
                    - (2.0 * 2f64.ln() - 1.0) * (b.close / b.open).ln().powi(2)
            })
            .sum::<f64>()
            / p as f64;
        let opens: Vec<_> = (i + 1 - p..=i)
            .map(|j| (b[j].open / b[j - 1].close).ln())
            .collect();
        let closes: Vec<_> = w.iter().map(|b| (b.close / b.open).ln()).collect();
        let rs = w
            .iter()
            .map(|b| {
                (b.high / b.open).ln() * (b.high / b.close).ln()
                    + (b.low / b.open).ln() * (b.low / b.close).ln()
            })
            .sum::<f64>()
            / p as f64;
        let k = 0.34 / (1.34 + (p as f64 + 1.0) / (p as f64 - 1.0));
        let y =
            stddev(&opens).unwrap().powi(2) + k * stddev(&closes).unwrap().powi(2) + (1.0 - k) * rs;
        pk[i] = Some((par.max(0.0) * annual).sqrt());
        gk[i] = Some((g.max(0.0) * annual).sqrt());
        yz[i] = Some((y.max(0.0) * annual).sqrt());
    }
    o.series("parkinson", pk, "annual volatility fraction");
    o.series("garman_klass", gk, "annual volatility fraction");
    o.series("yang_zhang", yz, "annual volatility fraction");
    Ok(())
}
