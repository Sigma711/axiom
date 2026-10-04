//! Auditable market-breadth snapshot over one fixed constituent universe.
//!
//! Daily member OHLCV supports cross-sectional close/volume measures and an
//! explicitly parameterized close-only Point & Figure BPI. Exchange TICK needs
//! separate intraday trade observations and is attached from `market_tick`.

use crate::{
    data::{fetch_public_market_snapshot, HttpFeed, MarketProvenance, StockSplitCoverage},
    indicators::{breadth, ma},
    types::Bar,
};
use chrono::{Duration, NaiveDate, Utc};
use futures::{stream, StreamExt, TryStreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
    time::{Duration as StdDuration, Instant},
};
use tokio::sync::Mutex;

pub const DOW_30: [&str; 30] = [
    "AAPL", "AMGN", "AMZN", "AXP", "BA", "CAT", "CRM", "CSCO", "CVX", "DIS", "GS", "HD", "HON",
    "IBM", "JNJ", "JPM", "KO", "MCD", "MMM", "MRK", "MSFT", "NKE", "NVDA", "PG", "SHW", "TRV",
    "UNH", "V", "VZ", "WMT",
];
pub const MARKET_BREADTH_IDS: [&str; 8] = [
    "ad_line",
    "breadth_thrust",
    "bullish_percent",
    "mcclellan",
    "new_high_low",
    "tick",
    "trin",
    "up_down_volume",
];
pub const MINIMUM_MEMBERS: usize = 27;

pub fn is_practice_supported(id: &str) -> bool {
    MARKET_BREADTH_IDS.contains(&id) || id == "book_mcclellan_sum"
}

#[derive(Debug, Clone)]
pub struct MemberDailySeries {
    pub symbol: String,
    pub bars: Vec<Bar>,
    pub provenance: MarketProvenance,
    pub split_event_dates: Vec<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UniverseContract {
    pub id: &'static str,
    pub name: &'static str,
    pub member_count: usize,
    pub constituents_as_of: &'static str,
    pub constituents_source: &'static str,
    pub symbols: Vec<&'static str>,
    pub scope_note: &'static str,
    pub calendar: &'static str,
    pub missing_policy: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageSummary {
    pub latest_eligible_members: usize,
    pub total_members: usize,
    pub minimum_required: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceMember {
    pub symbol: String,
    pub provider: String,
    pub endpoint: String,
    pub price_basis: String,
    pub corporate_actions: String,
    pub split_event_dates: Vec<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSummary {
    pub retrieval: &'static str,
    pub retrieved_at: chrono::DateTime<Utc>,
    pub cache_status: String,
    pub members: Vec<SourceMember>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreadthObservation {
    pub date: NaiveDate,
    pub eligible_members: usize,
    pub advances: usize,
    pub declines: usize,
    pub unchanged: usize,
    pub up_volume: f64,
    pub down_volume: f64,
    pub new_highs: Option<usize>,
    pub new_lows: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreadthPoint {
    pub date: NaiveDate,
    pub value: f64,
    pub eligible_members: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreadthConcept {
    pub id: &'static str,
    pub status: &'static str,
    pub unit: &'static str,
    pub latest: Option<f64>,
    pub triggered: Option<bool>,
    pub series: Vec<BreadthPoint>,
    pub reason: Option<String>,
    pub definition: &'static str,
    pub inputs: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketBreadthSnapshot {
    pub schema_version: u8,
    pub as_of: NaiveDate,
    pub universe: UniverseContract,
    pub coverage: CoverageSummary,
    pub source: SourceSummary,
    pub observations: Vec<BreadthObservation>,
    pub concepts: Vec<BreadthConcept>,
}

impl MarketBreadthSnapshot {
    /// Attach the independently sourced intraday crypto-basket TICK result.
    /// Daily breadth remains usable when the intraday source fails, while the
    /// TICK card keeps the exact provider failure instead of fabricating a value.
    pub fn attach_crypto_tick(
        mut self,
        result: Result<crate::market_tick::CryptoTickSnapshot, String>,
    ) -> Self {
        let tick = self
            .concepts
            .iter_mut()
            .find(|concept| concept.id == "tick")
            .expect("the fixed contract always contains tick");
        match result {
            Ok(snapshot) => {
                tick.status = "available";
                tick.latest = Some(snapshot.net_tick as f64);
                tick.reason = None;
                tick.definition = "Fixed BTCUSDT/ETHUSDT/BNBUSDT Binance Spot basket: latest aggregate-trade price direction per member at one common UTC cutoff; this is not NYSE TICK.";
                tick.inputs = serde_json::to_value(snapshot)
                    .expect("CryptoTickSnapshot serialization is infallible");
            }
            Err(error) => {
                tick.reason = Some(format!("Real intraday crypto TICK unavailable: {error}"));
            }
        }
        self
    }
}

/// Fetch the fixed historical basket from the existing real US-stock gateway.
/// Requests are bounded to six concurrent members to avoid turning one page view
/// into an unbounded burst against the public providers.
pub async fn fetch_live_snapshot(feed: &HttpFeed) -> Result<MarketBreadthSnapshot, String> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, (Instant, MarketBreadthSnapshot)>>> =
        OnceLock::new();
    let cache_key = format!(
        "{}|{}|{}|dow30-2024-11-08|600",
        feed.us_stock_adjustment_url,
        feed.us_stock_adjustment_fallback_url,
        feed.us_stock_relay_url.as_deref().unwrap_or("")
    );
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut guard = cache.lock().await;
    if let Some((stored_at, snapshot)) = guard.get(&cache_key) {
        if stored_at.elapsed() < StdDuration::from_secs(15 * 60) {
            let mut cached = snapshot.clone();
            cached.source.cache_status = "memory_cache_15m".into();
            return Ok(cached);
        }
    }
    let since = Utc::now() - Duration::days(1_100);
    let requests: Vec<_> = DOW_30
        .into_iter()
        .map(|symbol| fetch_member(feed, symbol, since))
        .collect();
    let members = stream::iter(requests)
        .buffer_unordered(6)
        .try_collect()
        .await?;
    let snapshot = build_snapshot(members)?;
    guard.insert(cache_key, (Instant::now(), snapshot.clone()));
    Ok(snapshot)
}

async fn fetch_member(
    feed: &HttpFeed,
    symbol: &'static str,
    since: chrono::DateTime<Utc>,
) -> Result<MemberDailySeries, String> {
    let snapshot = fetch_public_market_snapshot(feed, "us_stock", symbol, since, 600)
        .await
        .map_err(|error| format!("{symbol}: {error:#}"))?;
    let split_event_dates = match &snapshot.stock_split_coverage {
        StockSplitCoverage::CrossesWindow { event_dates, .. } => event_dates.clone(),
        _ => Vec::new(),
    };
    let mut provenance = snapshot.provenance;
    provenance.corporate_actions = if split_event_dates.is_empty() {
        "provider events report no split in returned window".into()
    } else {
        "provider split events disclosed; returned quote continuity is used as provider-restated history, with adjustment semantics not independently verified".into()
    };
    Ok(MemberDailySeries {
        symbol: symbol.to_owned(),
        bars: snapshot.bars,
        provenance,
        split_event_dates,
    })
}

#[derive(Default)]
struct DailyAccumulator {
    eligible: usize,
    advances: usize,
    declines: usize,
    unchanged: usize,
    up_volume: f64,
    down_volume: f64,
    high_low_eligible: usize,
    new_highs: usize,
    new_lows: usize,
}

pub fn calculate_ad_line(observations: &[BreadthObservation]) -> Vec<f64> {
    let advances: Vec<_> = observations.iter().map(|row| row.advances as f64).collect();
    let declines: Vec<_> = observations.iter().map(|row| row.declines as f64).collect();
    breadth::market_advance_decline_line(&advances, &declines, 0.0)
}

pub fn calculate_trin(observation: &BreadthObservation) -> Option<f64> {
    (observation.declines > 0 && observation.down_volume > 0.0 && observation.up_volume > 0.0).then(
        || {
            breadth::trin(
                observation.advances as f64,
                observation.declines as f64,
                observation.up_volume,
                observation.down_volume,
            )
        },
    )
}

pub fn calculate_mcclellan(observations: &[BreadthObservation]) -> Vec<Option<f64>> {
    let net: Vec<_> = observations
        .iter()
        .map(|row| row.advances as f64 - row.declines as f64)
        .collect();
    breadth::mcclellan_oscillator(&net)
}

pub fn calculate_mcclellan_summation(series: &[BreadthPoint]) -> Vec<BreadthPoint> {
    let mut total = 0.0;
    series
        .iter()
        .map(|point| {
            total += point.value;
            BreadthPoint {
                date: point.date,
                value: total,
                eligible_members: point.eligible_members,
            }
        })
        .collect()
}

pub fn calculate_new_high_low(observation: &BreadthObservation) -> Option<f64> {
    observation
        .new_highs
        .zip(observation.new_lows)
        .map(|(highs, lows)| highs as f64 - lows as f64)
}

pub type BreadthThrustCalculation = (Vec<Option<f64>>, Vec<Option<f64>>, bool);

pub fn calculate_breadth_thrust(
    observations: &[BreadthObservation],
) -> Option<BreadthThrustCalculation> {
    let fractions: Vec<_> = observations
        .iter()
        .map(|row| {
            let directional = row.advances + row.declines;
            (directional > 0).then(|| row.advances as f64 / directional as f64)
        })
        .collect();
    let mut ema = vec![None; fractions.len()];
    let mut start = 0;
    while start < fractions.len() {
        while start < fractions.len() && fractions[start].is_none() {
            start += 1;
        }
        let mut end = start;
        while end < fractions.len() && fractions[end].is_some() {
            end += 1;
        }
        if start < end {
            let run: Vec<_> = fractions[start..end].iter().flatten().copied().collect();
            ema[start..end].copy_from_slice(&ma::ema(&run, 10));
        }
        start = end;
    }
    let latest_run: Vec<_> = fractions
        .iter()
        .rev()
        .take_while(|value| value.is_some())
        .flatten()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    (latest_run.len() >= 10).then(|| {
        let triggered = breadth::breadth_thrust(&latest_run);
        (fractions, ema, triggered)
    })
}

pub fn calculate_bullish_percent(
    members: &[MemberDailySeries],
    as_of: NaiveDate,
) -> (usize, usize, Option<f64>) {
    let signals: Vec<_> = members
        .iter()
        .filter(|member| {
            member
                .bars
                .iter()
                .any(|bar| bar.timestamp.date_naive() == as_of)
        })
        .filter_map(|member| {
            let closes: Vec<_> = member
                .bars
                .iter()
                .take_while(|bar| bar.timestamp.date_naive() <= as_of)
                .map(|bar| bar.close)
                .collect();
            breadth::point_and_figure_buy_signal(&closes, 0.01, 3)
        })
        .collect();
    let buys = signals.iter().filter(|signal| **signal).count();
    let eligible = signals.len();
    let value = (eligible >= MINIMUM_MEMBERS)
        .then(|| breadth::bullish_percent_index(buys as f64, eligible as f64));
    (buys, eligible, value)
}

pub fn calculate_up_down_volume(observation: &BreadthObservation) -> Option<f64> {
    (observation.down_volume > 0.0)
        .then(|| breadth::up_down_volume_ratio(observation.up_volume, observation.down_volume))
}

pub fn build_snapshot(members: Vec<MemberDailySeries>) -> Result<MarketBreadthSnapshot, String> {
    let expected: BTreeSet<_> = DOW_30.into_iter().collect();
    let actual: BTreeSet<_> = members
        .iter()
        .map(|member| member.symbol.as_str())
        .collect();
    if members.len() != DOW_30.len() || actual != expected {
        return Err(
            "market breadth requires every member of the fixed universe exactly once".into(),
        );
    }
    let mut daily: BTreeMap<NaiveDate, DailyAccumulator> = BTreeMap::new();
    for member in &members {
        crate::practice::validate_bars(&member.bars)
            .map_err(|error| format!("{}: {error}", member.symbol))?;
        for index in 1..member.bars.len() {
            let previous = member.bars[index - 1];
            let current = member.bars[index];
            let row = daily.entry(current.timestamp.date_naive()).or_default();
            row.eligible += 1;
            if current.close > previous.close {
                row.advances += 1;
                row.up_volume += current.volume;
            } else if current.close < previous.close {
                row.declines += 1;
                row.down_volume += current.volume;
            } else {
                row.unchanged += 1;
            }
            if index >= 252 {
                row.high_low_eligible += 1;
                let history = &member.bars[index - 252..index];
                if current.high
                    >= history
                        .iter()
                        .map(|bar| bar.high)
                        .fold(f64::NEG_INFINITY, f64::max)
                {
                    row.new_highs += 1;
                }
                if current.low
                    <= history
                        .iter()
                        .map(|bar| bar.low)
                        .fold(f64::INFINITY, f64::min)
                {
                    row.new_lows += 1;
                }
            }
        }
    }

    let observations: Vec<_> = daily
        .into_iter()
        .filter(|(_, row)| row.eligible >= MINIMUM_MEMBERS)
        .map(|(date, row)| BreadthObservation {
            date,
            eligible_members: row.eligible,
            advances: row.advances,
            declines: row.declines,
            unchanged: row.unchanged,
            up_volume: row.up_volume,
            down_volume: row.down_volume,
            new_highs: (row.high_low_eligible >= MINIMUM_MEMBERS).then_some(row.new_highs),
            new_lows: (row.high_low_eligible >= MINIMUM_MEMBERS).then_some(row.new_lows),
        })
        .collect();
    let latest = observations.last().ok_or_else(|| {
        "no provider-observed session met the fixed-universe 90% coverage rule".to_owned()
    })?;
    let (point_figure_buys, point_figure_eligible, bullish_percent) =
        calculate_bullish_percent(&members, latest.date);

    let dates: Vec<_> = observations.iter().map(|row| row.date).collect();
    let coverages: Vec<_> = observations
        .iter()
        .map(|row| row.eligible_members)
        .collect();
    let ad = calculate_ad_line(&observations);
    let mcclellan = calculate_mcclellan(&observations);
    let thrust = calculate_breadth_thrust(&observations);
    let points = |values: &[Option<f64>]| {
        values
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                value.map(|value| BreadthPoint {
                    date: dates[index],
                    value,
                    eligible_members: coverages[index],
                })
            })
            .collect()
    };
    let direct_points =
        |values: &[f64]| points(&values.iter().copied().map(Some).collect::<Vec<_>>());
    let trin = calculate_trin(latest);
    let up_down = calculate_up_down_volume(latest);
    let new_high_low = calculate_new_high_low(latest);

    let concepts = vec![
        available("ad_line", "issues", ad.last().copied(), None, direct_points(&ad), "Cumulative daily advances minus declines, based at zero on the first eligible session in this response window.", json!({"initial_value":0.0})),
        conditional("trin", "ratio", trin, "(advances/declines)/(up volume/down volume).", json!({"advances":latest.advances,"declines":latest.declines,"up_volume":latest.up_volume,"down_volume":latest.down_volume}), "TRIN needs nonzero advances, declines, up volume and down volume."),
        available("mcclellan", "issues", mcclellan.last().copied().flatten(), None, points(&mcclellan), "EMA(19) minus EMA(39) of daily net advances; undefined during warmup.", json!({"fast_period":19,"slow_period":39})),
        conditional("new_high_low", "issues", new_high_low, "New 252-session highs minus new 252-session lows among members with full provider-restated quote history.", json!({"new_highs":latest.new_highs,"new_lows":latest.new_lows,"lookback_sessions":252}), "At least 27 members need 252 preceding observations."),
        unavailable("tick", "issues", "A daily OHLCV source has no contemporaneous intraday uptick/downtick observations; day-over-day direction is not an exchange TICK.", "Contemporaneous member upticks minus downticks at one intraday sampling instant."),
        match thrust.as_ref() {
            Some((fractions, thrust_ema, triggered)) => available("breadth_thrust", "fraction", thrust_ema.last().copied().flatten(), Some(*triggered), points(thrust_ema), "Zweig: 10-session EMA of advances/(advances+declines), rising from below 0.40 to above 0.615 within 10 observations. A session with no directional member is undefined and resets the EMA warmup.", json!({"ema_period":10,"low_threshold":0.40,"high_threshold":0.615,"latest_advance_fraction":fractions.last().copied().flatten()})),
            None => BreadthConcept { id:"breadth_thrust", status:"unavailable", unit:"fraction", latest:None, triggered:None, series:Vec::new(), reason:Some("The latest contiguous run after an undefined all-unchanged session has fewer than 10 observations.".into()), definition:"Zweig: 10-session EMA of advances/(advances+declines), rising from below 0.40 to above 0.615 within 10 observations. A session with no directional member is undefined and resets the EMA warmup.", inputs:json!({"ema_period":10,"low_threshold":0.40,"high_threshold":0.615}) },
        },
        conditional("bullish_percent", "percent", bullish_percent, "Members whose close-only logarithmic 1% box, 3-box-reversal Point & Figure state is a persistent buy signal, divided by members with a determinate signal, times 100.", json!({"buy_signals":point_figure_buys,"eligible_signals":point_figure_eligible,"box_method":"logarithmic 1% close-only","reversal_boxes":3}), "At least 27 members need enough history to establish a persistent Point & Figure buy or sell signal; indeterminate members are not counted as sells."),
        conditional("up_down_volume", "ratio", up_down, "Volume of advancing members divided by volume of declining members.", json!({"up_volume":latest.up_volume,"down_volume":latest.down_volume}), "Up/down volume ratio needs nonzero declining volume."),
    ];
    let source = SourceSummary {
        retrieval: "public US-stock daily OHLCV requested for this response; actual provider and endpoint disclosed per member",
        retrieved_at: Utc::now(),
        cache_status: "fetched_live".into(),
        members: members
            .into_iter()
            .map(|member| SourceMember {
                symbol: member.symbol,
                provider: member.provenance.provider,
                endpoint: member.provenance.endpoint,
                price_basis: member.provenance.price_basis,
                corporate_actions: member.provenance.corporate_actions,
                split_event_dates: member.split_event_dates,
            })
            .collect(),
    };
    Ok(MarketBreadthSnapshot {
        schema_version: 1,
        as_of: latest.date,
        universe: UniverseContract {
            id: "dow_30_2024_11_08",
            name: "Fixed 2024-11-08 Dow 30 constituent basket",
            member_count: DOW_30.len(),
            constituents_as_of: "2024-11-08",
            constituents_source: "https://press.spglobal.com/2024-11-01-NVIDIA-and-Sherwin-Williams-Set-to-Join-Dow-Jones-Industrial-Average-Vistra-to-Join-Dow-Jones-Utility-Average",
            symbols: DOW_30.to_vec(),
            scope_note: "The same 2024-11-08 member list is applied retrospectively to every displayed date. Dates before or after that effective date are fixed-basket history, not historical index membership or an official DJIA breadth series.",
            calendar: "union of provider-observed member sessions; retain a date only when at least 27 of 30 members have current and preceding daily bars",
            missing_policy: "exclude a missing member from that session; never count it as unchanged or declining; drop the entire session below 90% coverage",
        },
        coverage: CoverageSummary {
            latest_eligible_members: latest.eligible_members,
            total_members: DOW_30.len(),
            minimum_required: MINIMUM_MEMBERS,
        },
        source,
        observations,
        concepts,
    })
}

fn available(
    id: &'static str,
    unit: &'static str,
    latest: Option<f64>,
    triggered: Option<bool>,
    series: Vec<BreadthPoint>,
    definition: &'static str,
    inputs: Value,
) -> BreadthConcept {
    BreadthConcept {
        id,
        status: "available",
        unit,
        latest,
        triggered,
        series,
        reason: None,
        definition,
        inputs,
    }
}

fn conditional(
    id: &'static str,
    unit: &'static str,
    latest: Option<f64>,
    definition: &'static str,
    inputs: Value,
    missing_reason: &'static str,
) -> BreadthConcept {
    match latest {
        Some(value) => available(id, unit, Some(value), None, Vec::new(), definition, inputs),
        None => BreadthConcept {
            id,
            status: "unavailable",
            unit,
            latest: None,
            triggered: None,
            series: Vec::new(),
            reason: Some(missing_reason.into()),
            definition,
            inputs,
        },
    }
}

fn unavailable(
    id: &'static str,
    unit: &'static str,
    reason: &'static str,
    definition: &'static str,
) -> BreadthConcept {
    BreadthConcept {
        id,
        status: "unavailable",
        unit,
        latest: None,
        triggered: None,
        series: Vec::new(),
        reason: Some(reason.into()),
        definition,
        inputs: json!({}),
    }
}
