//! Causal, source-backed alpha research examples.
//! Qlib Alpha158 below mirrors the default configuration in Microsoft's loader.
use crate::types::Bar;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const WQ_SOURCE: &str = "https://arxiv.org/pdf/1601.00991";
const QLIB_SOURCE: &str = "https://github.com/microsoft/qlib/blob/main/qlib/contrib/data/loader.py";
const WINDOWS: [usize; 5] = [5, 10, 20, 30, 60];
const KBAR: [(&str, &str); 9] = [
    ("kmid", "($close-$open)/$open"),
    ("klen", "($high-$low)/$open"),
    ("kmid2", "($close-$open)/($high-$low+1e-12)"),
    ("kup", "($high-Greater($open,$close))/$open"),
    ("kup2", "($high-Greater($open,$close))/($high-$low+1e-12)"),
    ("klow", "(Less($open,$close)-$low)/$open"),
    ("klow2", "(Less($open,$close)-$low)/($high-$low+1e-12)"),
    ("ksft", "(2*$close-$high-$low)/$open"),
    ("ksft2", "(2*$close-$high-$low)/($high-$low+1e-12)"),
];
const ROLLING: [&str; 29] = [
    "roc", "ma", "std", "beta", "rsqr", "resi", "max", "min", "qtlu", "qtld", "rank", "rsv",
    "imax", "imin", "imxd", "corr", "cord", "cntp", "cntn", "cntd", "sump", "sumn", "sumd", "vma",
    "vstd", "wvma", "vsump", "vsumn", "vsumd",
];

#[derive(Clone, Serialize)]
pub struct Factor {
    pub id: String,
    pub name: String,
    pub status: &'static str,
    pub required_fields: Vec<String>,
    pub summary: String,
    pub formula: Option<String>,
    pub source_url: &'static str,
    pub code_url: Option<String>,
}

#[derive(Serialize)]
pub struct Collection {
    pub id: &'static str,
    pub name: &'static str,
    pub source_url: &'static str,
    pub total: usize,
    pub factors: Vec<Factor>,
}

pub fn catalog() -> Vec<Collection> {
    vec![
        Collection {
            id: "worldquant101",
            name: "WorldQuant Alpha101",
            source_url: WQ_SOURCE,
            total: 101,
            factors: (1..=101).map(wq_factor).collect(),
        },
        Collection {
            id: "qlib_alpha158",
            name: "Microsoft Qlib Alpha158",
            source_url: QLIB_SOURCE,
            total: 158,
            factors: qlib_catalog(),
        },
    ]
}

fn wq_factor(n: usize) -> Factor {
    let entry = &wq_manifest().factors[n - 1];
    let ok = n == 101;
    Factor {
        id: entry.id.clone(),
        name: format!("Alpha {n}"),
        status: if ok { "available" } else { "unavailable" },
        required_fields: entry.required_fields.clone(),
        summary: entry.summary.clone(),
        formula: Some(entry.formula.clone()),
        source_url: WQ_SOURCE,
        code_url: ok
            .then(|| crate::code_links::resolve("src/alpha_research.rs::wq_101"))
            .flatten()
            .map(|x| x.url),
    }
}

#[derive(Deserialize)]
struct WqManifest {
    factors: Vec<WqManifestFactor>,
}

#[derive(Deserialize)]
struct WqManifestFactor {
    id: String,
    formula: String,
    summary: String,
    required_fields: Vec<String>,
}

fn wq_manifest() -> &'static WqManifest {
    static DATA: OnceLock<WqManifest> = OnceLock::new();
    DATA.get_or_init(|| {
        let parsed: WqManifest = serde_json::from_str(include_str!("alpha101_catalog.json"))
            .expect("checked WorldQuant Alpha101 manifest");
        assert_eq!(parsed.factors.len(), 101, "Alpha101 manifest count");
        for (index, factor) in parsed.factors.iter().enumerate() {
            assert_eq!(factor.id, format!("alpha{}", index + 1));
        }
        parsed
    })
}

fn qlib_catalog() -> Vec<Factor> {
    let mut out = Vec::with_capacity(158);
    for (id, f) in KBAR {
        out.push(qlib_factor(id, f.into()));
    }
    for (id, field) in [
        ("open0", "$open"),
        ("high0", "$high"),
        ("low0", "$low"),
        ("vwap0", "$vwap"),
    ] {
        out.push(qlib_factor(id, format!("{field}/$close")));
    }
    for family in ROLLING {
        for d in WINDOWS {
            out.push(qlib_factor(
                &format!("{family}{d}"),
                rolling_formula(family, d),
            ));
        }
    }
    debug_assert_eq!(out.len(), 158);
    out
}

fn qlib_factor(id: &str, formula: String) -> Factor {
    let missing = id == "vwap0";
    Factor {
        id: id.into(),
        name: id.to_ascii_uppercase(),
        status: if missing { "unavailable" } else { "available" },
        required_fields: required(id).into_iter().map(str::to_owned).collect(),
        summary: if missing {
            "需要真实成交额与成交量形成的 VWAP；不能用 OHLC 典型价替代，因此当前不可计算。".into()
        } else {
            summary(id)
        },
        formula: Some(formula),
        source_url: QLIB_SOURCE,
        code_url: (!missing)
            .then(|| crate::code_links::resolve("src/alpha_research.rs::qlib_alpha158"))
            .flatten()
            .map(|x| x.url),
    }
}

fn family(id: &str) -> &str {
    id.trim_end_matches(|c: char| c.is_ascii_digit())
}
fn required(id: &str) -> Vec<&'static str> {
    match family(id) {
        "open" => vec!["open", "close"],
        "high" | "max" | "imax" => vec!["high", "close"],
        "low" | "min" | "imin" => vec!["low", "close"],
        "vwap" => vec!["vwap", "close"],
        "rsv" | "imxd" => vec!["high", "low", "close"],
        "corr" | "cord" | "wvma" => vec!["close", "volume"],
        "vma" | "vstd" | "vsump" | "vsumn" | "vsumd" => vec!["volume"],
        "kmid" | "klen" | "kmid2" | "kup" | "kup2" | "klow" | "klow2" | "ksft" | "ksft2" => {
            vec!["open", "high", "low", "close"]
        }
        _ => vec!["close"],
    }
}
fn summary(id: &str) -> String {
    match family(id) {
        "kmid" => "实体涨跌幅：收盘相对开盘的变化。",
        "klen" => "整根 K 线振幅相对开盘价的比例。",
        "kmid2" => "实体长度占当日完整振幅的比例。",
        "kup" | "kup2" => "上影线长度及其缩放值。",
        "klow" | "klow2" => "下影线长度及其缩放值。",
        "ksft" | "ksft2" => "收盘在日内区间中的偏移。",
        "open" | "high" | "low" => "当日价格相对收盘价的比例。",
        "roc" => "窗口日前收盘价相对当前收盘价。",
        "ma" => "窗口平均收盘价相对当前收盘价。",
        "std" => "窗口收盘价样本标准差。",
        "beta" | "rsqr" | "resi" => "窗口收盘价对时间的线性趋势、拟合度或残差。",
        "max" | "min" | "qtlu" | "qtld" => "窗口价格极值或分位数相对当前收盘价。",
        "rank" | "rsv" => "当前价格在窗口历史分布或高低区间的位置。",
        "imax" | "imin" | "imxd" => "窗口极值出现位置及其相对先后。",
        "corr" | "cord" => "价格与成交量或两者变化的窗口相关性。",
        "cntp" | "cntn" | "cntd" => "窗口上涨、下跌天数占比及两者之差。",
        "sump" | "sumn" | "sumd" => "窗口涨跌额占总绝对变动的比例。",
        "vma" | "vstd" => "窗口成交量均值或样本标准差相对当前成交量。",
        "wvma" => "成交量加权绝对收益的变异系数。",
        "vsump" | "vsumn" | "vsumd" => "窗口成交量增减额占总绝对变化的比例。",
        _ => "Qlib Alpha158 默认特征。",
    }
    .into()
}

fn rolling_formula(f: &str, d: usize) -> String {
    match f {
    "roc" => format!("Ref($close,{d})/$close"), "ma" => format!("Mean($close,{d})/$close"),
    "std" => format!("Std($close,{d})/$close"), "beta" => format!("Slope($close,{d})/$close"),
    "rsqr" => format!("Rsquare($close,{d})"), "resi" => format!("Resi($close,{d})/$close"),
    "max" => format!("Max($high,{d})/$close"), "min" => format!("Min($low,{d})/$close"),
    "qtlu" => format!("Quantile($close,{d},0.8)/$close"), "qtld" => format!("Quantile($close,{d},0.2)/$close"),
    "rank" => format!("Rank($close,{d})"),
    "rsv" => format!("($close-Min($low,{d}))/(Max($high,{d})-Min($low,{d})+1e-12)"),
    "imax" => format!("IdxMax($high,{d})/{d}"), "imin" => format!("IdxMin($low,{d})/{d}"),
    "imxd" => format!("(IdxMax($high,{d})-IdxMin($low,{d}))/{d}"),
    "corr" => format!("Corr($close,Log($volume+1),{d})"),
    "cord" => format!("Corr($close/Ref($close,1),Log($volume/Ref($volume,1)+1),{d})"),
    "cntp" => format!("Mean($close>Ref($close,1),{d})"), "cntn" => format!("Mean($close<Ref($close,1),{d})"),
    "cntd" => format!("Mean($close>Ref($close,1),{d})-Mean($close<Ref($close,1),{d})"),
    "sump" => ratio_formula("$close", d, 1), "sumn" => ratio_formula("$close", d, -1),
    "sumd" => ratio_formula("$close", d, 0), "vma" => format!("Mean($volume,{d})/($volume+1e-12)"),
    "vstd" => format!("Std($volume,{d})/($volume+1e-12)"),
    "wvma" => format!("Std(Abs($close/Ref($close,1)-1)*$volume,{d})/(Mean(Abs($close/Ref($close,1)-1)*$volume,{d})+1e-12)"),
    "vsump" => ratio_formula("$volume", d, 1), "vsumn" => ratio_formula("$volume", d, -1),
    "vsumd" => ratio_formula("$volume", d, 0), _ => unreachable!(),
}
}
fn ratio_formula(x: &str, d: usize, dir: i8) -> String {
    let up = format!("Sum(Greater({x}-Ref({x},1),0),{d})");
    let down = format!("Sum(Greater(Ref({x},1)-{x},0),{d})");
    let den = format!("Sum(Abs({x}-Ref({x},1)),{d})+1e-12");
    match dir {
        1 => format!("{up}/({den})"),
        -1 => format!("{down}/({den})"),
        _ => format!("({up}-{down})/({den})"),
    }
}

pub fn available(collection: &str, id: &str) -> Option<Factor> {
    match collection {
        "worldquant101" if id == "alpha101" => Some(wq_factor(101)),
        "qlib_alpha158" => qlib_catalog()
            .into_iter()
            .find(|f| f.id == id && f.status == "available"),
        _ => None,
    }
}

#[derive(Clone, Serialize)]
pub struct Point {
    pub timestamp: String,
    pub value: Option<f64>,
}
#[derive(Serialize)]
pub struct Stats {
    pub paired_count: usize,
    pub time_series_correlation: Option<f64>,
}
#[derive(Serialize)]
pub struct Research {
    pub factor: Factor,
    pub series: Vec<Point>,
    pub forward_returns: Vec<Point>,
    pub stats: Stats,
    pub notes: Vec<&'static str>,
}

/// The label matches Qlib's `Ref($close,-2)/Ref($close,-1)-1` exactly.
pub fn research(collection: &str, id: &str, bars: &[Bar]) -> Result<Research, &'static str> {
    let factor = available(collection, id).ok_or("该因子尚未具备经过核验的真实行情计算")?;
    if bars.len() < 3 {
        return Err("需要至少三天连续、已收盘的真实日线");
    }
    crate::practice::validate_bars(bars).map_err(|_| "行情包含无效或乱序 K 线")?;
    let values = if collection == "worldquant101" {
        bars.iter().map(wq_101).collect()
    } else {
        qlib_alpha158(id, bars)?
    };
    let mut pairs = vec![];
    let forward_returns = bars
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let value = bars
                .get(i + 1)
                .zip(bars.get(i + 2))
                .and_then(|(a, z)| finite(z.close / a.close - 1.0));
            if let Some(pair) = values[i].zip(value) {
                pairs.push(pair);
            }
            Point {
                timestamp: b.timestamp.to_rfc3339(),
                value,
            }
        })
        .collect();
    let series = bars
        .iter()
        .zip(values)
        .map(|(b, value)| Point {
            timestamp: b.timestamp.to_rfc3339(),
            value,
        })
        .collect();
    Ok(Research {
        factor,
        series,
        forward_returns,
        stats: Stats {
            paired_count: pairs.len(),
            time_series_correlation: pair_correlation(&pairs),
        },
        notes: vec![
            "特征只使用信号日及以前的数据。",
            "标签采用 Qlib Ref(close,-2)/Ref(close,-1)-1。",
            "相关系数是单标的时间序列诊断，不是横截面 IC。",
        ],
    })
}

fn wq_101(b: &Bar) -> Option<f64> {
    finite((b.close - b.open) / (b.high - b.low + 0.001))
}

/// Single implementation seam for all executable Qlib Alpha158 features.
fn qlib_alpha158(id: &str, bars: &[Bar]) -> Result<Vec<Option<f64>>, &'static str> {
    if KBAR.iter().any(|(x, _)| *x == id) {
        return Ok(bars.iter().map(|b| kbar(id, b)).collect());
    }
    match id {
        "open0" => return Ok(bars.iter().map(|b| finite(b.open / b.close)).collect()),
        "high0" => return Ok(bars.iter().map(|b| finite(b.high / b.close)).collect()),
        "low0" => return Ok(bars.iter().map(|b| finite(b.low / b.close)).collect()),
        "vwap0" => return Err("缺少真实 VWAP，不能用典型价近似"),
        _ => {}
    }
    let (f, d) = parse_rolling(id).ok_or("未知的 Qlib Alpha158 特征")?;
    Ok(rolling(f, d, bars))
}
fn kbar(id: &str, b: &Bar) -> Option<f64> {
    let r = b.high - b.low;
    let top = b.open.max(b.close);
    let bot = b.open.min(b.close);
    finite(match id {
        "kmid" => (b.close - b.open) / b.open,
        "klen" => r / b.open,
        "kmid2" => (b.close - b.open) / (r + 1e-12),
        "kup" => (b.high - top) / b.open,
        "kup2" => (b.high - top) / (r + 1e-12),
        "klow" => (bot - b.low) / b.open,
        "klow2" => (bot - b.low) / (r + 1e-12),
        "ksft" => (2.0 * b.close - b.high - b.low) / b.open,
        "ksft2" => (2.0 * b.close - b.high - b.low) / (r + 1e-12),
        _ => unreachable!(),
    })
}
fn parse_rolling(id: &str) -> Option<(&'static str, usize)> {
    for f in ROLLING {
        if let Some(s) = id.strip_prefix(f) {
            if let Ok(d) = s.parse() {
                if WINDOWS.contains(&d) && id == format!("{f}{d}") {
                    return Some((f, d));
                }
            }
        }
    }
    None
}

fn rolling(f: &str, d: usize, bars: &[Bar]) -> Vec<Option<f64>> {
    let c: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let h: Vec<f64> = bars.iter().map(|b| b.high).collect();
    let l: Vec<f64> = bars.iter().map(|b| b.low).collect();
    let v: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let dc = lag(&c, |a, b| a - b);
    let dv = lag(&v, |a, b| a - b);
    let cr = lag(&c, |a, b| a / b);
    let vlr = lag(&v, |a, b| (a / b + 1.0).ln());
    let wr: Vec<Option<f64>> = cr
        .iter()
        .zip(&v)
        .map(|(r, v)| r.and_then(|x| finite((x - 1.0).abs() * v)))
        .collect();
    (0..bars.len())
        .map(|i| {
            let s = (i + 1).saturating_sub(d);
            let cs = &c[s..=i];
            let hs = &h[s..=i];
            let ls = &l[s..=i];
            let vs = &v[s..=i];
            let ch = opts(&dc[s..=i]);
            let vh = opts(&dv[s..=i]);
            let weights = opts(&wr[s..=i]);
            match f {
                "roc" => i.checked_sub(d).and_then(|j| finite(c[j] / c[i])),
                "ma" => finite(mean(cs)? / c[i]),
                "std" => finite(std(cs)? / c[i]),
                "beta" => finite(reg(cs)?.0 / c[i]),
                "rsqr" => reg(cs).and_then(|x| x.1),
                "resi" => reg(cs).and_then(|x| finite(x.2 / c[i])),
                "max" => finite(max(hs) / c[i]),
                "min" => finite(min(ls) / c[i]),
                "qtlu" => finite(quantile(cs, 0.8)? / c[i]),
                "qtld" => finite(quantile(cs, 0.2)? / c[i]),
                "rank" => rank(cs),
                "rsv" => finite((c[i] - min(ls)) / (max(hs) - min(ls) + 1e-12)),
                "imax" => finite((arg(hs, true) + 1) as f64 / d as f64),
                "imin" => finite((arg(ls, false) + 1) as f64 / d as f64),
                "imxd" => finite((arg(hs, true) as f64 - arg(ls, false) as f64) / d as f64),
                "corr" => {
                    let logs: Vec<f64> = vs.iter().map(|x| (x + 1.0).ln()).collect();
                    corr(cs, &logs)
                }
                "cord" => {
                    let pairs = cr[s..=i]
                        .iter()
                        .zip(&vlr[s..=i])
                        .filter_map(|(a, b)| a.zip(*b))
                        .collect::<Vec<_>>();
                    let left = pairs.iter().map(|(a, _)| *a).collect::<Vec<_>>();
                    let right = pairs.iter().map(|(_, b)| *b).collect::<Vec<_>>();
                    corr(&left, &right)
                }
                "cntp" => count_options(&dc[s..=i], 1),
                "cntn" => count_options(&dc[s..=i], -1),
                "cntd" => count_options(&dc[s..=i], 0),
                "sump" => change_ratio(&ch, 1),
                "sumn" => change_ratio(&ch, -1),
                "sumd" => change_ratio(&ch, 0),
                "vma" => finite(mean(vs)? / (v[i] + 1e-12)),
                "vstd" => finite(std(vs)? / (v[i] + 1e-12)),
                "wvma" => finite(std(&weights)? / (mean(&weights)? + 1e-12)),
                "vsump" => change_ratio(&vh, 1),
                "vsumn" => change_ratio(&vh, -1),
                "vsumd" => change_ratio(&vh, 0),
                _ => unreachable!(),
            }
        })
        .collect()
}
fn lag(x: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<Option<f64>> {
    let mut o = vec![None];
    o.extend((1..x.len()).map(|i| finite(f(x[i], x[i - 1]))));
    o
}
fn opts(x: &[Option<f64>]) -> Vec<f64> {
    x.iter().flatten().copied().collect()
}
fn mean(x: &[f64]) -> Option<f64> {
    (!x.is_empty()).then(|| x.iter().sum::<f64>() / x.len() as f64)
}
fn std(x: &[f64]) -> Option<f64> {
    if x.len() < 2 {
        return None;
    }
    let m = mean(x)?;
    finite((x.iter().map(|z| (z - m).powi(2)).sum::<f64>() / (x.len() - 1) as f64).sqrt())
}
fn max(x: &[f64]) -> f64 {
    x.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
fn min(x: &[f64]) -> f64 {
    x.iter().copied().fold(f64::INFINITY, f64::min)
}
fn reg(y: &[f64]) -> Option<(f64, Option<f64>, f64)> {
    if y.len() < 2 {
        return None;
    }
    let n = y.len() as f64;
    let xm = (n + 1.0) / 2.0;
    let ym = mean(y)?;
    let xx: f64 = (0..y.len()).map(|i| (i as f64 + 1.0 - xm).powi(2)).sum();
    let yy: f64 = y.iter().map(|z| (z - ym).powi(2)).sum();
    let xy: f64 = y
        .iter()
        .enumerate()
        .map(|(i, z)| (i as f64 + 1.0 - xm) * (z - ym))
        .sum();
    let b = xy / xx;
    let rsquare = ((yy / (y.len() - 1) as f64).sqrt() > 2e-5).then(|| xy * xy / (xx * yy));
    Some((b, rsquare, y[y.len() - 1] - (ym + b * (n - xm))))
}
fn quantile(x: &[f64], q: f64) -> Option<f64> {
    let mut z = x.to_vec();
    z.sort_by(f64::total_cmp);
    let p = (z.len() - 1) as f64 * q;
    let a = p.floor() as usize;
    let b = p.ceil() as usize;
    finite(z[a] + (z[b] - z[a]) * (p - a as f64))
}
fn rank(x: &[f64]) -> Option<f64> {
    let z = *x.last()?;
    let a = x.iter().filter(|v| **v < z).count() as f64;
    let e = x.iter().filter(|v| **v == z).count() as f64;
    finite((a + (e + 1.0) / 2.0) / x.len() as f64)
}
fn arg(x: &[f64], hi: bool) -> usize {
    let mut b = 0;
    for i in 1..x.len() {
        if (hi && x[i] > x[b]) || (!hi && x[i] < x[b]) {
            b = i
        }
    }
    b
}
fn count_options(x: &[Option<f64>], dir: i8) -> Option<f64> {
    if x.is_empty() {
        return None;
    }
    // Qlib comparisons turn the initial Ref(...)=NaN into false; unlike sums,
    // that false remains part of pandas' rolling mean denominator.
    let p = x.iter().flatten().filter(|z| **z > 0.0).count() as f64;
    let n = x.iter().flatten().filter(|z| **z < 0.0).count() as f64;
    finite(
        match dir {
            1 => p,
            -1 => n,
            _ => p - n,
        } / x.len() as f64,
    )
}
fn change_ratio(x: &[f64], dir: i8) -> Option<f64> {
    if x.is_empty() {
        return None;
    }
    let p: f64 = x.iter().map(|z| z.max(0.0)).sum();
    let n: f64 = x.iter().map(|z| (-z).max(0.0)).sum();
    finite(
        match dir {
            1 => p,
            -1 => n,
            _ => p - n,
        } / (p + n + 1e-12),
    )
}
fn finite(x: f64) -> Option<f64> {
    x.is_finite().then_some(x)
}
fn corr(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    let a = mean(x)?;
    let b = mean(y)?;
    let xx: f64 = x.iter().map(|z| (z - a).powi(2)).sum();
    let yy: f64 = y.iter().map(|z| (z - b).powi(2)).sum();
    if (xx / (x.len() - 1) as f64).sqrt() <= 2e-5 || (yy / (y.len() - 1) as f64).sqrt() <= 2e-5 {
        return None;
    }
    finite(x.iter().zip(y).map(|(i, j)| (i - a) * (j - b)).sum::<f64>() / (xx * yy).sqrt())
}
fn pair_correlation(p: &[(f64, f64)]) -> Option<f64> {
    if p.len() < 3 {
        return None;
    }
    let x: Vec<f64> = p.iter().map(|z| z.0).collect();
    let y: Vec<f64> = p.iter().map(|z| z.1).collect();
    corr(&x, &y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let b = 100.0 + i as f64;
                Bar {
                    timestamp: Utc.timestamp_opt(i as i64 * 86400, 0).unwrap(),
                    open: b,
                    high: b + 3.0,
                    low: b - 2.0,
                    close: b + (i % 3) as f64,
                    volume: 1000.0 + (i * i) as f64,
                }
            })
            .collect()
    }
    #[test]
    fn catalog_is_canonical() {
        let c = catalog();
        assert_eq!((c[0].total, c[1].total), (101, 158));
        let q = &c[1].factors;
        assert_eq!(q.len(), 158);
        assert_eq!(q[0].id, "kmid");
        assert_eq!(q[12].id, "vwap0");
        assert_eq!(q[13].id, "roc5");
        assert_eq!(q[157].id, "vsumd60");
        assert_eq!(q.iter().filter(|f| f.status == "available").count(), 157);
        assert!(available("qlib_alpha158", "vwap0").is_none());
    }
    #[test]
    fn kmid_and_label_are_golden() {
        let b = bars(5);
        let r = research("qlib_alpha158", "kmid", &b).unwrap();
        assert_eq!(r.series[0].value, Some(0.0));
        assert_eq!(
            r.forward_returns[0].value,
            Some(b[2].close / b[1].close - 1.0)
        );
        assert!(r.factor.code_url.is_some());
    }
    #[test]
    fn rolling_literals_match_qlib() {
        let b = bars(8);
        let ma = research("qlib_alpha158", "ma5", &b).unwrap();
        assert_eq!(ma.series[0].value, Some(1.0));
        let e = b[..=3].iter().map(|z| z.close).sum::<f64>() / 4.0 / b[3].close;
        assert!((ma.series[3].value.unwrap() - e).abs() < 1e-12);
        let s = research("qlib_alpha158", "std5", &b).unwrap();
        assert!(s.series[0].value.is_none());
        let vals = [b[0].close, b[1].close];
        assert!((s.series[1].value.unwrap() - std(&vals).unwrap() / b[1].close).abs() < 1e-12);
        let roc = research("qlib_alpha158", "roc5", &b).unwrap();
        assert!(roc.series[..5].iter().all(|p| p.value.is_none()));
        assert_eq!(roc.series[5].value, Some(b[0].close / b[5].close));
        let cntp = research("qlib_alpha158", "cntp5", &b).unwrap();
        assert_eq!(cntp.series[0].value, Some(0.0));
        assert_eq!(cntp.series[1].value, Some(0.5));
    }
    #[test]
    fn constant_price_preserves_qlib_slope_and_residual_but_undefined_rsquare() {
        let mut b = bars(8);
        for row in &mut b {
            row.open = 100.0;
            row.high = 101.0;
            row.low = 99.0;
            row.close = 100.0;
        }
        let beta = research("qlib_alpha158", "beta5", &b).unwrap();
        let resi = research("qlib_alpha158", "resi5", &b).unwrap();
        let rsqr = research("qlib_alpha158", "rsqr5", &b).unwrap();
        assert_eq!(beta.series[4].value, Some(0.0));
        assert_eq!(resi.series[4].value, Some(0.0));
        assert_eq!(rsqr.series[4].value, None);
    }
    #[test]
    fn all_available_are_prefix_causal() {
        let a = bars(70);
        let b = bars(75);
        for f in qlib_catalog()
            .into_iter()
            .filter(|f| f.status == "available")
        {
            let x =
                research("qlib_alpha158", &f.id, &a).unwrap_or_else(|e| panic!("{}: {e}", f.id));
            let y =
                research("qlib_alpha158", &f.id, &b).unwrap_or_else(|e| panic!("{}: {e}", f.id));
            for i in 0..70 {
                match (x.series[i].value, y.series[i].value) {
                    (Some(u), Some(v)) => assert!((u - v).abs() < 1e-12, "{} {i}", f.id),
                    (None, None) => {}
                    z => panic!("{} {i}: {z:?}", f.id),
                }
            }
        }
    }
    #[test]
    fn ids_and_vwap_fail_closed() {
        assert!(available("qlib_alpha158", "KMID").is_none());
        assert!(available("qlib_alpha158", "roc05").is_none());
        assert!(research("qlib_alpha158", "vwap0", &bars(10)).is_err());
        assert!(available("worldquant101", "alpha101").is_some());
        assert!(available("worldquant101", "alpha1").is_none());
    }
}
