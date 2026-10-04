//! A fixed Binance spot basket sampled from executed trades at one UTC cutoff.
//! This is a small-universe TICK demonstration, not the NYSE TICK index.

use crate::data::HttpFeed;
use chrono::{DateTime, Duration, TimeZone, Utc};
use futures::future::try_join_all;
use serde::Serialize;
use serde_json::Value;

pub const CRYPTO_TICK_UNIVERSE: [&str; 3] = ["BTCUSDT", "ETHUSDT", "BNBUSDT"];
const MAX_TRADE_AGE_MS: i64 = 30_000;

#[derive(Debug, Clone, Serialize)]
pub struct CryptoTickMember {
    pub symbol: String,
    pub previous_at: DateTime<Utc>,
    pub latest_at: DateTime<Utc>,
    pub previous_price: f64,
    pub latest_price: f64,
    pub direction: &'static str,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryptoTickSnapshot {
    pub market: &'static str,
    pub definition: &'static str,
    pub sample_at: DateTime<Utc>,
    pub universe: Vec<String>,
    pub members: Vec<CryptoTickMember>,
    pub up_ticks: usize,
    pub down_ticks: usize,
    pub unchanged_ticks: usize,
    pub net_tick: i64,
}

fn snapshot_is_consistent(snapshot: &CryptoTickSnapshot) -> bool {
    let expected_universe: Vec<_> = CRYPTO_TICK_UNIVERSE
        .iter()
        .map(|symbol| (*symbol).to_owned())
        .collect();
    let member_symbols: Vec<_> = snapshot
        .members
        .iter()
        .map(|member| member.symbol.clone())
        .collect();
    let member_count_matches = snapshot.members.len() == CRYPTO_TICK_UNIVERSE.len();
    let universe_matches = snapshot.universe == expected_universe;
    let member_order_matches = member_symbols == expected_universe;
    let counts_reconcile = snapshot.up_ticks + snapshot.down_ticks + snapshot.unchanged_ticks
        == snapshot.members.len();
    let net_reconciles = snapshot.net_tick == snapshot.up_ticks as i64 - snapshot.down_ticks as i64;
    let timestamps_ordered = snapshot
        .members
        .iter()
        .all(|member| member.previous_at <= member.latest_at);
    let timestamps_at_cutoff = snapshot
        .members
        .iter()
        .all(|member| member.latest_at <= snapshot.sample_at);
    let prices_finite = snapshot
        .members
        .iter()
        .all(|member| member.previous_price.is_finite() && member.latest_price.is_finite());
    let prices_positive = snapshot
        .members
        .iter()
        .all(|member| member.previous_price > 0.0 && member.latest_price > 0.0);
    let directions_known = snapshot
        .members
        .iter()
        .all(|member| matches!(member.direction, "up" | "down" | "unchanged"));
    let up_reconciles = snapshot
        .members
        .iter()
        .filter(|member| member.latest_price > member.previous_price)
        .count()
        == snapshot.up_ticks;
    let down_reconciles = snapshot
        .members
        .iter()
        .filter(|member| member.latest_price < member.previous_price)
        .count()
        == snapshot.down_ticks;
    let unchanged_reconciles = snapshot
        .members
        .iter()
        .filter(|member| member.latest_price == member.previous_price)
        .count()
        == snapshot.unchanged_ticks;
    let market_named = snapshot.market == "binance_spot_usdt_fixed_three";
    let definition_present = !snapshot.definition.trim().is_empty();
    member_count_matches
        && universe_matches
        && member_order_matches
        && counts_reconcile
        && net_reconciles
        && timestamps_ordered
        && timestamps_at_cutoff
        && prices_finite
        && prices_positive
        && directions_known
        && up_reconciles
        && down_reconciles
        && unchanged_reconciles
        && market_named
        && definition_present
}

fn trade(row: &Value) -> Result<(u64, DateTime<Utc>, f64), String> {
    let id = row["a"]
        .as_u64()
        .ok_or("aggregate trade ID missing or invalid")?;
    let millis = row["T"]
        .as_i64()
        .ok_or("aggregate trade timestamp missing or invalid")?;
    let at = Utc
        .timestamp_millis_opt(millis)
        .single()
        .ok_or("aggregate trade timestamp invalid")?;
    let price = row["p"]
        .as_str()
        .ok_or("aggregate trade price missing")?
        .parse::<f64>()
        .map_err(|_| "aggregate trade price invalid")?;
    if !price.is_finite() || price <= 0.0 {
        return Err("aggregate trade price must be positive and finite".into());
    }
    Ok((id, at, price))
}

/// Public calculation seam: all three real exchange responses must be present,
/// ordered and recent at the *same* sample cutoff. A missing member is an error.
pub fn calculate_crypto_tick(
    sample_at: DateTime<Utc>,
    payloads: &[(String, Value)],
) -> Result<CryptoTickSnapshot, String> {
    if payloads.len() != CRYPTO_TICK_UNIVERSE.len() {
        return Err("fixed crypto TICK basket requires all three members".into());
    }
    let mut members = Vec::with_capacity(CRYPTO_TICK_UNIVERSE.len());
    for symbol in CRYPTO_TICK_UNIVERSE {
        let mut matches = payloads.iter().filter(|(candidate, _)| candidate == symbol);
        let (_, payload) = matches
            .next()
            .ok_or_else(|| format!("missing fixed crypto TICK member {symbol}"))?;
        if matches.next().is_some() {
            return Err(format!("duplicate crypto TICK member {symbol}"));
        }
        let rows = payload
            .as_array()
            .ok_or_else(|| format!("{symbol}: aggregate trades must be an array"))?;
        if !(2..=1_000).contains(&rows.len()) {
            return Err(format!("{symbol}: need two bounded aggregate trades"));
        }
        let (previous_id, previous_at, previous_price) = trade(&rows[rows.len() - 2])?;
        let (latest_id, latest_at, latest_price) = trade(&rows[rows.len() - 1])?;
        if previous_id >= latest_id || previous_at > latest_at {
            return Err(format!("{symbol}: aggregate trades are not ordered"));
        }
        if latest_at > sample_at
            || sample_at
                .signed_duration_since(previous_at)
                .num_milliseconds()
                > MAX_TRADE_AGE_MS
        {
            return Err(format!(
                "{symbol}: aggregate trades are outside the common sample window"
            ));
        }
        let direction = if latest_price > previous_price {
            "up"
        } else if latest_price < previous_price {
            "down"
        } else {
            "unchanged"
        };
        members.push(CryptoTickMember {
            symbol: symbol.into(),
            previous_at,
            latest_at,
            previous_price,
            latest_price,
            direction,
            source_url: String::new(),
        });
    }
    let up_ticks = members
        .iter()
        .filter(|member| member.direction == "up")
        .count();
    let down_ticks = members
        .iter()
        .filter(|member| member.direction == "down")
        .count();
    let unchanged_ticks = members.len() - up_ticks - down_ticks;
    let snapshot = CryptoTickSnapshot {
        market: "binance_spot_usdt_fixed_three",
        definition:
            "各成员截至同一 UTC 时点的最近两笔聚合成交价方向；不是 NYSE TICK，也不是日线涨跌家数",
        sample_at,
        universe: CRYPTO_TICK_UNIVERSE
            .iter()
            .map(|symbol| (*symbol).into())
            .collect(),
        members,
        up_ticks,
        down_ticks,
        unchanged_ticks,
        net_tick: up_ticks as i64 - down_ticks as i64,
    };
    if !snapshot_is_consistent(&snapshot) {
        return Err("internal crypto TICK snapshot invariant failed".into());
    }
    Ok(snapshot)
}

/// Fetches uncached official Binance spot aggregate trades. The server clock
/// establishes one cutoff before any member request starts.
pub async fn fetch_crypto_tick(feed: &HttpFeed) -> Result<CryptoTickSnapshot, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| format!("cannot create Binance client: {error}"))?;
    let time: Value = client
        .get(format!("{}/api/v3/time", feed.base_url))
        .send()
        .await
        .map_err(|error| format!("Binance time request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Binance time response failed: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Binance time JSON failed: {error}"))?;
    let server_ms = time["serverTime"]
        .as_i64()
        .ok_or("Binance server time missing or invalid")?;
    let sample_at = Utc
        .timestamp_millis_opt(server_ms)
        .single()
        .ok_or("Binance server time outside UTC range")?
        - Duration::seconds(3);
    let cutoff_ms = sample_at.timestamp_millis();
    let responses = try_join_all(CRYPTO_TICK_UNIVERSE.into_iter().map(|symbol| {
        let client = client.clone();
        let endpoint = format!("{}/api/v3/aggTrades", feed.base_url);
        async move {
            let url = format!("{endpoint}?symbol={symbol}&endTime={cutoff_ms}&limit=100");
            let payload: Value = client
                .get(&url)
                .send()
                .await
                .map_err(|error| format!("{symbol}: Binance trades request failed: {error}"))?
                .error_for_status()
                .map_err(|error| format!("{symbol}: Binance trades response failed: {error}"))?
                .json()
                .await
                .map_err(|error| format!("{symbol}: Binance trades JSON failed: {error}"))?;
            Ok::<_, String>((symbol.to_string(), payload, url))
        }
    }))
    .await?;
    let payloads: Vec<_> = responses
        .iter()
        .map(|(symbol, payload, _)| (symbol.clone(), payload.clone()))
        .collect();
    let mut snapshot = calculate_crypto_tick(sample_at, &payloads)?;
    for (member, (_, _, source_url)) in snapshot.members.iter_mut().zip(responses) {
        member.source_url = source_url;
    }
    Ok(snapshot)
}
