use super::*;
use crate::indicators::{
    extra, ma, momentum as mom, statistics as stats, trend, volatility as vol, volume,
};
pub(super) fn is_market(id: &str) -> bool {
    matches!(
        id,
        "inside_outside"
            | "cvd"
            | "latest_price"
            | "ohlc"
            | "amplitude"
            | "vwap_price"
            | "volume_ratio"
            | "sma"
            | "ema"
            | "wma"
            | "rma"
            | "hma"
            | "dema"
            | "vwma"
            | "bbi"
            | "macd"
            | "dmi_adx"
            | "aroon"
            | "parabolic_sar"
            | "supertrend"
            | "donchian"
            | "keltner"
            | "ichimoku"
            | "zigzag"
            | "alligator"
            | "bias"
            | "rsi"
            | "stochastic"
            | "kdj"
            | "stoch_rsi"
            | "cci"
            | "williams_r"
            | "momentum"
            | "roc"
            | "cmo"
            | "tsi"
            | "ultimate"
            | "awesome"
            | "dpo"
            | "atr"
            | "atr_percent"
            | "hv"
            | "bbands"
            | "squeeze"
            | "chaikin_vol"
            | "mass_index"
            | "ulcer_index"
            | "obv"
            | "adl"
            | "cmf"
            | "mfi"
            | "vwap"
            | "chaikin_osc"
            | "pvt"
            | "force_index"
            | "nvi"
            | "vol_osc"
            | "vroc"
            | "vr"
            | "wvad"
            | "z_score"
            | "hurst"
            | "support_resistance"
            | "trendline"
            | "pivot_points"
            | "fibonacci_retracement"
            | "fibonacci_extension"
            | "k_pattern_hammer"
            | "k_pattern_doji"
            | "k_pattern_engulfing"
            | "k_pattern_star"
            | "fractal"
            | "elder_ray"
            | "bop"
            | "fisher_transform"
            | "rvi"
            | "demarker"
            | "kst"
            | "coppock"
            | "psy"
            | "arbr"
            | "cr"
            | "td_sequential"
            | "anchored_vwap"
            | "klinger"
            | "pitfall_rsi"
            | "pitfall_golden_cross"
            | "pitfall_divergence"
            | "pitfall_multi_osc"
            | "pitfall_volume"
    )
}
pub(super) fn defaults(id: &str) -> Value {
    match id {
        "macd" => json!({"fast":12,"slow":26,"signal":9}),
        "ichimoku" => json!({"tenkan":9,"kijun":26,"senkou_b":52,"displacement":26}),
        "parabolic_sar" => json!({"af_start":0.02,"af_step":0.02,"af_max":0.2}),
        "zigzag" => json!({"threshold_pct":5.0}),
        "anchored_vwap" => json!({"anchor_index":0}),
        "bbands" | "keltner" | "supertrend" | "squeeze" => json!({"period":20,"multiplier":2.0}),
        "hv" => json!({"period":20,"periods_per_year":8760.0}),
        "tsi" => json!({"long":25,"short":13}),
        "ultimate" => json!({"short":7,"medium":14,"long":28}),
        "awesome" | "vol_osc" | "klinger" | "chaikin_osc" => json!({"fast":5,"slow":34}),
        "kdj" | "stochastic" | "stoch_rsi" => json!({"period":14,"smooth_k":3,"smooth_d":3}),
        "pitfall_rsi" => json!({"period":14,"overbought":70.0,"oversold":30.0}),
        "pitfall_golden_cross" => json!({"fast":5,"slow":20}),
        "fibonacci_extension" => json!({"period":30,"extension_ratio":1.618}),
        "inside_outside" | "cvd" => json!({}),
        "latest_price"
        | "ohlc"
        | "amplitude"
        | "vwap_price"
        | "vwap"
        | "obv"
        | "adl"
        | "pvt"
        | "force_index"
        | "nvi"
        | "wvad"
        | "bbi"
        | "alligator"
        | "fractal"
        | "kst"
        | "coppock"
        | "td_sequential"
        | "pivot_points"
        | "k_pattern_hammer"
        | "k_pattern_doji"
        | "k_pattern_engulfing"
        | "k_pattern_star" => json!({}),
        _ => json!({"period":14}),
    }
}
pub(super) fn evaluate(id: &str, b: &[Bar], v: &Value, o: &mut Output) -> Result<(), String> {
    let c: Vec<_> = b.iter().map(|x| x.close).collect();
    let n = b.len();
    let last = &b[n - 1];
    let p = if v.get("period").is_some() {
        period(v, "period")?
    } else {
        14
    };
    let param = |k| num(v, k);
    let per = |k| period(v, k);
    let mult = if v.get("multiplier").is_some() {
        let m = param("multiplier")?;
        if m <= 0.0 {
            return Err("multiplier 必须大于零".into());
        }
        m
    } else {
        2.0
    };
    macro_rules! ser {
        ($expr:expr,$unit:expr) => {
            o.series(id, $expr, $unit)
        };
    }
    match id {
        "inside_outside" | "cvd" => {
            return Err("该概念必须由 API 使用 Binance 已收盘现货K线的主动成交字段计算".into())
        }
        "latest_price" => {
            o.number("latest_close", last.close, "price");
            let prev = b.get(n.wrapping_sub(2)).map(|x| x.close);
            o.value(
                "bar_change",
                prev.map(|x| last.close - x),
                "price",
                "至少两根 K 线",
            );
            o.value(
                "bar_return",
                prev.and_then(|x| div(last.close - x, x)),
                "fraction",
                "至少两根 K 线",
            );
            o.note("上一根收盘不是上一交易日昨收；此处展示逐 K 线变动。");
        }
        "ohlc" => {
            for (k, x) in [
                ("open", last.open),
                ("high", last.high),
                ("low", last.low),
                ("close", last.close),
            ] {
                o.number(k, x, "price");
            }
            o.number("volume", last.volume, "base units");
        }
        "amplitude" => ser!(
            (0..n)
                .map(|i| if i == 0 {
                    None
                } else {
                    div((b[i].high - b[i].low) * 100.0, b[i - 1].close)
                })
                .collect(),
            "percent per bar"
        ),
        "sma" => ser!(ma::sma(&c, p), "price"),
        "ema" => ser!(ma::ema(&c, p), "price"),
        "wma" => ser!(ma::wma(&c, p), "price"),
        "rma" => ser!(ma::rma(&c, p), "price"),
        "hma" => ser!(ma::hma(&c, p), "price"),
        "dema" => ser!(ma::dema(&c, p), "price"),
        "vwma" => ser!(ma::vwma(b, p), "price"),
        "bbi" => ser!(ma::bbi(b), "price"),
        "bias" => ser!(ma::bias(&c, p), "percent"),
        "macd" => {
            let r = trend::macd(&c, per("fast")?, per("slow")?, per("signal")?);
            o.series("dif", r.dif, "price");
            o.series("dea", r.dea, "price");
            o.series("hist", r.hist, "price");
        }
        "dmi_adx" => {
            let r = trend::dmi(b, p);
            o.series("plus_di", r.plus_di, "percent");
            o.series("minus_di", r.minus_di, "percent");
            o.series("adx", r.adx, "index 0..100");
        }
        "aroon" => {
            let r = trend::aroon(b, p);
            o.series("up", r.up, "percent");
            o.series("down", r.down, "percent");
        }
        "parabolic_sar" => {
            let a = param("af_start")?;
            let s = param("af_step")?;
            let m = param("af_max")?;
            if a <= 0.0 || s <= 0.0 || m < a {
                return Err("SAR 加速参数必须正且 max >= start".into());
            }
            ser!(trend::parabolic_sar(b, a, s, m).sar, "price");
        }
        "supertrend" => {
            let r = trend::supertrend(b, p, mult);
            o.series("supertrend", r.trend, "price");
            o.series(
                "direction",
                r.direction
                    .into_iter()
                    .map(|x| x.map(|x| x as f64))
                    .collect(),
                "sign",
            );
        }
        "donchian" | "support_resistance" => {
            let r = trend::donchian(b, p);
            o.series("prior_high_resistance", r.upper, "price");
            o.series("prior_low_support", r.lower, "price");
            o.note("取前 N 根已完成 K 线极值，不包含当前 K 线；属于滚动区间定义。");
        }
        "keltner" => {
            let r = trend::keltner(b, p, mult);
            o.series("upper", r.upper, "price");
            o.series("middle", r.middle, "price");
            o.series("lower", r.lower, "price");
        }
        "ichimoku" => {
            let r = trend::ichimoku(
                b,
                per("tenkan")?,
                per("kijun")?,
                per("senkou_b")?,
                per("displacement")?,
            );
            o.series("tenkan", r.tenkan, "price");
            o.series("kijun", r.kijun, "price");
            o.series("senkou_a", r.senkou_a, "price");
            o.series("senkou_b", r.senkou_b, "price");
            o.note("迟行线是当前价格回画历史，本实践不输出它以避免误作历史可得信号。");
        }
        "zigzag" => {
            let threshold = param("threshold_pct")?;
            if threshold <= 0.0 || threshold >= 100.0 {
                return Err("threshold_pct 必须在 0..100 内".into());
            }
            let mut direction = 0;
            let mut extreme = c[0];
            let mut r = vec![None; n];
            for i in 1..n {
                let reversal = (c[i] / extreme - 1.0) * 100.0;
                if direction >= 0 && reversal <= -threshold {
                    r[i] = Some(extreme);
                    direction = -1;
                    extreme = c[i];
                } else if direction <= 0 && reversal >= threshold {
                    r[i] = Some(extreme);
                    direction = 1;
                    extreme = c[i];
                } else if (direction >= 0 && c[i] > extreme) || (direction <= 0 && c[i] < extreme) {
                    extreme = c[i];
                }
            }
            ser!(r, "confirmed pivot price");
            o.note("转折仅在回撤超过阈值的确认时点发出，不回填峰谷时点；末端未确认转折不输出。");
        }
        "alligator" => {
            let r = ma::alligator(b);
            o.series("jaw", r.jaw, "price");
            o.series("teeth", r.teeth, "price");
            o.series("lips", r.lips, "price");
        }
        "rsi" => ser!(mom::rsi(&c, p), "index 0..100"),
        "stochastic" => {
            let r = mom::stochastic(b, p, per("smooth_d")?, per("smooth_k")?);
            o.series("k", r.k, "index 0..100");
            o.series("d", r.d, "index 0..100");
        }
        "kdj" => {
            let r = mom::kdj(b, p, per("smooth_k")?, per("smooth_d")?);
            o.series("k", r.k, "index 0..100");
            o.series("d", r.d, "index 0..100");
            o.series("j", r.j, "index (unbounded)");
        }
        "stoch_rsi" => {
            let r = mom::stochastic_rsi(&c, p, p, per("smooth_k")?, per("smooth_d")?);
            o.series("k", r.k, "index 0..100");
            o.series("d", r.d, "index 0..100");
        }
        "cci" => ser!(mom::cci(b, p), "index"),
        "williams_r" => ser!(mom::williams_r(b, p), "index -100..0"),
        "momentum" => ser!(mom::momentum(&c, p), "price"),
        "roc" => ser!(mom::roc(&c, p), "percent"),
        "cmo" => ser!(mom::cmo(&c, p), "index -100..100"),
        "tsi" => ser!(mom::tsi(&c, per("long")?, per("short")?), "index"),
        "ultimate" => ser!(
            mom::ultimate_oscillator(b, per("short")?, per("medium")?, per("long")?),
            "index 0..100"
        ),
        "awesome" => ser!(
            mom::awesome_oscillator(b, per("fast")?, per("slow")?),
            "price"
        ),
        "dpo" => {
            ser!(mom::dpo(&c, p), "price");
            o.note("采用未居中 DPO；不将结果向过去回画。");
        }
        "atr" => ser!(vol::atr(b, p), "price"),
        "atr_percent" => ser!(vol::atr_percent(b, p), "percent"),
        "hv" => {
            let y = param("periods_per_year")?;
            if y <= 0.0 {
                return Err("periods_per_year 必须正".into());
            }
            ser!(vol::historical_volatility(&c, p, y), "annualized fraction");
        }
        "bbands" => {
            let r = vol::bollinger_bands(&c, p, mult);
            o.series("upper", r.upper, "price");
            o.series("middle", r.middle, "price");
            o.series("lower", r.lower, "price");
        }
        "squeeze" => ser!(
            vol::squeeze(b, p, p, mult, 1.5)
                .into_iter()
                .map(|x| x.map(|b| if b { 1.0 } else { 0.0 }))
                .collect(),
            "boolean (1/0)"
        ),
        "chaikin_vol" => ser!(vol::chaikin_volatility(b, p, p), "percent"),
        "mass_index" => ser!(vol::mass_index(b, 9, p), "index"),
        "ulcer_index" => ser!(vol::ulcer_index(&c, p), "percent"),
        "obv" => ser!(volume::obv(b), "base units"),
        "adl" => ser!(volume::adl(b), "base units"),
        "cmf" => ser!(volume::cmf(b, p), "ratio"),
        "mfi" => ser!(volume::mfi(b, p), "index 0..100"),
        "vwap" | "vwap_price" => {
            ser!(volume::vwap(b), "price");
            o.note("所选区间累计典型价 VWAP：sum((H+L+C)/3*V)/sum(V)，非逐笔成交均价，未自动按交易日重置。");
        }
        "chaikin_osc" => ser!(
            volume::chaikin_oscillator(b, per("fast")?, per("slow")?),
            "base units"
        ),
        "pvt" => ser!(volume::pvt(b), "base units"),
        "force_index" => ser!(volume::force_index(b), "price × base units"),
        "nvi" => ser!(volume::nvi(b), "index base 1000"),
        "vol_osc" => ser!(
            volume::volume_oscillator(b, per("fast")?, per("slow")?),
            "percent"
        ),
        "vroc" => ser!(volume::vroc(b, p), "percent"),
        "vr" => ser!(volume::vr(b, p), "percent"),
        "wvad" => ser!(volume::wvad(b), "base units"),
        "volume_ratio" => {
            ser!(volume::volume_ratio(b, p), "ratio");
            o.note("当前 K 线量与滚动均量比；并非股票日内相同交易时段量比。");
        }
        "z_score" => ser!(stats::zscore(&c, p), "standard deviations"),
        "hurst" => {
            let data = &c[n.saturating_sub(p.max(20))..];
            o.value(
                "hurst",
                stats::hurst(data),
                "R/S estimate",
                "Hurst 样本或波动不足",
            );
            o.note("单窗口 R/S 粗估计；不是可靠的趋势概率或买卖信号。");
        }
        "trendline" => {
            let data = &c[n.saturating_sub(p)..];
            let r = stats::linear_regression(data);
            o.value("slope_per_bar", r.map(|x| x.0), "price/bar", "至少两点");
            o.value(
                "fitted_current",
                r.map(|(s, i)| s * (data.len() - 1) as f64 + i),
                "price",
                "至少两点",
            );
            o.note("最小二乘趋势线使用当前窗口历史值，不是人工选择高低点的唯一趋势线。");
        }
        "pivot_points" => {
            if n < 2 {
                o.value("pivot", None, "price", "需要前一根 K 线");
            } else {
                let b = &b[n - 2];
                let r = extra::pivot_points(b.high, b.low, b.close);
                for (k, x) in [
                    ("pivot", r.pivot),
                    ("r1", r.r1),
                    ("r2", r.r2),
                    ("r3", r.r3),
                    ("s1", r.s1),
                    ("s2", r.s2),
                    ("s3", r.s3),
                ] {
                    o.number(k, x, "price");
                }
            }
        }
        "fibonacci_retracement" | "fibonacci_extension" => {
            let w = &b[n.saturating_sub(p)..];
            let hi = w.iter().map(|x| x.high).fold(f64::NEG_INFINITY, f64::max);
            let lo = w.iter().map(|x| x.low).fold(f64::INFINITY, f64::min);
            if id == "fibonacci_retracement" {
                for (k, r) in [
                    ("retracement_236", 0.236),
                    ("retracement_382", 0.382),
                    ("retracement_500", 0.5),
                    ("retracement_618", 0.618),
                ] {
                    o.number(k, hi - (hi - lo) * r, "price");
                }
            } else {
                o.number(
                    "upward_extension",
                    lo + (hi - lo) * param("extension_ratio")?,
                    "price",
                );
            }
            o.note("以选定窗口低点到高点作为明确的上行参考段，不宣称自动识别了真实波段。");
        }
        "k_pattern_hammer" | "k_pattern_doji" => {
            let target = if id == "k_pattern_hammer" {
                extra::CandlePattern::Hammer
            } else {
                extra::CandlePattern::Doji
            };
            ser!(
                b.iter()
                    .map(|x| Some(if extra::detect_pattern(x) == target {
                        1.0
                    } else {
                        0.0
                    }))
                    .collect(),
                "boolean (1/0)"
            );
        }
        "k_pattern_engulfing" => ser!(
            (0..n)
                .map(|i| if i == 0 {
                    None
                } else {
                    Some(
                        if extra::detect_engulfing(&b[i - 1], &b[i])
                            == extra::CandlePattern::Engulfing
                        {
                            if b[i].close > b[i].open {
                                1.0
                            } else {
                                -1.0
                            }
                        } else {
                            0.0
                        },
                    )
                })
                .collect(),
            "bull +1 / bear -1 / none 0"
        ),
        "k_pattern_star" => ser!(
            (0..n)
                .map(|i| if i < 2 {
                    None
                } else {
                    Some(extra::detect_star(&b[i - 2], &b[i - 1], &b[i]) as f64)
                })
                .collect(),
            "morning +1 / evening -1 / none 0"
        ),
        "fractal" => {
            let f = ma::fractal(b);
            let mut r = vec![None; n];
            for i in 2..n {
                r[i] = Some(f[i - 2] as f64);
            }
            ser!(r, "confirmed sign");
            o.note("5 根分形在中心点后两根确认，结果记录在确认时点。");
        }
        "elder_ray" => {
            let (a, z) = mom::elder_ray(b, p);
            o.series("bull_power", a, "price");
            o.series("bear_power", z, "price");
        }
        "bop" => ser!(mom::bop(b), "ratio"),
        "fisher_transform" => ser!(mom::fisher_transform(&c, p), "transform"),
        "rvi" => ser!(mom::rvi(b, p), "ratio"),
        "demarker" => ser!(mom::demarker(b, p), "index"),
        "kst" => ser!(mom::kst(&c), "index"),
        "coppock" => ser!(mom::coppock(&c), "index"),
        "psy" => ser!(mom::psy(&c, p), "percent"),
        "arbr" => {
            o.series("ar", mom::ar(b, p), "percent");
            o.series("br", mom::br(b, p), "percent");
        }
        "cr" => ser!(mom::cr(b, p), "index"),
        "td_sequential" => ser!(
            trend::td_sequential(&c)
                .into_iter()
                .map(|x| Some(x as f64))
                .collect(),
            "setup count"
        ),
        "anchored_vwap" => {
            let a = param("anchor_index")?;
            if a < 0.0 || a.fract() != 0.0 {
                return Err("anchor_index 必须非负整数".into());
            }
            let a = a as usize;
            let mut r = vec![None; n];
            if a < n {
                let z = volume::vwap(&b[a..]);
                r[a..].copy_from_slice(&z);
            }
            ser!(r, "price");
        }
        "klinger" => ser!(
            volume::klinger(b, per("fast")?, per("slow")?),
            "volume force"
        ),
        "pitfall_rsi" => {
            let r = mom::rsi(&c, p);
            let z = r.last().copied().flatten();
            o.value("rsi", z, "index 0..100", "RSI 预热不足");
            o.value(
                "overbought_observation",
                z.map(|x| {
                    if x > param("overbought").unwrap() {
                        1.0
                    } else {
                        0.0
                    }
                }),
                "boolean (1/0)",
                "RSI 预热不足",
            );
            o.note("超买/超卖仅描述当前强弱，不自动生成反向交易；应结合趋势与样本外评价。");
        }
        "pitfall_golden_cross" => {
            let f = ma::sma(&c, per("fast")?);
            let s = ma::sma(&c, per("slow")?);
            let cross = if n >= 2 {
                match (f[n - 2], s[n - 2], f[n - 1], s[n - 1]) {
                    (Some(a), Some(b), Some(c), Some(d)) => {
                        Some(if a <= b && c > d { 1.0 } else { 0.0 })
                    }
                    _ => None,
                }
            } else {
                None
            };
            o.value("golden_cross_now", cross, "boolean (1/0)", "均线预热不足");
            o.note("交叉基于已发生价格，不能当作趋势起点的提前预测。");
        }
        "pitfall_divergence" => {
            let r = mom::rsi(&c, p);
            let z = if n > p {
                match (r[n - 1], r[n - 1 - p]) {
                    (Some(a), Some(z)) => Some(if c[n - 1] > c[n - 1 - p] && a < z {
                        1.0
                    } else if c[n - 1] < c[n - 1 - p] && a > z {
                        -1.0
                    } else {
                        0.0
                    }),
                    _ => None,
                }
            } else {
                None
            };
            o.value(
                "endpoint_divergence",
                z,
                "bear +1 / bull -1 / none 0",
                "需要两个已预热端点",
            );
            o.note("这是固定间距端点背离诊断，不冒充已确认的局部峰谷背离，也不承诺反转。");
        }
        "pitfall_multi_osc" => {
            let r = mom::rsi(&c, p);
            let k = mom::stochastic(b, p, 3, 3).k;
            let pairs: Vec<_> = r
                .into_iter()
                .zip(k)
                .filter_map(|(a, b)| Some((a?, b?)))
                .collect();
            let a: Vec<_> = pairs.iter().map(|x| x.0).collect();
            let z: Vec<_> = pairs.iter().map(|x| x.1).collect();
            o.value(
                "rsi_stochastic_correlation",
                stats::correlation(&a, &z),
                "correlation",
                "至少两个有效且有方差的配对样本",
            );
            o.note("高度相关的震荡指标不等于多份独立证据。");
        }
        "pitfall_volume" => {
            o.series("relative_volume", volume::volume_ratio(b, p), "ratio");
            o.number(
                "current_return",
                if n > 1 {
                    last.close / c[n - 2] - 1.0
                } else {
                    0.0
                },
                "fraction",
            );
            o.note("成交量没有买卖方向；放量不能证明主力净买入。");
        }
        _ => return Err(format!("市场概念没有 evaluator: {id}")),
    }
    Ok(())
}
