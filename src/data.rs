//! K 线数据源。
//!
//! 三个 adapter 共享统一 trait `DataFeed`:
//!   - `SyntheticFeed`:本地随机生成,零依赖,适合测试和教学
//!   - `CsvFeed`:      从本地 CSV 加载(用于缓存真实历史数据)
//!   - `HttpFeed`:     通过 reqwest(异步)拉取真实 Binance 行情
//!
//! 注意:`DataFeed` trait 既是 sync 又是 async-friendly ——
//! sync 实现用于回测,async 实现用于实时/异步上下文。

use crate::types::Bar;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Timelike, Utc, Weekday};
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

const YAHOO_CHART_PRIMARY: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const YAHOO_CHART_FALLBACK: &str = "https://query2.finance.yahoo.com/v8/finance/chart";
const YAHOO_RELAY_DEFAULT: &str = "https://sigma711.top/axiom/api/provider/yahoo-chart";

/// 数据源 trait —— sync 接口,用于回测
pub trait DataFeed: Send + Sync {
    fn fetch_historical(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>>;
    fn stream_live(&self, symbol: &str) -> Result<Vec<Bar>>;
}

/// 异步数据源 trait —— 用于实时拉取(在 tokio runtime 里跑)
#[async_trait]
pub trait AsyncDataFeed: Send + Sync {
    async fn fetch_historical_async(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>>;
    async fn stream_live_async(&self, symbol: &str) -> Result<Vec<Bar>>;
}

// -----------------------------------------------------------------------------
// 合成数据 (SyntheticFeed) —— 用几何布朗运动
// -----------------------------------------------------------------------------

pub struct SyntheticFeed {
    pub seed: u64,
    pub default_period_seconds: i64,
}

impl Default for SyntheticFeed {
    fn default() -> Self {
        Self {
            seed: 42,
            default_period_seconds: 3600,
        }
    }
}

impl SyntheticFeed {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            default_period_seconds: 3600,
        }
    }
}

impl DataFeed for SyntheticFeed {
    fn fetch_historical(
        &self,
        _symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let normal = Normal::new(0.0, 1.0)?;

        let mut price = 30_000.0_f64;
        let period = self.default_period_seconds;
        let periods_per_year = (365.0 * 24.0 * 3600.0) / period as f64;
        let dt = 1.0 / periods_per_year;
        let mu = 0.10;
        let sigma = 0.60;

        let mut bars = Vec::with_capacity(limit);
        let mut current_time = since;

        for _ in 0..limit {
            let z = normal.sample(&mut rng);
            let shock = (mu - 0.5 * sigma * sigma) * dt + sigma * dt.sqrt() * z;
            let new_price = price * shock.exp();

            let open = price;
            let close = new_price;
            let up_noise: f64 = rng.gen_range(0.0..1.0) * 0.003;
            let down_noise: f64 = rng.gen_range(0.0..1.0) * 0.003;
            let high = open.max(close) * (1.0 + up_noise);
            let low = open.min(close) * (1.0 - down_noise);
            let volume = rng.gen_range(500.0..1500.0);

            bars.push(Bar {
                timestamp: current_time,
                open,
                high,
                low,
                close,
                volume,
            });

            price = new_price;
            current_time += Duration::seconds(period);
        }
        Ok(bars)
    }

    fn stream_live(&self, symbol: &str) -> Result<Vec<Bar>> {
        let now = Utc::now()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        self.fetch_historical(symbol, now, 1)
    }
}

// -----------------------------------------------------------------------------
// CSV 数据 (CsvFeed) —— 缓存真实数据
// -----------------------------------------------------------------------------

pub struct CsvFeed {
    pub cache_dir: PathBuf,
}

impl CsvFeed {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let dir = cache_dir.into();
        std::fs::create_dir_all(&dir).ok();
        Self { cache_dir: dir }
    }

    fn path(&self, symbol: &str, timeframe: &str) -> PathBuf {
        let safe = |value: &str| {
            value
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        };
        self.cache_dir
            .join(format!("{}_{}.csv", safe(symbol), safe(timeframe)))
    }

    pub fn save(&self, symbol: &str, timeframe: &str, bars: &[Bar]) -> Result<PathBuf> {
        let path = self.path(symbol, timeframe);
        crate::practice::validate_bars(bars).map_err(anyhow::Error::msg)?;
        let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut f = File::create(&temp).with_context(|| format!("创建文件失败: {:?}", path))?;
        writeln!(f, "timestamp,open,high,low,close,volume")?;
        for b in bars {
            writeln!(
                f,
                "{},{},{},{},{},{}",
                b.timestamp.to_rfc3339(),
                b.open,
                b.high,
                b.low,
                b.close,
                b.volume
            )?;
        }
        f.sync_all()?;
        drop(f);
        std::fs::rename(&temp, &path)?;
        Ok(path)
    }

    pub fn load(&self, symbol: &str, timeframe: &str) -> Result<Vec<Bar>> {
        let path = self.path(symbol, timeframe);
        let f = File::open(&path).with_context(|| format!("打开文件失败: {:?}", path))?;
        let reader = BufReader::new(f);
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);
        let mut bars = Vec::new();
        for result in rdr.records() {
            let record = result?;
            anyhow::ensure!(
                record.len() == 6,
                "CSV must contain timestamp/open/high/low/close/volume"
            );
            let ts = DateTime::parse_from_rfc3339(&record[0])?.with_timezone(&Utc);
            bars.push(Bar {
                timestamp: ts,
                open: record[1].parse()?,
                high: record[2].parse()?,
                low: record[3].parse()?,
                close: record[4].parse()?,
                volume: record[5].parse()?,
            });
        }
        crate::practice::validate_bars(&bars).map_err(anyhow::Error::msg)?;
        Ok(bars)
    }
}

impl DataFeed for CsvFeed {
    fn fetch_historical(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>> {
        let all = self.load(symbol, "1h")?;
        let filtered: Vec<Bar> = all
            .into_iter()
            .filter(|b| b.timestamp >= since)
            .take(limit)
            .collect();
        Ok(filtered)
    }

    fn stream_live(&self, symbol: &str) -> Result<Vec<Bar>> {
        let since = Utc::now() - Duration::hours(1);
        self.fetch_historical(symbol, since, 1)
    }
}

// -----------------------------------------------------------------------------
// HTTP 数据 (HttpFeed) —— 真实 Binance 行情(异步)
// -----------------------------------------------------------------------------

/// Binance 返回的 K 线是 JSON 数组(不是对象),且元素超过 6 个。
/// 用 serde_json::Value 先解析,再手工提取前 6 个字段(这样可以容忍多余字段)。
fn parse_binance_kline(v: &serde_json::Value) -> Option<Bar> {
    let arr = v.as_array()?;
    let ts = arr.first()?.as_i64()?;
    let open: f64 = arr.get(1)?.as_str()?.parse().ok()?;
    let high: f64 = arr.get(2)?.as_str()?.parse().ok()?;
    let low: f64 = arr.get(3)?.as_str()?.parse().ok()?;
    let close: f64 = arr.get(4)?.as_str()?.parse().ok()?;
    let volume: f64 = arr.get(5)?.as_str()?.parse().ok()?;
    Some(Bar {
        timestamp: Utc.timestamp_millis_opt(ts).single()?,
        open,
        high,
        low,
        close,
        volume,
    })
}

/// A Binance 1-hour candle observed before the exchange says it is closed.
/// Its `close` is a snapshot value and never a final close.
#[derive(Clone, Debug)]
pub struct ProvisionalCandleSnapshot {
    pub candle: Bar,
    pub fetched_at: DateTime<Utc>,
    pub expected_close_at: DateTime<Utc>,
}

/// Adjacent candles fetched from one Binance response, so cache age cannot
/// separate the last completed candle from the live observation.
#[derive(Clone, Debug)]
pub struct OpenCandlePair {
    pub last_completed: Bar,
    pub provisional: ProvisionalCandleSnapshot,
}

#[derive(Clone, Debug)]
pub struct BinanceTradeBar {
    pub bar: Bar,
    pub quote_volume: f64,
    /// Binance kline field 8: number of trades in this candle.
    pub trade_count: u64,
    /// Binance kline field 9: base-asset quantity where the buyer was taker.
    pub taker_buy_base_volume: f64,
    /// Binance kline field 10: quote-asset quantity where the buyer was taker.
    pub taker_buy_quote_volume: f64,
}

/// Executed spot trade. Decimal price spelling is retained for exact-price grouping.
#[derive(Clone, Debug, Serialize)]
pub struct BinanceRecentTrade {
    pub id: u64,
    pub price: f64,
    pub price_decimal: String,
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StockAdjustmentObservation {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub adjusted_close: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct StockSplitEvent {
    pub kind: &'static str,
    pub effective_at: DateTime<Utc>,
    pub effective_trading_date: NaiveDate,
    pub numerator: f64,
    pub denominator: f64,
    pub split_ratio: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StockAdjustmentEvidence {
    pub provider: &'static str,
    pub endpoint: String,
    pub fetched_at: DateTime<Utc>,
    pub retrieval: &'static str,
    pub scope: &'static str,
    pub event: StockSplitEvent,
    pub issuer_confirmation_url: &'static str,
    pub issuer_confirmation: &'static str,
    pub observations: Vec<StockAdjustmentObservation>,
    pub quote_basis: &'static str,
    pub adjusted_close_basis: &'static str,
    pub calculation: &'static str,
}

fn parse_binance_recent_trades(
    raw: &serde_json::Value,
    fetched_at: DateTime<Utc>,
) -> Result<Vec<BinanceRecentTrade>> {
    let rows = raw.as_array().context("recent trades must be an array")?;
    anyhow::ensure!(
        (2..=1000).contains(&rows.len()),
        "recent trades need 2..1000 records"
    );
    let mut trades: Vec<BinanceRecentTrade> = Vec::with_capacity(rows.len());
    let mut represented_prices = std::collections::HashMap::new();
    for row in rows {
        let decimal = |key: &str| -> Result<(f64, String)> {
            let text = row[key]
                .as_str()
                .context("trade decimal must be a string")?;
            anyhow::ensure!(
                !text.is_empty()
                    && text.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                    && text.bytes().filter(|b| *b == b'.').count() <= 1,
                "invalid trade decimal spelling"
            );
            let value = text.parse::<f64>().context("invalid trade decimal")?;
            anyhow::ensure!(
                value.is_finite() && value > 0.0,
                "trade price and quantity must be positive finite"
            );
            let canonical = if text.contains('.') {
                text.trim_end_matches('0').trim_end_matches('.')
            } else {
                text
            };
            let canonical = canonical.trim_start_matches('0');
            Ok((
                value,
                if canonical.starts_with('.') {
                    format!("0{canonical}")
                } else {
                    canonical.to_owned()
                },
            ))
        };
        let (price, price_decimal) = decimal("price")?;
        if let Some(previous) = represented_prices.insert(price.to_bits(), price_decimal.clone()) {
            anyhow::ensure!(
                previous == price_decimal,
                "distinct decimal prices collapse at numeric precision"
            );
        }
        let (quantity, _) = decimal("qty")?;
        let id = row["id"]
            .as_u64()
            .context("trade id must be an unsigned integer")?;
        let millis = row["time"]
            .as_i64()
            .context("trade time must be milliseconds")?;
        let timestamp = Utc
            .timestamp_millis_opt(millis)
            .single()
            .context("invalid trade timestamp")?;
        anyhow::ensure!(
            millis > 0 && timestamp <= fetched_at,
            "trade timestamp is future or invalid"
        );
        if let Some(previous) = trades.last() {
            anyhow::ensure!(
                previous.id.checked_add(1) == Some(id) && previous.timestamp <= timestamp,
                "recent trade IDs must be consecutive ascending and times nondecreasing"
            );
        }
        trades.push(BinanceRecentTrade {
            id,
            price,
            price_decimal,
            quantity,
            timestamp,
        });
    }
    Ok(trades)
}

/// One visible price level from a Binance spot depth snapshot. It is an
/// outstanding order, not an executed trade.
#[derive(Clone, Debug, Serialize)]
pub struct BinanceDepthLevel {
    pub price: f64,
    pub quantity: f64,
}

/// A point-in-time Binance spot REST depth response. `update_id` is a sequence
/// identifier supplied by the exchange; Binance's depth endpoint has no
/// exchange timestamp, so callers must never treat it as one.
#[derive(Clone, Debug, Serialize)]
pub struct BinanceDepthSnapshot {
    pub update_id: u64,
    pub bids: Vec<BinanceDepthLevel>,
    pub asks: Vec<BinanceDepthLevel>,
}

/// A Bitcoin mainnet block as returned by an Esplora blocks snapshot.
#[derive(Clone, Debug, Serialize)]
pub struct BitcoinBlock {
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
    /// Count declared by the Esplora block header response. This includes the
    /// coinbase transaction and is deliberately not inferred from a page.
    pub tx_count: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct BitcoinBlockSnapshot {
    pub provider: String,
    pub endpoint: String,
    pub fetched_at: DateTime<Utc>,
    pub blocks: Vec<BitcoinBlock>,
}
fn valid_bitcoin_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn parse_bitcoin_block_snapshot(raw: &serde_json::Value) -> Result<Vec<BitcoinBlock>> {
    let rows = raw
        .as_array()
        .context("Bitcoin blocks response must be an array")?;
    anyhow::ensure!(
        rows.len() == 10,
        "Bitcoin block snapshot must contain exactly 10 blocks"
    );
    let mut blocks: Vec<BitcoinBlock> = Vec::with_capacity(rows.len());
    for row in rows {
        let height = row["height"]
            .as_u64()
            .context("Bitcoin block height must be an unsigned integer")?;
        let hash = row["id"]
            .as_str()
            .context("Bitcoin block id must be a string")?
            .to_owned();
        let previous_hash = row["previousblockhash"]
            .as_str()
            .context("Bitcoin previousblockhash must be a string")?
            .to_owned();
        anyhow::ensure!(
            valid_bitcoin_hash(&hash) && valid_bitcoin_hash(&previous_hash),
            "Bitcoin block hashes must be 64 hexadecimal characters"
        );
        anyhow::ensure!(
            hash != previous_hash,
            "Bitcoin block hash cannot equal its previous hash"
        );
        let size_bytes = row["size"]
            .as_u64()
            .filter(|size| *size > 0)
            .context("Bitcoin block size must be positive bytes")?;
        let tx_count = row["tx_count"]
            .as_u64()
            .filter(|count| *count > 0)
            .context("Bitcoin block tx_count must be a positive unsigned integer")?;
        let seconds = row["timestamp"]
            .as_i64()
            .context("Bitcoin block timestamp must be Unix seconds")?;
        let timestamp = Utc
            .timestamp_opt(seconds, 0)
            .single()
            .context("Bitcoin block timestamp is invalid")?;
        if let Some(newer) = blocks.last() {
            anyhow::ensure!(
                newer.height.checked_sub(1) == Some(height),
                "Bitcoin block heights must be consecutive newest-to-oldest"
            );
            anyhow::ensure!(
                newer.previous_hash == hash,
                "Bitcoin block hashes are not linked newest-to-oldest"
            );
        }
        blocks.push(BitcoinBlock {
            height,
            hash,
            previous_hash,
            timestamp,
            size_bytes,
            tx_count,
        });
    }
    blocks.reverse();
    for pair in blocks.windows(2) {
        anyhow::ensure!(
            pair[0].height.checked_add(1) == Some(pair[1].height),
            "Bitcoin block heights must be consecutive oldest-to-newest"
        );
        anyhow::ensure!(
            pair[1].previous_hash == pair[0].hash,
            "Bitcoin block hashes are not linked oldest-to-newest"
        );
    }
    Ok(blocks)
}

#[derive(Clone, Debug, Serialize)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub fee_sats: u64,
    pub size_bytes: u64,
    /// Values of every previous output spent by this ordinary transaction.
    /// Esplora supplies these as `vin[].prevout.value`, in satoshis.
    pub spent_prevout_values_sats: Vec<u64>,
    /// Decoded script addresses, if Esplora exposes one for each spent prevout.
    /// An address is a script label, not a person or an economic counterparty.
    pub spent_prevout_addresses: Vec<Option<String>>,
    /// Outputs created by this ordinary transaction. `op_return` outputs are
    /// retained so consumers can disclose their explicit exclusion.
    pub outputs: Vec<BitcoinTransactionOutput>,
}
#[derive(Clone, Debug, Serialize)]
pub struct BitcoinTransactionOutput {
    pub value_sats: u64,
    pub scriptpubkey_type: String,
    pub scriptpubkey_address: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct BitcoinTransactionSample {
    pub provider: String,
    pub endpoint: String,
    pub fetched_at: DateTime<Utc>,
    pub block: BitcoinBlock,
    pub returned_count: usize,
    pub excluded_coinbase_count: usize,
    pub transactions: Vec<BitcoinTransaction>,
}
fn bitcoin_script_address(value: &serde_json::Value, field: &str) -> Result<Option<String>> {
    match value.get("scriptpubkey_address") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(address))
            if !address.trim().is_empty() && address.len() <= 128 && address.is_ascii() =>
        {
            Ok(Some(address.clone()))
        }
        _ => anyhow::bail!("{field}.scriptpubkey_address must be a nonempty ASCII address or null"),
    }
}
fn parse_bitcoin_transactions(
    raw: &serde_json::Value,
    block: &BitcoinBlock,
) -> Result<(usize, usize, Vec<BitcoinTransaction>)> {
    let rows = raw
        .as_array()
        .context("Bitcoin transaction response must be an array")?;
    anyhow::ensure!(
        !rows.is_empty() && rows.len() <= 25,
        "Bitcoin transaction first page must contain 1..25 rows"
    );
    let mut seen = std::collections::HashSet::new();
    let mut excluded = 0;
    let mut out = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let vin = row["vin"]
            .as_array()
            .context("transaction vin must be a nonempty array")?;
        anyhow::ensure!(!vin.is_empty(), "transaction vin must be nonempty");
        let txid = row["txid"]
            .as_str()
            .context("Bitcoin transaction txid must be a string")?
            .to_owned();
        anyhow::ensure!(
            valid_bitcoin_hash(&txid) && seen.insert(txid.clone()),
            "Bitcoin transaction txids must be unique hashes"
        );
        anyhow::ensure!(
            row["status"]["confirmed"].as_bool() == Some(true)
                && row["status"]["block_hash"].as_str() == Some(&block.hash)
                && row["status"]["block_height"].as_u64() == Some(block.height),
            "transaction must be confirmed in the pinned block"
        );
        let coinbase = vin.first().and_then(|x| x["is_coinbase"].as_bool()) == Some(true);
        if coinbase {
            anyhow::ensure!(
                index == 0,
                "coinbase must be the first transaction on page zero"
            );
            excluded += 1;
            continue;
        }
        anyhow::ensure!(
            index != 0,
            "first transaction on page zero must be coinbase"
        );
        let fee_sats = row["fee"]
            .as_u64()
            .context("transaction fee must be unsigned sats")?;
        let size_bytes = row["size"]
            .as_u64()
            .filter(|x| *x > 0)
            .context("transaction size must be positive bytes")?;
        let spent_prevout_values_sats = vin
            .iter()
            .enumerate()
            .map(|(vin_index, input)| {
                anyhow::ensure!(
                    input["is_coinbase"].as_bool() != Some(true),
                    "ordinary transaction vin[{vin_index}] cannot be coinbase"
                );
                input["prevout"]["value"].as_u64().with_context(|| {
                    format!(
                        "ordinary transaction vin[{vin_index}] prevout value must be unsigned sats"
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let spent_prevout_addresses = vin
            .iter()
            .enumerate()
            .map(|(vin_index, input)| {
                bitcoin_script_address(&input["prevout"], &format!("vin[{vin_index}].prevout"))
            })
            .collect::<Result<Vec<_>>>()?;
        let vout = row["vout"]
            .as_array()
            .context("ordinary transaction vout must be a nonempty array")?;
        anyhow::ensure!(
            !vout.is_empty(),
            "ordinary transaction vout must be nonempty"
        );
        let outputs = vout
            .iter()
            .enumerate()
            .map(|(vout_index, output)| {
                let value_sats = output["value"]
                    .as_u64()
                    .with_context(|| format!("ordinary transaction vout[{vout_index}] value must be unsigned sats"))?;
                let scriptpubkey_type = output["scriptpubkey_type"]
                    .as_str()
                    .filter(|kind| !kind.is_empty())
                    .with_context(|| format!("ordinary transaction vout[{vout_index}] scriptpubkey_type must be a nonempty string"))?
                    .to_owned();
                let scriptpubkey_address = bitcoin_script_address(output, &format!("vout[{vout_index}]"))?;
                Ok(BitcoinTransactionOutput { value_sats, scriptpubkey_type, scriptpubkey_address })
            })
            .collect::<Result<Vec<_>>>()?;
        let spent_value_sats =
            spent_prevout_values_sats
                .iter()
                .try_fold(0u64, |total, value| {
                    total
                        .checked_add(*value)
                        .context("ordinary transaction input value overflow")
                })?;
        let created_value_sats = outputs.iter().try_fold(0u64, |total, output| {
            total
                .checked_add(output.value_sats)
                .context("ordinary transaction output value overflow")
        })?;
        anyhow::ensure!(
            created_value_sats.checked_add(fee_sats) == Some(spent_value_sats),
            "ordinary transaction prevout values must equal all output values plus fee"
        );
        out.push(BitcoinTransaction {
            txid,
            fee_sats,
            size_bytes,
            spent_prevout_values_sats,
            spent_prevout_addresses,
            outputs,
        });
    }
    anyhow::ensure!(
        excluded == 1 && out.len() <= 24,
        "first page must contain a leading coinbase, then at most 24 ordinary transactions"
    );
    Ok((rows.len(), excluded, out))
}

fn parse_binance_depth_snapshot(raw: &serde_json::Value) -> Result<BinanceDepthSnapshot> {
    let update_id = raw["lastUpdateId"]
        .as_u64()
        .filter(|id| *id > 0)
        .context("Binance depth response is missing a positive lastUpdateId")?;
    let parse_side = |key: &str, descending: bool| -> Result<Vec<BinanceDepthLevel>> {
        let rows = raw[key]
            .as_array()
            .with_context(|| format!("Binance depth response has no {key} array"))?;
        anyhow::ensure!(!rows.is_empty(), "Binance depth {key} side is empty");
        let mut levels: Vec<BinanceDepthLevel> = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let fields = row
                .as_array()
                .with_context(|| format!("Binance depth {key}[{index}] is not a tuple"))?;
            anyhow::ensure!(
                fields.len() == 2,
                "Binance depth {key}[{index}] must contain exactly price and quantity"
            );
            let parse_decimal = |field: usize, label: &str| -> Result<f64> {
                fields[field]
                    .as_str()
                    .and_then(|value| value.parse::<f64>().ok())
                    .filter(|value| value.is_finite())
                    .with_context(|| format!("Binance depth {key}[{index}] has invalid {label}"))
            };
            let price = parse_decimal(0, "price")?;
            let quantity = parse_decimal(1, "quantity")?;
            anyhow::ensure!(
                price > 0.0,
                "Binance depth {key}[{index}] price must be positive"
            );
            anyhow::ensure!(
                quantity >= 0.0,
                "Binance depth {key}[{index}] quantity must be nonnegative"
            );
            if let Some(previous) = levels.last() {
                let sorted = if descending {
                    previous.price > price
                } else {
                    previous.price < price
                };
                anyhow::ensure!(
                    sorted,
                    "Binance depth {key} prices must be strictly {}",
                    if descending {
                        "descending"
                    } else {
                        "ascending"
                    }
                );
            }
            levels.push(BinanceDepthLevel { price, quantity });
        }
        Ok(levels)
    };
    let bids = parse_side("bids", true)?;
    let asks = parse_side("asks", false)?;
    anyhow::ensure!(
        bids[0].price < asks[0].price,
        "Binance depth snapshot is locked or crossed"
    );
    Ok(BinanceDepthSnapshot {
        update_id,
        bids,
        asks,
    })
}

pub struct HttpFeed {
    pub base_url: String,
    pub us_stock_adjustment_url: String,
    pub us_stock_adjustment_fallback_url: String,
    pub us_stock_relay_url: Option<String>,
    pub bitcoin_esplora_url: String,
    pub bitcoin_mempool_url: String,
    pub csv: CsvFeed,
    client: reqwest::Client,
}

struct YahooRequestFailure {
    error: anyhow::Error,
    retryable: bool,
}

async fn request_yahoo_chart_json(
    client: &reqwest::Client,
    endpoint: &str,
    query: &[(&str, String)],
) -> std::result::Result<(serde_json::Value, String), YahooRequestFailure> {
    let response = client
        .get(endpoint)
        .header(
            reqwest::header::USER_AGENT,
            "AXIOM educational market reader/1.0",
        )
        .query(query)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|error| YahooRequestFailure {
            error: anyhow::Error::new(error).context("Yahoo chart request failed"),
            retryable: true,
        })?;
    let selected_endpoint = response.url().to_string();
    let status = response.status();
    if !status.is_success() {
        return Err(YahooRequestFailure {
            error: anyhow::anyhow!("Yahoo chart returned HTTP {status}"),
            // A 429 is host-specific rate limiting: immediately try the
            // alternate host instead of amplifying it with a tight retry.
            retryable: status.is_server_error(),
        });
    }
    let raw = response.json().await.map_err(|error| YahooRequestFailure {
        error: anyhow::Error::new(error).context("invalid Yahoo chart JSON"),
        retryable: false,
    })?;
    Ok((raw, selected_endpoint))
}

#[derive(Deserialize)]
struct YahooRelayEnvelope {
    provider: String,
    retrieval: String,
    source_url: String,
    payload: serde_json::Value,
}

fn validated_yahoo_relay_url(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let parsed = reqwest::Url::parse(value).ok()?;
    let loopback_http = parsed.scheme() == "http"
        && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "::1"));
    (parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.fragment().is_none()
        && (parsed.scheme() == "https" || loopback_http))
        .then(|| parsed.to_string())
}

fn configured_yahoo_relay_url() -> Option<String> {
    match std::env::var("AXIOM_YAHOO_RELAY_URL") {
        Ok(value) => validated_yahoo_relay_url(&value),
        Err(_) => Some(YAHOO_RELAY_DEFAULT.to_owned()),
    }
}

fn validate_yahoo_relay_source_url(
    source_url: &str,
    symbol: &str,
    period1: &str,
    period2: &str,
) -> Result<String> {
    let parsed = reqwest::Url::parse(source_url).context("relay source URL is invalid")?;
    let expected_path = format!("/v8/finance/chart/{}", symbol.replace('.', "-"));
    anyhow::ensure!(
        parsed.scheme() == "https"
            && parsed.host_str() == Some("query2.finance.yahoo.com")
            && parsed.port().is_none()
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.path() == expected_path
            && parsed.fragment().is_none(),
        "relay source must be the exact restricted Yahoo query2 chart endpoint"
    );
    let pairs: Vec<_> = parsed.query_pairs().collect();
    anyhow::ensure!(
        pairs.len() == 5,
        "relay source query must contain exactly five fields"
    );
    let mut query = std::collections::HashMap::new();
    for (key, value) in pairs {
        anyhow::ensure!(
            query.insert(key.into_owned(), value.into_owned()).is_none(),
            "relay source query fields must be unique"
        );
    }
    anyhow::ensure!(
        query.get("period1").map(String::as_str) == Some(period1)
            && query.get("period2").map(String::as_str) == Some(period2)
            && query.get("interval").map(String::as_str) == Some("1d")
            && query.get("includePrePost").map(String::as_str) == Some("false")
            && query.get("events").map(String::as_str) == Some("div,splits"),
        "relay source query does not match the requested daily chart"
    );
    Ok(parsed.to_string())
}

async fn request_yahoo_relay_json(
    client: &reqwest::Client,
    relay_url: &str,
    symbol: &str,
    period1: &str,
    period2: &str,
) -> Result<(serde_json::Value, String)> {
    let response = client
        .get(relay_url)
        .header(
            reqwest::header::USER_AGENT,
            "AXIOM educational market reader/1.0",
        )
        .query(&[
            ("symbol", symbol),
            ("period1", period1),
            ("period2", period2),
        ])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("restricted Yahoo relay request failed")?
        .error_for_status()
        .context("restricted Yahoo relay returned HTTP error")?;
    let envelope: YahooRelayEnvelope = response
        .json()
        .await
        .context("restricted Yahoo relay returned invalid JSON")?;
    anyhow::ensure!(
        envelope.provider == "yahoo" && envelope.retrieval == "restricted_server_relay",
        "restricted Yahoo relay metadata is invalid"
    );
    let source_url =
        validate_yahoo_relay_source_url(&envelope.source_url, symbol, period1, period2)?;
    Ok((envelope.payload, source_url))
}

impl HttpFeed {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_url: "https://data-api.binance.vision".to_string(),
            us_stock_adjustment_url: YAHOO_CHART_PRIMARY.to_string(),
            us_stock_adjustment_fallback_url: YAHOO_CHART_FALLBACK.to_string(),
            us_stock_relay_url: configured_yahoo_relay_url(),
            bitcoin_esplora_url: "https://blockstream.info".to_string(),
            bitcoin_mempool_url: "https://mempool.space".to_string(),
            csv: CsvFeed::new(cache_dir),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("无法创建 HTTP client"),
        }
    }

    pub async fn fetch_aapl_split_adjustment_evidence(&self) -> Result<StockAdjustmentEvidence> {
        let endpoints = [
            self.us_stock_adjustment_url.as_str(),
            self.us_stock_adjustment_fallback_url.as_str(),
        ];
        let query = [
            ("period1", "1595808000".to_owned()),
            ("period2", "1601510400".to_owned()),
            ("interval", "1d".to_owned()),
            ("includePrePost", "false".to_owned()),
            ("events", "div,splits".to_owned()),
        ];
        let mut errors = Vec::new();
        for base in endpoints {
            let endpoint = format!("{}/AAPL", base.trim_end_matches('/'));
            for attempt in 1..=2 {
                let (raw, selected_endpoint) =
                    match request_yahoo_chart_json(&self.client, &endpoint, &query).await {
                        Ok(value) => value,
                        Err(failure) => {
                            errors
                                .push(format!("attempt {attempt} {endpoint}: {:#}", failure.error));
                            if failure.retryable && attempt == 1 {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                continue;
                            }
                            break;
                        }
                    };
                match parse_aapl_split_adjustment_evidence(&raw, selected_endpoint, Utc::now()) {
                    Ok(evidence) => return Ok(evidence),
                    Err(error) => {
                        // A syntactically valid but semantically unusable
                        // provider response is not transient. Try the other
                        // same-schema host without repeating this response.
                        errors.push(format!("{endpoint}: {error:#}"));
                        break;
                    }
                }
            }
        }
        if let Some(relay_url) = self.us_stock_relay_url.as_deref() {
            match request_yahoo_relay_json(
                &self.client,
                relay_url,
                "AAPL",
                "1595808000",
                "1601510400",
            )
            .await
            {
                Ok((raw, source_url)) => {
                    let mut evidence =
                        parse_aapl_split_adjustment_evidence(&raw, source_url, Utc::now())?;
                    evidence.provider = "yahoo_via_restricted_relay";
                    evidence.retrieval = "restricted_server_relay";
                    return Ok(evidence);
                }
                Err(error) => errors.push(format!("restricted relay: {error:#}")),
            }
        }
        anyhow::bail!(
            "AAPL adjustment evidence Yahoo endpoints and relay failed closed: {}",
            errors.join(" | ")
        )
    }

    async fn fetch_bitcoin_blocks_from(
        &self,
        provider: &str,
        endpoint: &str,
    ) -> Result<BitcoinBlockSnapshot> {
        let endpoint = endpoint.trim_end_matches('/');
        let raw: serde_json::Value = self
            .client
            .get(format!("{}/api/blocks", endpoint))
            .send()
            .await
            .context("Bitcoin blocks request failed")?
            .error_for_status()
            .context("Bitcoin blocks response failed")?
            .json()
            .await
            .context("invalid Bitcoin blocks JSON")?;
        let blocks = parse_bitcoin_block_snapshot(&raw)?;
        Ok(BitcoinBlockSnapshot {
            provider: provider.into(),
            endpoint: format!("{}/api/blocks", endpoint),
            fetched_at: Utc::now(),
            blocks,
        })
    }
    /// Fetches an uncached current Bitcoin-mainnet window. Mempool is used only
    /// when Blockstream fails; neither provider is cached or synthesized.
    pub async fn fetch_bitcoin_mainnet_blocks(&self) -> Result<BitcoinBlockSnapshot> {
        match self
            .fetch_bitcoin_blocks_from("blockstream_esplora", &self.bitcoin_esplora_url)
            .await
        {
            Ok(snapshot) => Ok(snapshot),
            Err(primary) => self
                .fetch_bitcoin_blocks_from("mempool_esplora", &self.bitcoin_mempool_url)
                .await
                .with_context(|| format!("Blockstream failed: {primary}")),
        }
    }

    async fn fetch_bitcoin_transactions_from(
        &self,
        provider: &str,
        base: &str,
        block: &BitcoinBlock,
    ) -> Result<BitcoinTransactionSample> {
        let base = base.trim_end_matches('/');
        let endpoint = format!("{}/api/block/{}/txs/0", base, block.hash);
        let raw: serde_json::Value = self
            .client
            .get(&endpoint)
            .send()
            .await
            .context("Bitcoin transaction request failed")?
            .error_for_status()
            .context("Bitcoin transaction response failed")?
            .json()
            .await
            .context("invalid Bitcoin transactions JSON")?;
        let (returned_count, excluded_coinbase_count, transactions) =
            parse_bitcoin_transactions(&raw, block)?;
        Ok(BitcoinTransactionSample {
            provider: provider.into(),
            endpoint,
            fetched_at: Utc::now(),
            block: block.clone(),
            returned_count,
            excluded_coinbase_count,
            transactions,
        })
    }
    pub async fn fetch_bitcoin_mainnet_transaction_sample(
        &self,
    ) -> Result<BitcoinTransactionSample> {
        let snapshot = self.fetch_bitcoin_mainnet_blocks().await?;
        let block = snapshot
            .blocks
            .get(3)
            .context("validated Bitcoin snapshot lacks confirmation-depth block")?;
        let primary = &snapshot.provider;
        let primary_base = if primary == "blockstream_esplora" {
            &self.bitcoin_esplora_url
        } else {
            &self.bitcoin_mempool_url
        };
        match self
            .fetch_bitcoin_transactions_from(primary, primary_base, block)
            .await
        {
            Ok(sample) => Ok(sample),
            Err(error) if primary == "blockstream_esplora" => self
                .fetch_bitcoin_transactions_from(
                    "mempool_esplora",
                    &self.bitcoin_mempool_url,
                    block,
                )
                .await
                .with_context(|| format!("Blockstream transaction page failed: {error}")),
            Err(error) => Err(error),
        }
    }

    /// Fetch the recent executed-trade window without cache or synthetic fallback.
    pub async fn fetch_recent_spot_trades(
        &self,
        symbol: &str,
    ) -> Result<(DateTime<Utc>, Vec<BinanceRecentTrade>)> {
        let raw: serde_json::Value = self
            .client
            .get(format!("{}/api/v3/trades", self.base_url))
            .query(&[("symbol", symbol), ("limit", "1000")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let fetched_at = self.binance_server_time().await?;
        Ok((fetched_at, parse_binance_recent_trades(&raw, fetched_at)?))
    }

    /// Fetch an uncached, normalized Binance USDT-spot book snapshot. Parsing
    /// and structural validation stay at the source boundary so no caller can
    /// compute a practice result from malformed or crossed upstream depth.
    pub async fn fetch_binance_usdt_spot_depth(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<BinanceDepthSnapshot> {
        anyhow::ensure!(
            symbol
                .strip_suffix("USDT")
                .is_some_and(|base| !base.is_empty())
                && symbol
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()),
            "Binance depth requires an uppercase Binance USDT spot symbol"
        );
        anyhow::ensure!(
            matches!(limit, 5 | 10 | 20 | 50 | 100),
            "unsupported Binance depth limit"
        );
        let raw: serde_json::Value = self
            .client
            .get(format!("{}/api/v3/depth", self.base_url))
            .query(&[("symbol", symbol), ("limit", &limit.to_string())])
            .send()
            .await
            .context("Binance depth request failed")?
            .error_for_status()
            .context("Binance depth response failed")?
            .json()
            .await
            .context("invalid Binance depth JSON")?;
        let snapshot = parse_binance_depth_snapshot(&raw)?;
        anyhow::ensure!(
            snapshot.bids.len() == limit && snapshot.asks.len() == limit,
            "Binance depth response did not return the requested number of levels"
        );
        Ok(snapshot)
    }

    async fn binance_server_time(&self) -> Result<DateTime<Utc>> {
        let raw: serde_json::Value = self
            .client
            .get(format!("{}/api/v3/time", self.base_url))
            .send()
            .await
            .context("Binance server time request failed")?
            .error_for_status()
            .context("Binance server time returned HTTP error")?
            .json()
            .await
            .context("invalid Binance server time JSON")?;
        let milliseconds = raw["serverTime"]
            .as_i64()
            .context("Binance server time is missing")?;
        Utc.timestamp_millis_opt(milliseconds)
            .single()
            .context("invalid Binance server time")
    }

    /// Fetch the last completed and current active candle from one uncached
    /// Binance response, validating the active interval against exchange time.
    pub async fn fetch_open_candle_pair_async(&self, symbol: &str) -> Result<OpenCandlePair> {
        let mut last_error = None;
        for _ in 0..2 {
            let before_request = self.binance_server_time().await?;
            let pair = self.fetch_open_candle_pair_at(symbol, before_request).await;
            let after_request = self.binance_server_time().await?;
            match pair {
                Ok(pair) if after_request < pair.provisional.expected_close_at => {
                    return Ok(OpenCandlePair {
                        provisional: ProvisionalCandleSnapshot {
                            fetched_at: after_request,
                            ..pair.provisional
                        },
                        ..pair
                    })
                }
                Ok(_) => {
                    last_error =
                        Some("Binance hour changed while requesting the active candle".to_string())
                }
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        Err(anyhow::anyhow!(last_error.unwrap_or_else(|| {
            "unable to observe an active Binance candle".to_string()
        })))
    }

    /// Deterministic seam for a two-candle response observed at exchange time.
    pub async fn fetch_open_candle_pair_at(
        &self,
        symbol: &str,
        as_of: DateTime<Utc>,
    ) -> Result<OpenCandlePair> {
        let hour_start = as_of
            .with_minute(0)
            .and_then(|time| time.with_second(0))
            .and_then(|time| time.with_nanosecond(0))
            .context("invalid Binance observation time")?;
        let raw: serde_json::Value = self
            .client
            .get(format!("{}/api/v3/klines", self.base_url))
            .query(&[
                ("symbol", symbol.replace('/', "")),
                ("interval", "1h".to_string()),
                (
                    "startTime",
                    (hour_start - Duration::hours(1))
                        .timestamp_millis()
                        .to_string(),
                ),
                ("limit", "2".to_string()),
            ])
            .send()
            .await
            .context("open-candle market request failed")?
            .error_for_status()
            .context("open-candle market returned HTTP error")?
            .json()
            .await
            .context("invalid open-candle market JSON")?;
        let rows = raw
            .as_array()
            .context("open-candle market response is not an array")?;
        anyhow::ensure!(
            rows.len() == 2,
            "open-candle market response needs adjacent completed and active rows"
        );
        let last_completed =
            parse_binance_kline(&rows[0]).context("invalid completed kline fields")?;
        let candle = parse_binance_kline(&rows[1]).context("invalid provisional kline fields")?;
        let close_time = |row: &serde_json::Value, label: &str| -> Result<DateTime<Utc>> {
            let milliseconds = row
                .as_array()
                .and_then(|fields| fields.get(6))
                .and_then(serde_json::Value::as_i64)
                .with_context(|| format!("{label} kline is missing exchange close time"))?;
            Utc.timestamp_millis_opt(milliseconds.saturating_add(1))
                .single()
                .with_context(|| format!("invalid {label} kline close time"))
        };
        let completed_close_at = close_time(&rows[0], "completed")?;
        let expected_close_at = close_time(&rows[1], "provisional")?;
        let valid_ohlcv = |bar: &Bar| {
            bar.open.is_finite()
                && bar.high.is_finite()
                && bar.low.is_finite()
                && bar.close.is_finite()
                && bar.volume.is_finite()
                && bar.open > 0.0
                && bar.high > 0.0
                && bar.low > 0.0
                && bar.close > 0.0
                && bar.high >= bar.low
                && bar.high >= bar.open.max(bar.close)
                && bar.low <= bar.open.min(bar.close)
                && bar.volume >= 0.0
        };
        anyhow::ensure!(
            valid_ohlcv(&last_completed) && valid_ohlcv(&candle),
            "Binance open-candle rows have invalid OHLCV"
        );
        anyhow::ensure!(
            last_completed.timestamp + Duration::hours(1) == candle.timestamp
                && candle.timestamp == hour_start,
            "open-candle rows are not adjacent current-hour candles"
        );
        anyhow::ensure!(
            completed_close_at == last_completed.timestamp + Duration::hours(1)
                && expected_close_at == candle.timestamp + Duration::hours(1),
            "Binance kline close times do not match the 1-hour interval"
        );
        anyhow::ensure!(
            completed_close_at <= as_of,
            "previous Binance row is not completed"
        );
        anyhow::ensure!(
            candle.timestamp <= as_of && as_of < expected_close_at,
            "exchange reports this kline as closed or not yet open"
        );
        Ok(OpenCandlePair {
            last_completed,
            provisional: ProvisionalCandleSnapshot {
                candle,
                fetched_at: as_of,
                expected_close_at,
            },
        })
    }

    /// Fetches the latest completed Binance spot 1-hour bars for the trade
    /// volume lesson. This deliberately bypasses the historical CSV cache:
    /// field 7 is exchange-observed quote notional, not a derivable OHLCV
    /// teaching value.
    pub async fn fetch_completed_binance_trade_bars(
        &self,
        symbol: &str,
    ) -> Result<Vec<BinanceTradeBar>> {
        const COMPLETED_BARS: usize = 24;
        // Capture the exchange cutoff before requesting candles. A post-request
        // time can cross an hour boundary while the returned current candle was
        // still provisional, so it is diagnostic evidence only.
        let completion_cutoff = self.binance_server_time().await?;
        let start = completion_cutoff
            .with_minute(0)
            .and_then(|x| x.with_second(0))
            .and_then(|x| x.with_nanosecond(0))
            .context("invalid current time")?
            - Duration::hours(COMPLETED_BARS as i64 + 1);
        let raw: serde_json::Value = self
            .client
            .get(format!("{}/api/v3/klines", self.base_url))
            .query(&[
                ("symbol", symbol.replace('/', "")),
                ("interval", "1h".to_string()),
                ("startTime", start.timestamp_millis().to_string()),
                ("limit", (COMPLETED_BARS + 1).to_string()),
            ])
            .send()
            .await
            .context("trade-volume request failed")?
            .error_for_status()
            .context("trade-volume response failed")?
            .json()
            .await
            .context("invalid trade-volume JSON")?;
        let rows = raw
            .as_array()
            .context("trade-volume response must be kline array")?;
        let mut out = Vec::new();
        let mut previous_timestamp = None;
        for row in rows {
            let fields = row
                .as_array()
                .context("trade-volume kline must be an array")?;
            anyhow::ensure!(
                fields.len() == 12,
                "trade-volume kline must contain exactly 12 Binance fields"
            );
            let bar = parse_binance_kline(row).context("invalid trade-volume kline")?;
            let close_millis = fields
                .get(6)
                .and_then(serde_json::Value::as_i64)
                .context("trade-volume kline missing exchange close time")?;
            let closes_at = Utc
                .timestamp_millis_opt(close_millis.saturating_add(1))
                .single()
                .context("invalid trade-volume kline close time")?;
            let quote = row
                .as_array()
                .and_then(|fields| fields.get(7))
                .and_then(serde_json::Value::as_str)
                .and_then(|x| x.parse::<f64>().ok())
                .context("trade-volume kline missing quote asset volume")?;
            let taker_buy_base = fields
                .get(9)
                .and_then(serde_json::Value::as_str)
                .and_then(|x| x.parse::<f64>().ok())
                .context("trade-volume kline missing taker buy base asset volume")?;
            let taker_buy_quote = fields
                .get(10)
                .and_then(serde_json::Value::as_str)
                .and_then(|x| x.parse::<f64>().ok())
                .context("trade-volume kline missing taker buy quote asset volume")?;
            let trade_count = fields
                .get(8)
                .and_then(serde_json::Value::as_u64)
                .context("trade-volume kline missing nonnegative trade count")?;
            anyhow::ensure!(
                [
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                    quote,
                    taker_buy_base,
                    taker_buy_quote,
                ]
                .iter()
                .all(|value| value.is_finite())
                    && bar.open > 0.0
                    && bar.high > 0.0
                    && bar.low > 0.0
                    && bar.close > 0.0
                    && bar.high >= bar.open.max(bar.close)
                    && bar.low <= bar.open.min(bar.close)
                    && bar.volume >= 0.0
                    && quote >= 0.0,
                "invalid Binance trade-volume OHLCV"
            );
            anyhow::ensure!(
                taker_buy_base <= bar.volume && taker_buy_quote <= quote,
                "Binance taker-buy volume exceeds total kline volume"
            );
            anyhow::ensure!(
                (trade_count == 0) == (bar.volume == 0.0),
                "Binance trade count and total volume disagree"
            );
            anyhow::ensure!(
                closes_at == bar.timestamp + Duration::hours(1),
                "Binance trade-volume close time does not match the 1-hour interval"
            );
            anyhow::ensure!(
                previous_timestamp.is_none_or(|previous| previous < bar.timestamp),
                "Binance trade-volume timestamps are not strictly increasing"
            );
            anyhow::ensure!(
                previous_timestamp
                    .is_none_or(|previous| { bar.timestamp == previous + Duration::hours(1) }),
                "Binance trade-volume candles are not continuous 1-hour observations"
            );
            anyhow::ensure!(
                (bar.volume == 0.0) == (quote == 0.0),
                "Binance trade-volume base and quote volumes disagree"
            );
            previous_timestamp = Some(bar.timestamp);
            if closes_at <= completion_cutoff {
                out.push(BinanceTradeBar {
                    bar,
                    quote_volume: quote,
                    trade_count,
                    taker_buy_base_volume: taker_buy_base,
                    taker_buy_quote_volume: taker_buy_quote,
                });
            }
        }
        if out.len() > COMPLETED_BARS {
            out = out.split_off(out.len() - COMPLETED_BARS);
        }
        anyhow::ensure!(
            out.len() == COMPLETED_BARS,
            "trade-volume response has fewer than 24 completed candles"
        );
        Ok(out)
    }

    /// Both legs use one exchange cutoff and bypass the local teaching/cache files.
    pub async fn fetch_completed_spot_pair(
        &self,
        first: &str,
        second: &str,
        limit: usize,
    ) -> Result<(DateTime<Utc>, Vec<Bar>, Vec<Bar>)> {
        let cutoff = self.binance_server_time().await?;
        let since = cutoff
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            - Duration::hours(limit as i64);
        let (mut a, mut b) = tokio::try_join!(
            self.fetch_remote(first, since, limit),
            self.fetch_remote(second, since, limit)
        )?;
        for bars in [&mut a, &mut b] {
            bars.retain(|bar| {
                bar.timestamp >= since && bar.timestamp + Duration::hours(1) <= cutoff
            });
        }
        Ok((cutoff, a, b))
    }

    async fn fetch_remote(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>> {
        anyhow::ensure!(
            (1..=5000).contains(&limit),
            "limit must be between 1 and 5000"
        );
        let mut cursor = since.timestamp_millis();
        let mut bars = Vec::with_capacity(limit);
        let now = Utc::now();
        while bars.len() < limit {
            let batch_limit = (limit - bars.len()).min(1000);
            let raw: serde_json::Value = self
                .client
                .get(format!("{}/api/v3/klines", self.base_url))
                .query(&[
                    ("symbol", symbol.replace('/', "")),
                    ("interval", "1h".into()),
                    ("startTime", cursor.to_string()),
                    ("limit", batch_limit.to_string()),
                ])
                .send()
                .await
                .context("market request failed")?
                .error_for_status()
                .context("market returned HTTP error")?
                .json()
                .await
                .context("invalid market JSON")?;
            let rows = raw
                .as_array()
                .context("market response must be a kline array")?;
            if rows.is_empty() {
                break;
            }
            let parsed: Vec<Bar> = rows
                .iter()
                .map(|row| parse_binance_kline(row).context("invalid kline fields"))
                .collect::<Result<_>>()?;
            crate::practice::validate_bars(&parsed).map_err(anyhow::Error::msg)?;
            anyhow::ensure!(
                parsed[0].timestamp.timestamp_millis() >= cursor,
                "market pagination moved backwards"
            );
            let last = parsed.last().unwrap().timestamp;
            cursor = last.timestamp_millis() + 3_600_000;
            bars.extend(
                parsed
                    .into_iter()
                    .filter(|bar| bar.timestamp + Duration::hours(1) <= now),
            );
            if rows.len() < batch_limit || last + Duration::hours(1) > now {
                break;
            }
        }
        bars.truncate(limit);
        crate::practice::validate_bars(&bars).map_err(anyhow::Error::msg)?;
        Ok(bars)
    }
}

impl HttpFeed {
    async fn fetch_historical_snapshot(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<MarketSnapshot> {
        // 1. 先看本地缓存
        if let Ok(cached) = self.csv.load(symbol, "1h") {
            let cached_after: Vec<Bar> = cached
                .iter()
                .filter(|b| b.timestamp >= since && b.timestamp + Duration::hours(1) <= Utc::now())
                .cloned()
                .collect();
            if cached_after.len() >= limit {
                // Legacy CSV files contain no authenticated provider or
                // adjustment metadata. Do not turn cache availability into a
                // claim that these bytes were just verified at the exchange.
                return Ok(MarketSnapshot::new(
                    cached_after.into_iter().take(limit).collect(),
                    "local_csv_cache",
                    "local historical CSV cache",
                    "cache_price_basis_unverified",
                )
                .with_stock_split_coverage(StockSplitCoverage::NotApplicable));
            }
        }

        // 2. 拉远程
        let bars = self.fetch_remote(symbol, since, limit).await?;

        // 3. 写回缓存
        let mut merged = self.csv.load(symbol, "1h").unwrap_or_default();
        let incoming: std::collections::BTreeSet<_> = bars.iter().map(|b| b.timestamp).collect();
        merged.retain(|b| !incoming.contains(&b.timestamp));
        merged.extend(bars.iter().copied());
        merged.sort_by_key(|b| b.timestamp);
        merged.dedup_by_key(|b| b.timestamp);
        self.csv.save(symbol, "1h", &merged).ok();

        let official_endpoint = reqwest::Url::parse(&self.base_url).is_ok_and(|url| {
            url.scheme() == "https"
                && [
                    "data-api.binance.vision",
                    "api.binance.com",
                    "api1.binance.com",
                    "api2.binance.com",
                    "api3.binance.com",
                    "api4.binance.com",
                ]
                .contains(&url.host_str().unwrap_or(""))
        });
        Ok(MarketSnapshot::new(
            bars,
            if official_endpoint {
                "binance_spot"
            } else {
                "configured_crypto_endpoint"
            },
            &format!("{}/api/v3/klines", self.base_url.trim_end_matches('/')),
            if official_endpoint {
                "spot_trade_prices"
            } else {
                "configured_feed_unverified"
            },
        )
        .with_stock_split_coverage(StockSplitCoverage::NotApplicable))
    }
}

#[async_trait]
impl AsyncDataFeed for HttpFeed {
    async fn fetch_historical_async(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>> {
        Ok(self
            .fetch_historical_snapshot(symbol, since, limit)
            .await?
            .bars)
    }

    async fn stream_live_async(&self, symbol: &str) -> Result<Vec<Bar>> {
        let now = Utc::now();
        // Avoid replaying cached history as live events. Binance includes the
        // current unfinished 1h candle, which must not drive a strategy yet.
        let mut bars = self
            .fetch_remote(symbol, now - Duration::hours(3), 3)
            .await?;
        bars.retain(|bar| bar.timestamp + Duration::hours(1) <= now);
        bars.sort_by_key(|bar| bar.timestamp);
        bars.dedup_by_key(|bar| bar.timestamp);
        Ok(bars)
    }
}

/// Real market sources exposed to learners. They all normalize their response
/// into the same completed-OHLCV contract before callers see a candle.
pub const PUBLIC_MARKET_SOURCES: &[&str] = &["binance", "a_share", "us_stock"];

pub fn is_public_market_source(source: &str) -> bool {
    PUBLIC_MARKET_SOURCES.contains(&source)
}

pub fn source_symbols(source: &str) -> Vec<String> {
    match source {
        "a_share" => ["600519", "000001", "300750", "601318"]
            .into_iter()
            .map(String::from)
            .collect(),
        "us_stock" => ["AAPL", "MSFT", "NVDA", "SPY"]
            .into_iter()
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

fn complete_daily_bar(
    date: &str,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
) -> Option<Bar> {
    let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let timestamp = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0)?);
    let bar = Bar {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    };
    crate::practice::validate_bars(&[bar]).ok()?;
    Some(bar)
}

fn parse_eastmoney_kline(row: &str) -> Option<Bar> {
    let fields: Vec<_> = row.split(',').collect();
    // date, open, close, high, low, volume, turnover...
    complete_daily_bar(
        fields.first()?.trim(),
        fields.get(1)?.trim().parse().ok()?,
        fields.get(3)?.trim().parse().ok()?,
        fields.get(4)?.trim().parse().ok()?,
        fields.get(2)?.trim().parse().ok()?,
        fields.get(5)?.trim().parse().ok()?,
    )
}

fn parse_aapl_split_adjustment_evidence(
    raw: &serde_json::Value,
    endpoint: String,
    fetched_at: DateTime<Utc>,
) -> Result<StockAdjustmentEvidence> {
    let result = raw
        .pointer("/chart/result/0")
        .context("AAPL adjustment response has no result")?;
    anyhow::ensure!(
        result
            .pointer("/meta/symbol")
            .and_then(serde_json::Value::as_str)
            == Some("AAPL")
            && result
                .pointer("/meta/currency")
                .and_then(serde_json::Value::as_str)
                == Some("USD")
            && matches!(
                result
                    .pointer("/meta/instrumentType")
                    .and_then(serde_json::Value::as_str),
                Some("EQUITY" | "ETF")
            ),
        "AAPL adjustment response identity does not match"
    );
    let times = result["timestamp"]
        .as_array()
        .context("AAPL adjustment timestamps missing")?;
    let quote = result
        .pointer("/indicators/quote/0")
        .context("AAPL provider quotes missing")?;
    let adjusted = result
        .pointer("/indicators/adjclose/0/adjclose")
        .and_then(serde_json::Value::as_array)
        .context("AAPL provider adjusted closes missing")?;
    let arrays = ["open", "high", "low", "close", "volume"]
        .map(|field| {
            quote[field]
                .as_array()
                .with_context(|| format!("AAPL provider quote {field} missing"))
        })
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(
        !times.is_empty()
            && adjusted.len() == times.len()
            && arrays.iter().all(|values| values.len() == times.len()),
        "AAPL provider quote and adjusted close observations are not aligned"
    );

    let mut observations = Vec::with_capacity(times.len());
    for index in 0..times.len() {
        let timestamp = Utc
            .timestamp_opt(
                times[index]
                    .as_i64()
                    .context("AAPL adjustment timestamp must be an integer")?,
                0,
            )
            .single()
            .context("AAPL adjustment timestamp is invalid")?;
        let number = |values: &[serde_json::Value], field: &str| -> Result<f64> {
            let value = values[index]
                .as_f64()
                .with_context(|| format!("AAPL {field} observation is missing"))?;
            anyhow::ensure!(value.is_finite(), "AAPL {field} observation is not finite");
            Ok(value)
        };
        let bar = Bar {
            timestamp,
            open: number(arrays[0], "open")?,
            high: number(arrays[1], "high")?,
            low: number(arrays[2], "low")?,
            close: number(arrays[3], "close")?,
            volume: number(arrays[4], "volume")?,
        };
        crate::practice::validate_bars(&[bar]).map_err(anyhow::Error::msg)?;
        let adjusted_close = number(adjusted, "adjusted close")?;
        anyhow::ensure!(adjusted_close > 0.0, "AAPL adjusted close must be positive");
        observations.push(StockAdjustmentObservation {
            timestamp,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            adjusted_close,
        });
    }
    anyhow::ensure!(
        observations
            .windows(2)
            .all(|pair| pair[0].timestamp < pair[1].timestamp),
        "AAPL adjustment observations are not strictly chronological"
    );

    let target_date = NaiveDate::from_ymd_opt(2020, 8, 31).expect("valid fixed case date");
    let splits = result
        .pointer("/events/splits")
        .and_then(serde_json::Value::as_object)
        .context("AAPL adjustment response has no dated split events")?;
    let event = splits
        .values()
        .find_map(|value| {
            let effective_at = Utc.timestamp_opt(value["date"].as_i64()?, 0).single()?;
            let numerator = value["numerator"].as_f64()?;
            let denominator = value["denominator"].as_f64()?;
            let split_ratio = value["splitRatio"].as_str()?.to_owned();
            (effective_at.date_naive() == target_date
                && numerator == 4.0
                && denominator == 1.0
                && split_ratio == "4:1")
                .then_some(StockSplitEvent {
                    kind: "split",
                    effective_at,
                    effective_trading_date: target_date,
                    numerator,
                    denominator,
                    split_ratio,
                })
        })
        .context("AAPL 2020-08-31 4:1 split event is missing")?;
    anyhow::ensure!(
        observations
            .iter()
            .any(|item| item.timestamp.date_naive() < target_date)
            && observations
                .iter()
                .any(|item| item.timestamp.date_naive() == target_date),
        "AAPL adjustment evidence needs provider observations before and on the split date"
    );

    let provider = reqwest::Url::parse(&endpoint)
        .ok()
        .and_then(|url| {
            (url.scheme() == "https"
                && matches!(
                    url.host_str(),
                    Some("query1.finance.yahoo.com" | "query2.finance.yahoo.com")
                ))
            .then_some("yahoo")
        })
        .unwrap_or("configured_endpoint");
    Ok(StockAdjustmentEvidence {
        provider,
        endpoint,
        fetched_at,
        retrieval: "live_provider_response",
        scope: "aapl_2020_4_for_1_split_historical_window",
        event,
        issuer_confirmation_url:
            "https://www.apple.com/newsroom/2020/07/apple-reports-third-quarter-results/",
        issuer_confirmation:
            "Apple announced a four-for-one split with split-adjusted trading beginning 2020-08-31",
        observations,
        quote_basis: "provider_quote_semantics_unverified_for_split_adjustment",
        adjusted_close_basis: "provider_adjusted_close_semantics_unverified_for_total_return",
        calculation: "split_only_price_multiplier = denominator / numerator",
    })
}

fn parse_yahoo_bars(raw: &serde_json::Value) -> Result<Vec<Bar>> {
    parse_yahoo_bars_at(raw, Utc::now())
}

fn yahoo_stock_split_coverage(raw: &serde_json::Value, bars: &[Bar]) -> StockSplitCoverage {
    let Some(first) = bars.first().map(|bar| bar.timestamp.date_naive()) else {
        return StockSplitCoverage::Unverified {
            reason: "Yahoo returned no bars for split-window verification".into(),
        };
    };
    let last = bars
        .last()
        .expect("a first bar implies a last bar")
        .timestamp
        .date_naive();
    let result = match raw.pointer("/chart/result/0") {
        Some(result) => result,
        None => {
            return StockSplitCoverage::Unverified {
                reason: "Yahoo response has no result for split-window verification".into(),
            };
        }
    };
    let Some(splits) = result.pointer("/events/splits") else {
        // Yahoo omits the events object when a requested event set is empty.
        return StockSplitCoverage::VerifiedNoSplitInWindow {
            evidence: "yahoo_events_splits_requested_and_absent".into(),
        };
    };
    let Some(splits) = splits.as_object() else {
        return StockSplitCoverage::Unverified {
            reason: "Yahoo split events are not an object".into(),
        };
    };
    let mut event_dates = Vec::new();
    for split in splits.values() {
        let valid_ratio = split["numerator"]
            .as_f64()
            .is_some_and(|value| value.is_finite() && value > 0.0)
            && split["denominator"]
                .as_f64()
                .is_some_and(|value| value.is_finite() && value > 0.0);
        let Some(event_date) = split["date"]
            .as_i64()
            .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0))
            .map(|timestamp| timestamp.date_naive())
        else {
            return StockSplitCoverage::Unverified {
                reason: "Yahoo split event has no valid date".into(),
            };
        };
        if !valid_ratio {
            return StockSplitCoverage::Unverified {
                reason: "Yahoo split event has no valid ratio".into(),
            };
        }
        if (first..=last).contains(&event_date) {
            event_dates.push(event_date);
        }
    }
    event_dates.sort_unstable();
    event_dates.dedup();
    if event_dates.is_empty() {
        StockSplitCoverage::VerifiedNoSplitInWindow {
            evidence: "yahoo_events_splits_requested_no_event_in_returned_window".into(),
        }
    } else {
        StockSplitCoverage::CrossesWindow {
            evidence: "yahoo_chart_split_events".into(),
            event_dates,
        }
    }
}

fn parse_yahoo_bars_at(raw: &serde_json::Value, now: DateTime<Utc>) -> Result<Vec<Bar>> {
    let result = raw
        .pointer("/chart/result/0")
        .context("Yahoo response has no result")?;
    let regular_session = result
        .pointer("/meta/currentTradingPeriod/regular")
        .and_then(|period| {
            Some((
                period["start"].as_i64()?,
                period["end"].as_i64()?,
                result["meta"]["gmtoffset"].as_i64()?,
            ))
        });
    let times = result["timestamp"]
        .as_array()
        .context("Yahoo timestamps missing")?;
    let quote = result
        .pointer("/indicators/quote/0")
        .context("Yahoo quotes missing")?;
    let opens = quote["open"].as_array().context("Yahoo opens missing")?;
    let highs = quote["high"].as_array().context("Yahoo highs missing")?;
    let lows = quote["low"].as_array().context("Yahoo lows missing")?;
    let closes = quote["close"].as_array().context("Yahoo closes missing")?;
    let volumes = quote["volume"]
        .as_array()
        .context("Yahoo volumes missing")?;
    let mut bars = Vec::new();
    for (i, time) in times.iter().enumerate() {
        let Some(ts) = time.as_i64() else {
            continue;
        };
        let (Some(open), Some(high), Some(low), Some(close), Some(volume)) = (
            opens.get(i).and_then(serde_json::Value::as_f64),
            highs.get(i).and_then(serde_json::Value::as_f64),
            lows.get(i).and_then(serde_json::Value::as_f64),
            closes.get(i).and_then(serde_json::Value::as_f64),
            volumes.get(i).and_then(serde_json::Value::as_f64),
        ) else {
            continue;
        };
        let Some(timestamp) = Utc.timestamp_opt(ts, 0).single() else {
            continue;
        };
        if !yahoo_bar_is_complete(timestamp, regular_session, now) {
            continue;
        }
        // Yahoo labels a US daily bar at local midnight (04:00/05:00 UTC).
        // Normalize its already-identified trading date to this project's UTC
        // daily-date convention; completeness remains checked against the raw time.
        let timestamp = Utc.from_utc_datetime(
            &timestamp
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("midnight is a valid time"),
        );
        let bar = Bar {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        };
        if crate::practice::validate_bars(&[bar]).is_ok() {
            bars.push(bar);
        }
    }
    bars.sort_by_key(|bar| bar.timestamp);
    bars.dedup_by_key(|bar| bar.timestamp);
    crate::practice::validate_bars(&bars).map_err(anyhow::Error::msg)?;
    Ok(bars)
}

fn yahoo_bar_is_complete(
    timestamp: DateTime<Utc>,
    regular_session: Option<(i64, i64, i64)>,
    now: DateTime<Utc>,
) -> bool {
    let Some((session_start, session_end, gmtoffset)) = regular_session else {
        return timestamp.date_naive() < now.date_naive();
    };
    let Some(session_date) = DateTime::from_timestamp(session_start, 0)
        .and_then(|value| value.checked_add_signed(Duration::seconds(gmtoffset)))
        .map(|value| value.date_naive())
    else {
        return timestamp.date_naive() < now.date_naive();
    };
    let Some(bar_local_date) = timestamp
        .checked_add_signed(Duration::seconds(gmtoffset))
        .map(|value| value.date_naive())
    else {
        return false;
    };
    bar_local_date < session_date
        || (bar_local_date == session_date && now.timestamp() >= session_end)
}

fn json_number(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn parse_tencent_a_share_bars(raw: &serde_json::Value, market_symbol: &str) -> Result<Vec<Bar>> {
    parse_tencent_a_share_bars_with_key(raw, market_symbol, "day")
        .context("Tencent A-share response has no unadjusted completed daily rows")
}

fn parse_tencent_a_share_bars_with_key(
    raw: &serde_json::Value,
    market_symbol: &str,
    key: &str,
) -> Result<Vec<Bar>> {
    let rows = raw
        .pointer(&format!("/data/{market_symbol}/{key}"))
        .and_then(serde_json::Value::as_array)
        .with_context(|| format!("Tencent A-share response has no {key} completed daily rows"))?;
    let bars: Vec<Bar> = rows
        .iter()
        .filter_map(|row| {
            let row = row.as_array()?;
            complete_daily_bar(
                row.first()?.as_str()?,
                json_number(row.get(1)?)?,
                json_number(row.get(3)?)?,
                json_number(row.get(4)?)?,
                json_number(row.get(2)?)?,
                json_number(row.get(5)?)?,
            )
        })
        .collect();
    crate::practice::validate_bars(&bars).map_err(anyhow::Error::msg)?;
    Ok(bars)
}

async fn fetch_a_share_tencent_at(
    client: &reqwest::Client,
    endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
    adjustment: Option<&str>,
) -> Result<Vec<Bar>> {
    let exchange = match symbol.as_bytes().first() {
        Some(b'6') | Some(b'9') if !symbol.starts_with("92") => "sh",
        Some(b'4') | Some(b'8') | Some(b'9') => "bj",
        _ => "sz",
    };
    let market_symbol = format!("{exchange}{symbol}");
    let suffix = adjustment.unwrap_or("");
    let raw: serde_json::Value = client
        .get(endpoint)
        .header(
            reqwest::header::USER_AGENT,
            "AXIOM educational market reader/1.0",
        )
        .query(&[("param", format!("{market_symbol},day,,,{limit},{suffix}"))])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("Tencent A-share market request failed")?
        .error_for_status()
        .context("Tencent A-share market returned HTTP error")?
        .json()
        .await
        .context("invalid Tencent A-share market JSON")?;
    let bars = if adjustment == Some("qfq") {
        parse_tencent_a_share_bars_with_key(&raw, &market_symbol, "qfqday")?
    } else {
        parse_tencent_a_share_bars(&raw, &market_symbol)?
    };
    Ok(latest_daily_bars(bars, since, limit))
}

fn latest_daily_bars(mut bars: Vec<Bar>, since: DateTime<Utc>, limit: usize) -> Vec<Bar> {
    bars.retain(|bar| bar.timestamp >= since);
    if bars.len() > limit {
        bars = bars.split_off(bars.len() - limit);
    }
    bars
}

async fn fetch_a_share_snapshot(
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    let client = reqwest::Client::new();
    fetch_a_share_snapshot_at(
        &client,
        "https://push2his.eastmoney.com/api/qt/stock/kline/get",
        "https://web.ifzq.gtimg.cn/appstock/app/fqkline/get",
        symbol,
        since,
        limit,
    )
    .await
}

#[cfg(test)]
async fn fetch_a_share_at(
    client: &reqwest::Client,
    eastmoney_endpoint: &str,
    tencent_endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>> {
    Ok(fetch_a_share_snapshot_at(
        client,
        eastmoney_endpoint,
        tencent_endpoint,
        symbol,
        since,
        limit,
    )
    .await?
    .bars)
}

async fn fetch_a_share_snapshot_at(
    client: &reqwest::Client,
    eastmoney_endpoint: &str,
    tencent_endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    anyhow::ensure!(
        symbol.len() == 6 && symbol.bytes().all(|byte| byte.is_ascii_digit()),
        "A-share symbol must be a six digit code"
    );
    let exchange = if symbol.starts_with('6') || symbol.starts_with("900") {
        "1"
    } else {
        // Eastmoney uses market 0 for Shenzhen and Beijing listings.
        "0"
    };
    let request = client
        .get(eastmoney_endpoint)
        .header(
            reqwest::header::USER_AGENT,
            "AXIOM educational market reader/1.0",
        )
        .query(&[
            ("secid", format!("{exchange}.{symbol}")),
            ("klt", "101".to_owned()),
            ("fqt", "0".to_owned()),
            ("lmt", limit.min(5000).to_string()),
            ("beg", since.format("%Y%m%d").to_string()),
            ("end", Utc::now().format("%Y%m%d").to_string()),
            ("fields1", "f1,f2,f3,f4,f5,f6".to_owned()),
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61".to_owned(),
            ),
        ]);
    // Keep one completed predecessor outside the requested window. A qfq
    // scale change on the first selected bar is otherwise indistinguishable
    // from a constant transform and could be falsely reported as no split.
    let evidence_since = since - Duration::days(14);
    let evidence_limit = limit.saturating_add(1).min(5000);
    let (primary, tencent_evidence, tencent_qfq) = tokio::join!(
        async {
            let raw: serde_json::Value = request
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .context("A-share market request failed")?
                .error_for_status()
                .context("A-share market returned HTTP error")?
                .json()
                .await
                .context("invalid A-share market JSON")?;
            let rows = raw
                .pointer("/data/klines")
                .and_then(serde_json::Value::as_array)
                .context("A-share response has no kline rows")?;
            let bars: Vec<Bar> = rows
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter_map(parse_eastmoney_kline)
                .collect();
            crate::practice::validate_bars(&bars).map_err(anyhow::Error::msg)?;
            Ok::<MarketSnapshot, anyhow::Error>(MarketSnapshot::new(
                latest_daily_bars(bars, since, limit),
                "eastmoney",
                eastmoney_endpoint,
                "unadjusted_requested",
            ))
        },
        async {
            fetch_a_share_tencent_at(
                client,
                tencent_endpoint,
                symbol,
                evidence_since,
                evidence_limit,
                None,
            )
            .await
            .map(|bars| {
                MarketSnapshot::new(bars, "tencent", tencent_endpoint, "unadjusted_requested")
            })
        },
        fetch_a_share_tencent_at(
            client,
            tencent_endpoint,
            symbol,
            evidence_since,
            evidence_limit,
            Some("qfq"),
        )
    );
    let tencent_raw = tencent_evidence
        .as_ref()
        .ok()
        .map(|snapshot| snapshot.bars.clone());
    let tencent = tencent_evidence.map(|mut snapshot| {
        snapshot.bars = latest_daily_bars(snapshot.bars, since, limit);
        snapshot
    });
    let mut selected = select_a_share_daily_bars(primary, tencent, Utc::now())?;
    selected.stock_split_coverage = match (tencent_raw.as_deref(), tencent_qfq.as_deref()) {
        (Some(raw), Ok(adjusted)) => a_share_stock_split_coverage(&selected.bars, raw, adjusted),
        (_, Err(error)) => StockSplitCoverage::Unverified {
            reason: format!("Tencent qfq split cross-check failed: {error}"),
        },
        (None, _) => StockSplitCoverage::Unverified {
            reason: "Tencent raw series was unavailable for split cross-check".into(),
        },
    };
    Ok(selected)
}

fn a_share_stock_split_coverage(
    selected: &[Bar],
    raw: &[Bar],
    adjusted: &[Bar],
) -> StockSplitCoverage {
    let (Some(selected_first), Some(selected_last)) = (selected.first(), selected.last()) else {
        return StockSplitCoverage::Unverified {
            reason: "A-share split cross-check received no selected bars".into(),
        };
    };
    if raw.first().map(|bar| bar.timestamp) > Some(selected_first.timestamp)
        || raw.last().map(|bar| bar.timestamp) < Some(selected_last.timestamp)
        || raw.len() != adjusted.len()
        || raw
            .iter()
            .zip(adjusted)
            .any(|(left, right)| left.timestamp != right.timestamp)
    {
        return StockSplitCoverage::Unverified {
            reason: "Tencent raw/qfq dates do not cover the selected A-share window".into(),
        };
    }

    for selected_bar in selected {
        let Some(raw_bar) = raw
            .iter()
            .find(|candidate| candidate.timestamp == selected_bar.timestamp)
        else {
            return StockSplitCoverage::Unverified {
                reason: "Tencent raw series is missing a selected A-share date".into(),
            };
        };
        let same_unadjusted_prices = [
            (selected_bar.open, raw_bar.open),
            (selected_bar.high, raw_bar.high),
            (selected_bar.low, raw_bar.low),
            (selected_bar.close, raw_bar.close),
        ]
        .into_iter()
        .all(|(selected, raw)| (selected - raw).abs() <= 0.021);
        if !same_unadjusted_prices {
            return StockSplitCoverage::Unverified {
                reason: "selected A-share OHLC differs from Tencent unadjusted cross-check".into(),
            };
        }
    }

    let mut scales = Vec::new();
    for (raw_bar, adjusted_bar) in raw.iter().zip(adjusted) {
        let raw_range = raw_bar.high - raw_bar.low;
        let adjusted_range = adjusted_bar.high - adjusted_bar.low;
        // Ignore narrow rows where provider mill precision could dominate the
        // range ratio. A verified window needs several informative rows.
        if raw_range < 1.0 || adjusted_range <= 0.0 {
            continue;
        }
        let scale = adjusted_range / raw_range;
        if !scale.is_finite() || scale <= 0.0 {
            return StockSplitCoverage::Unverified {
                reason: "Tencent raw/qfq price transform is invalid".into(),
            };
        }
        scales.push((raw_bar.timestamp.date_naive(), scale));
    }
    if scales.len() < 3 {
        return StockSplitCoverage::Unverified {
            reason: "Tencent raw/qfq series has too few informative rows".into(),
        };
    }
    if !scales
        .iter()
        .any(|(date, _)| *date < selected_first.timestamp.date_naive())
    {
        return StockSplitCoverage::Unverified {
            reason: "Tencent raw/qfq series lacks a pre-window scale observation".into(),
        };
    }
    let mut event_dates = Vec::new();
    for pair in scales.windows(2) {
        let relative_change = (pair[1].1 / pair[0].1 - 1.0).abs();
        if relative_change > 0.005 {
            event_dates.push(pair[1].0);
        }
    }
    event_dates.sort_unstable();
    event_dates.dedup();
    if event_dates.is_empty() {
        StockSplitCoverage::VerifiedNoSplitInWindow {
            evidence: "tencent_raw_qfq_ohlc_range_scale_constant".into(),
        }
    } else {
        StockSplitCoverage::CrossesWindow {
            evidence: "tencent_raw_qfq_ohlc_range_scale_change".into(),
            event_dates,
        }
    }
}

fn select_a_share_daily_bars<T: AsRef<[Bar]>>(
    primary: Result<T>,
    fallback: Result<T>,
    now: DateTime<Utc>,
) -> Result<T> {
    let primary_current = primary
        .as_ref()
        .is_ok_and(|bars| has_current_daily_bars(bars.as_ref(), now));
    let fallback_current = fallback
        .as_ref()
        .is_ok_and(|bars| has_current_daily_bars(bars.as_ref(), now));
    match (primary_current, fallback_current) {
        (true, true) => {
            let primary = primary.expect("current primary result was checked above");
            let fallback = fallback.expect("current fallback result was checked above");
            if fallback.as_ref().last().map(|bar| bar.timestamp)
                > primary.as_ref().last().map(|bar| bar.timestamp)
            {
                Ok(fallback)
            } else {
                Ok(primary)
            }
        }
        (true, false) => primary,
        (false, true) => fallback,
        // A calendar-age check cannot distinguish an upstream outage from a
        // valid multi-day exchange closure. When neither provider has a
        // weekday-recent bar, retain the newest structurally valid completed
        // series within a bounded closure window; callers disclose its last
        // observation timestamp rather than presenting it as today's price.
        (false, false) => {
            let bars = newest_nonempty_daily_series(primary, fallback)?;
            let age_days = now
                .date_naive()
                .signed_duration_since(bars.as_ref().last().unwrap().timestamp.date_naive())
                .num_days();
            anyhow::ensure!(
                (0..=21).contains(&age_days),
                "A-share latest completed daily bar is {age_days} calendar days old; provider history requires verification"
            );
            Ok(bars)
        }
    }
}

fn newest_nonempty_daily_series<T: AsRef<[Bar]>>(
    primary: Result<T>,
    fallback: Result<T>,
) -> Result<T> {
    match (primary, fallback) {
        (Ok(primary), Ok(fallback))
            if !primary.as_ref().is_empty() && !fallback.as_ref().is_empty() =>
        {
            if fallback.as_ref().last().map(|bar| bar.timestamp)
                > primary.as_ref().last().map(|bar| bar.timestamp)
            {
                Ok(fallback)
            } else {
                Ok(primary)
            }
        }
        (Ok(primary), _) if !primary.as_ref().is_empty() => Ok(primary),
        (_, Ok(fallback)) if !fallback.as_ref().is_empty() => Ok(fallback),
        (primary, fallback) => {
            let primary_error = primary
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "primary returned no daily rows".into());
            let fallback_error = fallback
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "Tencent returned no daily rows".into());
            anyhow::bail!(
                "no usable A-share daily series; primary: {primary_error}; Tencent: {fallback_error}"
            )
        }
    }
}

fn has_current_daily_bars(bars: &[Bar], now: DateTime<Utc>) -> bool {
    let cutoff = match now.weekday() {
        Weekday::Sat => now.date_naive() - Duration::days(1),
        Weekday::Sun => now.date_naive() - Duration::days(2),
        Weekday::Mon => now.date_naive() - Duration::days(3),
        _ => now.date_naive() - Duration::days(1),
    };
    bars.last().is_some_and(|bar| {
        let date = bar.timestamp.date_naive();
        date >= cutoff && date <= now.date_naive()
    })
}

fn display_number(value: &serde_json::Value) -> Option<f64> {
    value.as_str()?.replace(['$', ','], "").parse().ok()
}

fn parse_nasdaq_bars(raw: &serde_json::Value) -> Result<Vec<Bar>> {
    let rows = raw
        .pointer("/data/tradesTable/rows")
        .and_then(serde_json::Value::as_array)
        .context("Nasdaq response has no daily rows")?;
    let mut bars: Vec<Bar> = rows
        .iter()
        .filter_map(|row| {
            let date = row.get("date")?.as_str()?;
            let day = NaiveDate::parse_from_str(date, "%m/%d/%Y").ok()?;
            let timestamp = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0)?);
            let bar = Bar {
                timestamp,
                open: display_number(row.get("open")?)?,
                high: display_number(row.get("high")?)?,
                low: display_number(row.get("low")?)?,
                close: display_number(row.get("close")?)?,
                volume: display_number(row.get("volume")?)?,
            };
            (crate::practice::validate_bars(&[bar]).is_ok()).then_some(bar)
        })
        .collect();
    bars.sort_by_key(|bar| bar.timestamp);
    crate::practice::validate_bars(&bars).map_err(anyhow::Error::msg)?;
    Ok(bars)
}

async fn fetch_us_stock_nasdaq_at(
    client: &reqwest::Client,
    endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    // Nasdaq separates common shares and exchange-traded funds by asset class.
    // The catalog includes both, so a missing stock result must retry as an ETF.
    let mut last_error = String::new();
    for assetclass in ["stocks", "etf"] {
        let attempt: Result<MarketSnapshot> = async {
            let response = client
                .get(format!("{endpoint}/{symbol}/historical"))
                .header(
                    reqwest::header::USER_AGENT,
                    "Mozilla/5.0 (compatible; AXIOM educational market reader/1.0)",
                )
                .header(reqwest::header::ACCEPT, "application/json")
                .query(&[
                    ("assetclass", assetclass.to_owned()),
                    ("fromdate", since.format("%Y-%m-%d").to_string()),
                    ("todate", Utc::now().format("%Y-%m-%d").to_string()),
                    ("limit", limit.min(5_000).to_string()),
                ])
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .context("Nasdaq market request failed")?
                .error_for_status()
                .context("Nasdaq market returned HTTP error")?;
            let selected_endpoint = response.url().to_string();
            let raw: serde_json::Value = response
                .json()
                .await
                .context("invalid Nasdaq market JSON")?;
            Ok(MarketSnapshot::new(
                latest_daily_bars(parse_nasdaq_bars(&raw)?, since, limit),
                "nasdaq",
                &selected_endpoint,
                "provider_adjustment_unverified",
            ))
        }
        .await;
        match attempt {
            Ok(snapshot) if !snapshot.bars.is_empty() => return Ok(snapshot),
            Ok(_) => last_error = format!("{assetclass} returned no daily rows"),
            Err(error) => last_error = format!("{assetclass}: {error}"),
        }
    }
    anyhow::bail!("Nasdaq market has no usable daily rows for {symbol}: {last_error}")
}

async fn fetch_us_stock_snapshot(
    feed: &HttpFeed,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    fetch_us_stock_snapshot_at(
        &feed.client,
        YahooStockSources {
            primary: &feed.us_stock_adjustment_url,
            fallback: &feed.us_stock_adjustment_fallback_url,
            relay: feed.us_stock_relay_url.as_deref(),
        },
        "https://api.nasdaq.com/api/quote",
        symbol,
        since,
        limit,
    )
    .await
}

fn market_snapshot_from_yahoo(
    raw: &serde_json::Value,
    selected_endpoint: &str,
    provider: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    let result = raw
        .pointer("/chart/result/0")
        .context("Yahoo chart response has no result")?;
    let expected_symbol = symbol.replace('.', "-");
    anyhow::ensure!(
        raw.pointer("/chart/error")
            .is_some_and(serde_json::Value::is_null)
            && result
                .pointer("/meta/symbol")
                .and_then(serde_json::Value::as_str)
                == Some(expected_symbol.as_str())
            && result
                .pointer("/meta/currency")
                .and_then(serde_json::Value::as_str)
                == Some("USD")
            && matches!(
                result
                    .pointer("/meta/instrumentType")
                    .and_then(serde_json::Value::as_str),
                Some("EQUITY" | "ETF")
            ),
        "Yahoo chart response identity does not match"
    );
    let mut bars = parse_yahoo_bars(raw)?;
    bars.retain(|bar| bar.timestamp >= since);
    if bars.len() > limit {
        bars = bars.split_off(bars.len() - limit);
    }
    anyhow::ensure!(!bars.is_empty(), "Yahoo returned no daily rows");
    let coverage = yahoo_stock_split_coverage(raw, &bars);
    Ok(MarketSnapshot::new(
        bars,
        provider,
        selected_endpoint,
        "provider_adjustment_unverified",
    )
    .with_stock_split_coverage(coverage))
}

async fn fetch_yahoo_stock_snapshot_at(
    client: &reqwest::Client,
    yahoo_endpoints: [&str; 2],
    symbol: &str,
    since: DateTime<Utc>,
    period1: i64,
    period2: i64,
    limit: usize,
) -> Result<MarketSnapshot> {
    let query = [
        ("period1", period1.to_string()),
        ("period2", period2.to_string()),
        ("interval", "1d".to_owned()),
        ("includePrePost", "false".to_owned()),
        ("events", "div,splits".to_owned()),
    ];
    let mut errors = Vec::new();
    for yahoo_endpoint in yahoo_endpoints {
        let endpoint = format!("{yahoo_endpoint}/{}", symbol.replace('.', "-"));
        for attempt in 1..=2 {
            let (raw, selected_endpoint) =
                match request_yahoo_chart_json(client, &endpoint, &query).await {
                    Ok(value) => value,
                    Err(failure) => {
                        errors.push(format!("attempt {attempt} {endpoint}: {:#}", failure.error));
                        if failure.retryable && attempt == 1 {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                        break;
                    }
                };
            let provider = reqwest::Url::parse(&selected_endpoint)
                .ok()
                .and_then(|url| {
                    matches!(
                        url.host_str(),
                        Some("query1.finance.yahoo.com" | "query2.finance.yahoo.com")
                    )
                    .then_some("yahoo")
                })
                .unwrap_or("configured_yahoo_endpoint");
            match market_snapshot_from_yahoo(
                &raw,
                &selected_endpoint,
                provider,
                symbol,
                since,
                limit,
            ) {
                Ok(snapshot) => return Ok(snapshot),
                Err(error) => {
                    errors.push(format!("{endpoint}: {error:#}"));
                    break;
                }
            }
        }
    }
    anyhow::bail!("Yahoo market endpoints failed: {}", errors.join(" | "))
}

struct YahooStockSources<'a> {
    primary: &'a str,
    fallback: &'a str,
    relay: Option<&'a str>,
}

fn valid_us_chart_symbol(symbol: &str) -> bool {
    symbol.len() <= 30
        && symbol
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_uppercase)
        && symbol.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        })
        && !symbol.split(['.', '-']).any(str::is_empty)
}

async fn fetch_us_stock_snapshot_at(
    client: &reqwest::Client,
    yahoo: YahooStockSources<'_>,
    nasdaq_endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    anyhow::ensure!(
        valid_us_chart_symbol(symbol),
        "US symbol must start with an uppercase letter and contain at most 30 uppercase letters, digits, dots or dashes with nonempty segments"
    );
    let period1 = Utc
        .from_utc_datetime(
            &since
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("a UTC date always has a midnight"),
        )
        .timestamp();
    let period2 = Utc::now().timestamp();
    match fetch_yahoo_stock_snapshot_at(
        client,
        [yahoo.primary, yahoo.fallback],
        symbol,
        since,
        period1,
        period2,
        limit,
    )
    .await
    {
        Ok(snapshot) => Ok(snapshot),
        Err(direct_error) => {
            if let Some(relay_url) = yahoo.relay {
                let period1 = period1.to_string();
                let period2 = period2.to_string();
                match request_yahoo_relay_json(client, relay_url, symbol, &period1, &period2)
                    .await
                    .and_then(|(raw, source_url)| {
                        market_snapshot_from_yahoo(
                            &raw,
                            &source_url,
                            "yahoo_via_restricted_relay",
                            symbol,
                            since,
                            limit,
                        )
                    }) {
                    Ok(snapshot) => return Ok(snapshot),
                    Err(relay_error) => {
                        return fetch_us_stock_nasdaq_at(
                            client,
                            nasdaq_endpoint,
                            symbol,
                            since,
                            limit,
                        )
                        .await
                        .with_context(|| {
                            format!(
                                "Yahoo direct and restricted relay failed: {direct_error:#}; {relay_error:#}"
                            )
                        });
                    }
                }
            }
            fetch_us_stock_nasdaq_at(client, nasdaq_endpoint, symbol, since, limit)
                .await
                .with_context(|| format!("Yahoo direct endpoints failed: {direct_error:#}"))
        }
    }
}

/// Fetch completed candles from a named free public source. Source-specific
/// response peculiarities, daily session rules and parsers stay behind this seam.
#[derive(Debug, Clone, Serialize)]
pub struct MarketProvenance {
    pub provider: String,
    pub endpoint: String,
    pub price_basis: String,
    pub corporate_actions: String,
}

/// Evidence that the returned stock-price window can be simulated without
/// applying an unverified share-ratio corporate action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StockSplitCoverage {
    NotApplicable,
    VerifiedNoSplitInWindow {
        evidence: String,
    },
    CrossesWindow {
        evidence: String,
        event_dates: Vec<NaiveDate>,
    },
    Unverified {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockSplitGuardError {
    pub code: &'static str,
    pub detail: String,
}

impl std::fmt::Display for StockSplitGuardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

/// Require affirmative split coverage before simulating a live stock window.
/// Event dates outside the final, completed-bar window do not block execution.
pub fn guard_stock_backtest_window(
    source: &str,
    coverage: &StockSplitCoverage,
    bars: &[Bar],
) -> std::result::Result<StockSplitCoverage, StockSplitGuardError> {
    if !matches!(source, "a_share" | "us_stock") {
        return Ok(coverage.clone());
    }
    let first = bars.first().map(|bar| bar.timestamp.date_naive());
    let last = bars.last().map(|bar| bar.timestamp.date_naive());
    match coverage {
        StockSplitCoverage::VerifiedNoSplitInWindow { .. } => Ok(coverage.clone()),
        StockSplitCoverage::CrossesWindow {
            evidence,
            event_dates,
        } => {
            let in_window: Vec<_> = event_dates
                .iter()
                .copied()
                .filter(|date| {
                    first
                        .zip(last)
                        .is_some_and(|(first, last)| (first..=last).contains(date))
                })
                .collect();
            if in_window.is_empty() {
                Ok(StockSplitCoverage::VerifiedNoSplitInWindow {
                    evidence: format!("{evidence}_outside_completed_window"),
                })
            } else {
                Err(StockSplitGuardError {
                    code: "stock_split_crosses_window",
                    detail: format!(
                        "该时段发生拆股，当前回测暂不支持自动处理；来源证据日期 {in_window:?}"
                    ),
                })
            }
        }
        StockSplitCoverage::Unverified { reason } => Err(StockSplitGuardError {
            code: "stock_split_coverage_unverified",
            detail: format!("当前行情缺少可核验的拆股覆盖，不能安全运行股票回测；{reason}"),
        }),
        StockSplitCoverage::NotApplicable => Err(StockSplitGuardError {
            code: "stock_split_coverage_unverified",
            detail: "股票来源错误地标记拆股核验为不适用，不能安全运行回测".into(),
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketSnapshot {
    pub bars: Vec<Bar>,
    pub provenance: MarketProvenance,
    pub stock_split_coverage: StockSplitCoverage,
}

impl AsRef<[Bar]> for MarketSnapshot {
    fn as_ref(&self) -> &[Bar] {
        &self.bars
    }
}

impl MarketSnapshot {
    fn new(bars: Vec<Bar>, provider: &str, endpoint: &str, price_basis: &str) -> Self {
        Self {
            bars,
            provenance: MarketProvenance {
                provider: provider.into(),
                endpoint: endpoint.into(),
                price_basis: price_basis.into(),
                corporate_actions: "not_simulated".into(),
            },
            stock_split_coverage: StockSplitCoverage::Unverified {
                reason: "provider did not supply split-event coverage".into(),
            },
        }
    }

    fn with_stock_split_coverage(mut self, coverage: StockSplitCoverage) -> Self {
        self.stock_split_coverage = coverage;
        self
    }
}

pub async fn fetch_public_market_bars(
    feed: &HttpFeed,
    source: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>> {
    Ok(
        fetch_public_market_snapshot(feed, source, symbol, since, limit)
            .await?
            .bars,
    )
}

/// Preserve the actual chosen provider and its price convention with its bars.
/// Quote OHLC must not be advertised as a dividend-adjusted total-return series.
pub async fn fetch_public_market_snapshot(
    feed: &HttpFeed,
    source: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<MarketSnapshot> {
    anyhow::ensure!(
        is_public_market_source(source),
        "unknown public market source"
    );
    match source {
        "binance" => feed.fetch_historical_snapshot(symbol, since, limit).await,
        "a_share" => fetch_a_share_snapshot(symbol, since, limit).await,
        "us_stock" => fetch_us_stock_snapshot(feed, symbol, since, limit).await,
        _ => unreachable!("source is validated above"),
    }
}

#[cfg(test)]
mod depth_snapshot_tests {
    use super::parse_binance_depth_snapshot;
    use serde_json::json;

    #[test]
    fn accepts_strictly_sorted_uncrossed_depth_and_rejects_invalid_books() {
        let valid = json!({
            "lastUpdateId":7,
            "bids":[["100","1"],["99","0"]],
            "asks":[["101","2"],["102","0"]]
        });
        let book = parse_binance_depth_snapshot(&valid).unwrap();
        assert_eq!(book.update_id, 7);
        assert_eq!(book.bids[0].price, 100.0);
        for invalid in [
            json!({"bids":[["100","1"]],"asks":[["101","1"]]}),
            json!({"lastUpdateId":7,"bids":[],"asks":[["101","1"]]}),
            json!({"lastUpdateId":7,"bids":[["99","1"],["100","1"]],"asks":[["101","1"]]}),
            json!({"lastUpdateId":7,"bids":[["100","1"]],"asks":[["100","1"]]}),
            json!({"lastUpdateId":7,"bids":[["100","1"]],"asks":[["101","-1"]]}),
            json!({"lastUpdateId":7,"bids":[["NaN","1"]],"asks":[["101","1"]]}),
        ] {
            assert!(parse_binance_depth_snapshot(&invalid).is_err(), "{invalid}");
        }
    }
}

#[cfg(test)]
mod deployment_endpoint_tests {
    use super::{valid_us_chart_symbol, validated_yahoo_relay_url, HttpFeed};

    #[test]
    fn default_http_feed_uses_the_public_binance_data_endpoint() {
        let feed = HttpFeed::new("target/test-market-cache");
        assert_eq!(feed.base_url, "https://data-api.binance.vision");
        assert_eq!(
            feed.us_stock_adjustment_url,
            "https://query1.finance.yahoo.com/v8/finance/chart"
        );
        assert_eq!(
            feed.us_stock_adjustment_fallback_url,
            "https://query2.finance.yahoo.com/v8/finance/chart"
        );
        assert_eq!(
            validated_yahoo_relay_url("https://relay.example/api/provider/yahoo-chart").as_deref(),
            Some("https://relay.example/api/provider/yahoo-chart")
        );
        assert_eq!(
            validated_yahoo_relay_url("http://127.0.0.1:18083/api/provider/yahoo-chart").as_deref(),
            Some("http://127.0.0.1:18083/api/provider/yahoo-chart")
        );
        assert!(validated_yahoo_relay_url("").is_none());
        assert!(validated_yahoo_relay_url("http://relay.example/yahoo").is_none());
        assert!(validated_yahoo_relay_url("https://user@relay.example/yahoo").is_none());
        assert!(valid_us_chart_symbol("AAPL"));
        assert!(valid_us_chart_symbol("BRK.B"));
        assert!(valid_us_chart_symbol("BRK-B"));
        assert!(valid_us_chart_symbol("A12345678901234567890123456789"));
        for invalid in [
            "",
            "1AAPL",
            "aapl",
            ".AAPL",
            "AAPL.",
            "BRK..B",
            "AAPL/USD",
            "A123456789012345678901234567890",
        ] {
            assert!(!valid_us_chart_symbol(invalid), "{invalid}");
        }
    }
}

#[cfg(test)]
mod public_market_source_tests {
    use super::*;
    use axum::{extract::Query, routing::get, Json, Router};
    use std::collections::HashMap;

    async fn stale_eastmoney_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "data": {"klines": ["2020-01-02,10,11,12,9,100"]}
        }))
    }

    async fn current_eastmoney_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "data": {"klines": [format!(
                "{},20,21,22,19,200",
                Utc::now().date_naive()
            )]}
        }))
    }

    async fn current_tencent_fixture(
        Query(query): Query<HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        let market_symbol = query
            .get("param")
            .and_then(|param| param.split(',').next())
            .unwrap_or("sz000001");
        Json(serde_json::json!({
            "data": {market_symbol: {"day": [[
                Utc::now().date_naive().to_string(), "10", "11", "12", "9", "100"
            ]]}}
        }))
    }

    async fn unavailable_tencent_fixture() -> axum::http::StatusCode {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    }

    async fn configured_crypto_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!([[
            1_704_153_600_000_i64,
            "10",
            "12",
            "9",
            "11",
            "100",
            1_704_157_199_999_i64,
            "1100",
            10,
            "30",
            "330",
            "0"
        ]]))
    }

    async fn empty_yahoo_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "chart": {"result": [{
                "timestamp": [],
                "indicators": {"quote": [{
                    "open": [], "high": [], "low": [], "close": [], "volume": []
                }]}
            }]}
        }))
    }

    async fn unavailable_yahoo_fixture() -> axum::http::StatusCode {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    }

    async fn aapl_split_yahoo_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "chart": {"result": [{
                "meta": {"symbol": "AAPL", "currency": "USD", "instrumentType": "EQUITY"},
                "timestamp": [1598621400_i64, 1598880600_i64],
                "events": {"splits": {"1598880600": {
                    "date": 1598880600_i64,
                    "numerator": 4.0,
                    "denominator": 1.0,
                    "splitRatio": "4:1"
                }}},
                "indicators": {
                    "quote": [{
                        "open": [126.01, 127.58],
                        "high": [126.44, 131.0],
                        "low": [124.58, 126.0],
                        "close": [124.81, 129.04],
                        "volume": [187630000.0, 225702700.0]
                    }],
                    "adjclose": [{"adjclose": [121.28, 125.39]}]
                }
            }], "error": null}
        }))
    }

    async fn yahoo_relay_fixture(
        Query(query): Query<HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        let symbol = query.get("symbol").unwrap();
        let mut source = reqwest::Url::parse(&format!(
            "https://query2.finance.yahoo.com/v8/finance/chart/{symbol}"
        ))
        .unwrap();
        source
            .query_pairs_mut()
            .append_pair("period1", query.get("period1").unwrap())
            .append_pair("period2", query.get("period2").unwrap())
            .append_pair("interval", "1d")
            .append_pair("includePrePost", "false")
            .append_pair("events", "div,splits");
        Json(serde_json::json!({
            "provider": "yahoo",
            "retrieval": "restricted_server_relay",
            "source_url": source.to_string(),
            "payload": aapl_split_yahoo_fixture().await.0
        }))
    }

    async fn invalid_yahoo_relay_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "provider": "yahoo",
            "retrieval": "restricted_server_relay",
            "source_url": "https://query1.finance.yahoo.com/v8/finance/chart/WRONG?period1=1&period2=2&interval=1d&includePrePost=false&events=div%2Csplits",
            "payload": aapl_split_yahoo_fixture().await.0
        }))
    }

    async fn nasdaq_fallback_fixture() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "data": {"tradesTable": {"rows": [{
                "date": "01/02/2024",
                "open": "$10.00",
                "high": "$11.00",
                "low": "$9.00",
                "close": "$10.50",
                "volume": "900"
            }]}}
        }))
    }

    async fn nasdaq_etf_fixture(
        axum::extract::Query(query): axum::extract::Query<
            std::collections::HashMap<String, String>,
        >,
    ) -> Json<serde_json::Value> {
        if query.get("assetclass").is_some_and(|value| value == "etf") {
            nasdaq_fallback_fixture().await
        } else {
            Json(serde_json::json!({"data": null}))
        }
    }

    async fn provider_fixture_server() -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/east", get(stale_eastmoney_fixture))
                    .route("/east-current", get(current_eastmoney_fixture))
                    .route("/tencent", get(current_tencent_fixture))
                    .route("/tencent-unavailable", get(unavailable_tencent_fixture))
                    .route("/yahoo/*symbol", get(empty_yahoo_fixture))
                    .route("/yahoo-unavailable/*symbol", get(unavailable_yahoo_fixture))
                    .route("/yahoo-split/*symbol", get(aapl_split_yahoo_fixture))
                    .route("/yahoo-relay", get(yahoo_relay_fixture))
                    .route("/yahoo-relay-invalid", get(invalid_yahoo_relay_fixture))
                    .route("/nasdaq/*symbol", get(nasdaq_fallback_fixture))
                    .route("/api/v3/klines", get(configured_crypto_fixture))
                    .route("/nasdaq-etf/*symbol", get(nasdaq_etf_fixture)),
            )
            .await
            .unwrap();
        });
        (format!("http://{address}"), server)
    }

    #[test]
    fn eastmoney_daily_row_maps_its_documented_ohlcv_columns() {
        let bar = parse_eastmoney_kline("2024-01-02,10.0,11.0,12.0,9.5,12345,0,0,0,0,0").unwrap();
        assert_eq!(bar.timestamp.to_rfc3339(), "2024-01-02T00:00:00+00:00");
        assert_eq!(
            (bar.open, bar.high, bar.low, bar.close, bar.volume),
            (10.0, 12.0, 9.5, 11.0, 12345.0)
        );
    }

    #[test]
    fn stale_primary_daily_series_is_not_accepted_as_current_market_data() {
        let now = Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap();
        let stale = vec![Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 14, 0, 0, 0).unwrap(),
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.5,
            volume: 100.0,
        }];
        let current = vec![Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 22, 0, 0, 0).unwrap(),
            ..stale[0]
        }];
        let two_sessions_old = vec![Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 21, 0, 0, 0).unwrap(),
            ..stale[0]
        }];
        assert!(!has_current_daily_bars(&[], now));
        assert!(!has_current_daily_bars(&stale, now));
        assert!(has_current_daily_bars(&current, now));
        assert!(!has_current_daily_bars(&two_sessions_old, now));

        let friday = vec![Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 18, 0, 0, 0).unwrap(),
            ..stale[0]
        }];
        assert!(has_current_daily_bars(
            &friday,
            Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap()
        ));
        assert!(has_current_daily_bars(
            &friday,
            Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap()
        ));
        assert!(has_current_daily_bars(
            &friday,
            Utc.with_ymd_and_hms(2026, 9, 21, 12, 0, 0).unwrap()
        ));
        assert!(!has_current_daily_bars(
            &friday,
            Utc.with_ymd_and_hms(2026, 9, 22, 12, 0, 0).unwrap()
        ));
    }

    #[test]
    fn daily_selection_prefers_freshest_and_preserves_primary_on_fallback_failure() {
        let now = Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap();
        let primary = Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 22, 0, 0, 0).unwrap(),
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.5,
            volume: 100.0,
        };
        let newer = Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap(),
            close: 12.0,
            ..primary
        };
        assert_eq!(
            select_a_share_daily_bars(Ok(vec![primary]), Ok(vec![newer]), now).unwrap()[0].close,
            12.0
        );
        assert_eq!(
            select_a_share_daily_bars(
                Ok(vec![primary]),
                Err(anyhow::anyhow!("fixture unavailable")),
                now
            )
            .unwrap()[0]
                .close,
            10.5
        );
        let holiday_now = Utc.with_ymd_and_hms(2026, 10, 5, 12, 0, 0).unwrap();
        let pre_closure = Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).unwrap(),
            ..primary
        };
        let older = Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 9, 29, 0, 0, 0).unwrap(),
            close: 9.5,
            ..primary
        };
        let selected =
            select_a_share_daily_bars(Ok(vec![older]), Ok(vec![pre_closure]), holiday_now).unwrap();
        assert_eq!(
            selected.last().unwrap().timestamp,
            Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).unwrap()
        );
        let stale = Bar {
            timestamp: Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap(),
            ..primary
        };
        assert!(select_a_share_daily_bars(
            Ok(vec![stale]),
            Err(anyhow::anyhow!("unavailable")),
            holiday_now
        )
        .unwrap_err()
        .to_string()
        .contains("requires verification"));
    }

    #[test]
    fn tencent_daily_rows_map_ohlcv_columns() {
        let raw = serde_json::json!({"data":{"sh600519":{"day":[["2024-01-02","10","11","12","9","100"]],"qfqday":[["2024-01-02","1","1","1","1","1"]]}}});
        let bars = parse_tencent_a_share_bars(&raw, "sh600519").unwrap();
        assert_eq!(
            (
                bars[0].open,
                bars[0].high,
                bars[0].low,
                bars[0].close,
                bars[0].volume
            ),
            (10.0, 12.0, 9.0, 11.0, 100.0)
        );
    }

    #[test]
    fn unadjusted_daily_requests_never_accept_forward_adjusted_fallback_rows() {
        let adjusted_only = serde_json::json!({"data":{"sh600519":{"qfqday":[
            ["2024-01-02","10","11","12","9","100"]
        ]}}});
        let error = parse_tencent_a_share_bars(&adjusted_only, "sh600519")
            .expect_err("a forward-adjusted series cannot replace unadjusted daily prices");
        assert!(error.to_string().contains("unadjusted"));
    }

    #[tokio::test]
    async fn cached_crypto_candles_do_not_claim_a_new_verified_exchange_response() {
        let directory =
            std::env::temp_dir().join(format!("axiom-provenance-{}", uuid::Uuid::new_v4()));
        let feed = HttpFeed::new(&directory);
        let bar = Bar {
            timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.0,
            volume: 100.0,
        };
        feed.csv.save("BTCUSDT", "1h", &[bar]).unwrap();
        let snapshot = fetch_public_market_snapshot(&feed, "binance", "BTCUSDT", bar.timestamp, 1)
            .await
            .unwrap();
        assert_eq!(snapshot.bars[0].close, 11.0);
        assert_eq!(snapshot.provenance.provider, "local_csv_cache");
        assert_eq!(
            snapshot.provenance.price_basis,
            "cache_price_basis_unverified"
        );
        assert_eq!(snapshot.provenance.endpoint, "local historical CSV cache");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn aapl_adjustment_uses_the_successful_fallback_request_url() {
        let (base, server) = provider_fixture_server().await;
        let directory =
            std::env::temp_dir().join(format!("axiom-yahoo-fallback-{}", uuid::Uuid::new_v4()));
        let mut feed = HttpFeed::new(&directory);
        feed.us_stock_adjustment_url = format!("{base}/yahoo-unavailable");
        feed.us_stock_adjustment_fallback_url = format!("{base}/yahoo-split");
        let evidence = feed.fetch_aapl_split_adjustment_evidence().await.unwrap();
        let selected = reqwest::Url::parse(&evidence.endpoint).unwrap();
        assert_eq!(selected.path(), "/yahoo-split/AAPL");
        assert!(selected
            .query_pairs()
            .any(|(key, value)| key == "events" && value == "div,splits"));
        assert_eq!(evidence.provider, "configured_endpoint");
        assert_eq!(evidence.event.split_ratio, "4:1");
        server.abort();
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn aapl_adjustment_accepts_only_a_matching_restricted_relay_source() {
        let (base, server) = provider_fixture_server().await;
        let directory =
            std::env::temp_dir().join(format!("axiom-yahoo-relay-{}", uuid::Uuid::new_v4()));
        let mut feed = HttpFeed::new(&directory);
        feed.us_stock_adjustment_url = format!("{base}/yahoo-unavailable");
        feed.us_stock_adjustment_fallback_url = format!("{base}/yahoo-unavailable");
        feed.us_stock_relay_url = Some(format!("{base}/yahoo-relay"));
        let evidence = feed.fetch_aapl_split_adjustment_evidence().await.unwrap();
        assert_eq!(evidence.provider, "yahoo_via_restricted_relay");
        assert_eq!(evidence.retrieval, "restricted_server_relay");
        let source = reqwest::Url::parse(&evidence.endpoint).unwrap();
        assert_eq!(source.host_str(), Some("query2.finance.yahoo.com"));
        assert_eq!(source.path(), "/v8/finance/chart/AAPL");

        feed.us_stock_relay_url = Some(format!("{base}/yahoo-relay-invalid"));
        assert!(feed.fetch_aapl_split_adjustment_evidence().await.is_err());
        server.abort();
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn aapl_adjustment_fails_closed_when_both_yahoo_endpoints_fail() {
        let (base, server) = provider_fixture_server().await;
        let directory =
            std::env::temp_dir().join(format!("axiom-yahoo-closed-{}", uuid::Uuid::new_v4()));
        let mut feed = HttpFeed::new(&directory);
        feed.us_stock_adjustment_url = format!("{base}/yahoo-unavailable");
        feed.us_stock_adjustment_fallback_url = format!("{base}/yahoo-unavailable");
        feed.us_stock_relay_url = None;
        let error = feed
            .fetch_aapl_split_adjustment_evidence()
            .await
            .expect_err("two failed provider requests must not fabricate split evidence");
        assert!(error.to_string().contains("HTTP"));
        server.abort();
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn configured_crypto_endpoints_are_not_misidentified_as_the_official_exchange() {
        let (base, server) = provider_fixture_server().await;
        let directory =
            std::env::temp_dir().join(format!("axiom-configured-feed-{}", uuid::Uuid::new_v4()));
        let mut feed = HttpFeed::new(&directory);
        feed.base_url = base.clone();
        let snapshot = fetch_public_market_snapshot(
            &feed,
            "binance",
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
            1,
        )
        .await
        .unwrap();
        assert_eq!(snapshot.provenance.provider, "configured_crypto_endpoint");
        assert_eq!(
            snapshot.provenance.endpoint,
            format!("{base}/api/v3/klines")
        );
        assert_eq!(snapshot.bars[0].close, 11.0);
        server.abort();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn daily_series_keeps_the_most_recent_requested_rows() {
        let raw = serde_json::json!({"data":{"sh600519":{"day":[
            ["2024-01-02","10","11","12","9","100"],
            ["2024-01-03","11","12","13","10","110"],
            ["2024-01-04","12","13","14","11","120"]
        ]}}});
        let bars = parse_tencent_a_share_bars(&raw, "sh600519").unwrap();
        let since = Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap();
        let latest = latest_daily_bars(bars, since, 1);
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].timestamp.date_naive().to_string(), "2024-01-04");
        assert_eq!(
            latest_daily_bars(latest.clone(), since, 5)[0].close,
            latest[0].close
        );
    }

    #[test]
    fn yahoo_parser_discards_current_and_incomplete_sessions() {
        let raw = serde_json::json!({"chart":{"result":[{"timestamp":[1704067200, 4102444800i64],"indicators":{"quote":[{"open":[10.0,null],"high":[12.0,null],"low":[9.0,null],"close":[11.0,null],"volume":[100.0,null]}]}}]}});
        let bars = parse_yahoo_bars(&raw).unwrap();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, 11.0);
    }

    #[test]
    fn yahoo_parser_uses_market_close_and_dst_offset_for_daily_completion() {
        let session_start = Utc.with_ymd_and_hms(2026, 9, 22, 13, 30, 0).unwrap();
        let session_end = Utc.with_ymd_and_hms(2026, 9, 22, 20, 0, 0).unwrap();
        let raw = serde_json::json!({
            "chart": {"result": [{
                "meta": {
                    "gmtoffset": -14_400,
                    "currentTradingPeriod": {"regular": {
                        "start": session_start.timestamp(), "end": session_end.timestamp()
                    }}
                },
                "timestamp": [
                    Utc.with_ymd_and_hms(2026, 9, 21, 4, 0, 0).unwrap().timestamp(),
                    Utc.with_ymd_and_hms(2026, 9, 22, 4, 0, 0).unwrap().timestamp()
                ],
                "indicators": {"quote": [{
                    "open": [10.0, 11.0], "high": [11.0, 12.0],
                    "low": [9.0, 10.0], "close": [10.5, 11.5],
                    "volume": [100.0, 110.0]
                }]}
            }]}
        });
        let before_close =
            parse_yahoo_bars_at(&raw, Utc.with_ymd_and_hms(2026, 9, 22, 19, 59, 59).unwrap())
                .unwrap();
        assert_eq!(before_close.len(), 1);
        assert_eq!(before_close[0].close, 10.5);
        let after_close =
            parse_yahoo_bars_at(&raw, Utc.with_ymd_and_hms(2026, 9, 22, 20, 0, 1).unwrap())
                .unwrap();
        assert_eq!(
            after_close[1].timestamp.to_rfc3339(),
            "2026-09-22T00:00:00+00:00"
        );
        assert_eq!(after_close.len(), 2);

        let weekend =
            parse_yahoo_bars_at(&raw, Utc.with_ymd_and_hms(2026, 9, 26, 12, 0, 0).unwrap())
                .unwrap();
        assert_eq!(weekend.len(), 2);
    }
    #[test]
    fn nasdaq_rows_parse_display_numbers_and_reverse_chronology() {
        let raw = serde_json::json!({"data":{"tradesTable":{"rows":[
            {"date":"01/03/2024","open":"$11.00","high":"$12.00","low":"$10.00","close":"$11.50","volume":"1,000"},
            {"date":"01/02/2024","open":"$10.00","high":"$11.00","low":"$9.00","close":"$10.50","volume":"900"}
        ]}}});
        let bars = parse_nasdaq_bars(&raw).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].timestamp.to_rfc3339(), "2024-01-02T00:00:00+00:00");
        assert_eq!((bars[1].close, bars[1].volume), (11.5, 1000.0));
    }

    #[test]
    fn provider_parsers_skip_malformed_rows_and_report_missing_shapes() {
        assert!(parse_binance_kline(&serde_json::json!([0, "1", "2"])).is_none());
        assert!(parse_binance_kline(&serde_json::json!([0, "bad", "2", "1", "2", "3"])).is_none());
        assert!(parse_eastmoney_kline("2024-01-02,not-a-number,11,12,9,100").is_none());
        assert!(parse_eastmoney_kline("not-a-date,10,11,12,9,100").is_none());
        assert!(parse_tencent_a_share_bars(&serde_json::json!({}), "sh600519").is_err());
        assert!(parse_nasdaq_bars(&serde_json::json!({})).is_err());
        assert!(parse_yahoo_bars(&serde_json::json!({})).is_err());
        let malformed_yahoo = serde_json::json!({
            "chart": {"result": [{
                "timestamp": ["not-an-integer", 9223372036854775807i64],
                "indicators": {"quote": [{
                    "open": [10.0, 10.0], "high": [11.0, 11.0],
                    "low": [9.0, 9.0], "close": [10.5, 10.5],
                    "volume": [100.0, 100.0]
                }]}
            }]}
        });
        assert!(parse_yahoo_bars(&malformed_yahoo).unwrap().is_empty());
    }

    #[test]
    fn csv_feed_filters_since_and_streams_the_recent_completed_bar() {
        let dir = format!("target/test-csv-live-{}", uuid::Uuid::new_v4());
        let feed = CsvFeed::new(&dir);
        let now = Utc::now();
        let bars = vec![
            Bar {
                timestamp: now - Duration::hours(2),
                open: 10.0,
                high: 11.0,
                low: 9.0,
                close: 10.5,
                volume: 100.0,
            },
            Bar {
                timestamp: now - Duration::minutes(30),
                open: 10.5,
                high: 11.5,
                low: 10.0,
                close: 11.0,
                volume: 110.0,
            },
        ];
        feed.save("BTC/USDT", "1h", &bars).unwrap();
        let fetched = feed
            .fetch_historical("BTC/USDT", now - Duration::hours(1), 5)
            .unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(feed.stream_live("BTC/USDT").unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn public_sources_have_curated_symbols_and_no_synthetic_source() {
        assert_eq!(PUBLIC_MARKET_SOURCES, &["binance", "a_share", "us_stock"]);
        assert_eq!(
            source_symbols("a_share"),
            vec!["600519", "000001", "300750", "601318"]
        );
        assert_eq!(
            source_symbols("us_stock"),
            vec!["AAPL", "MSFT", "NVDA", "SPY"]
        );
        assert!(is_public_market_source("binance"));
        assert!(!is_public_market_source("synthetic"));
    }

    #[tokio::test]
    async fn local_provider_fixtures_cover_stale_a_share_and_us_fallbacks() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let a_share_snapshot = fetch_a_share_snapshot_at(
            &client,
            &format!("{base}/east"),
            &format!("{base}/tencent"),
            "000001",
            since,
            5,
        )
        .await
        .unwrap();
        assert_eq!(a_share_snapshot.provenance.provider, "tencent");
        assert_eq!(
            a_share_snapshot.provenance.endpoint,
            format!("{base}/tencent")
        );
        assert_eq!(
            a_share_snapshot.provenance.price_basis,
            "unadjusted_requested"
        );
        assert_eq!(
            a_share_snapshot.provenance.corporate_actions,
            "not_simulated"
        );
        let a_share = a_share_snapshot.bars;
        assert_eq!(a_share.len(), 1);
        assert_eq!(a_share[0].close, 11.0);
        assert_eq!(a_share[0].timestamp.date_naive(), Utc::now().date_naive());

        let us_stock_snapshot = fetch_us_stock_snapshot_at(
            &client,
            YahooStockSources {
                primary: &format!("{base}/yahoo"),
                fallback: &format!("{base}/yahoo"),
                relay: None,
            },
            &format!("{base}/nasdaq"),
            "AAPL",
            since,
            5,
        )
        .await
        .unwrap();
        assert_eq!(us_stock_snapshot.provenance.provider, "nasdaq");
        assert_eq!(
            us_stock_snapshot.provenance.price_basis,
            "provider_adjustment_unverified"
        );
        let selected = reqwest::Url::parse(&us_stock_snapshot.provenance.endpoint).unwrap();
        assert_eq!(selected.path(), "/nasdaq/AAPL/historical");
        assert!(selected
            .query_pairs()
            .any(|(key, value)| key == "assetclass" && value == "stocks"));
        assert!(matches!(
            us_stock_snapshot.stock_split_coverage,
            StockSplitCoverage::Unverified { .. }
        ));
        assert!(guard_stock_backtest_window(
            "us_stock",
            &us_stock_snapshot.stock_split_coverage,
            &us_stock_snapshot.bars
        )
        .is_err());
        let us_stock = us_stock_snapshot.bars;
        assert_eq!(us_stock.len(), 1);
        assert_eq!(us_stock[0].close, 10.5);
        server.abort();
    }

    #[tokio::test]
    async fn public_us_stock_snapshot_uses_the_feed_transport_configuration() {
        let (base, server) = provider_fixture_server().await;
        let directory =
            std::env::temp_dir().join(format!("axiom-yahoo-public-{}", uuid::Uuid::new_v4()));
        let mut feed = HttpFeed::new(&directory);
        feed.us_stock_adjustment_url = format!("{base}/yahoo-unavailable");
        feed.us_stock_adjustment_fallback_url = format!("{base}/yahoo-split");
        feed.us_stock_relay_url = None;
        let since = Utc.with_ymd_and_hms(2020, 8, 27, 0, 0, 0).unwrap();
        let snapshot = fetch_public_market_snapshot(&feed, "us_stock", "AAPL", since, 1)
            .await
            .unwrap();
        assert_eq!(snapshot.bars.len(), 1);
        assert_eq!(snapshot.provenance.provider, "configured_yahoo_endpoint");
        assert!(matches!(
            snapshot.stock_split_coverage,
            StockSplitCoverage::CrossesWindow { .. }
        ));
        server.abort();
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn us_stock_daily_request_floors_provider_window_but_filters_at_exact_since() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 8, 28, 17, 0, 0).unwrap();
        let snapshot = fetch_us_stock_snapshot_at(
            &client,
            YahooStockSources {
                primary: &format!("{base}/yahoo-unavailable"),
                fallback: &format!("{base}/yahoo-split"),
                relay: None,
            },
            &format!("{base}/nasdaq"),
            "AAPL",
            since,
            5,
        )
        .await
        .unwrap();
        let selected = reqwest::Url::parse(&snapshot.provenance.endpoint).unwrap();
        let query: std::collections::HashMap<_, _> = selected.query_pairs().collect();
        assert_eq!(
            query.get("period1").map(|value| value.as_ref()),
            Some("1598572800")
        );
        assert_eq!(snapshot.bars.len(), 1);
        assert_eq!(
            snapshot.bars[0].timestamp,
            Utc.with_ymd_and_hms(2020, 8, 31, 0, 0, 0).unwrap()
        );
        server.abort();
    }

    #[tokio::test]
    async fn us_stock_uses_secondary_yahoo_with_its_actual_request_url() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 8, 27, 0, 0, 0).unwrap();
        let snapshot = fetch_us_stock_snapshot_at(
            &client,
            YahooStockSources {
                primary: &format!("{base}/yahoo-unavailable"),
                fallback: &format!("{base}/yahoo-split"),
                relay: None,
            },
            &format!("{base}/nasdaq"),
            "AAPL",
            since,
            5,
        )
        .await
        .unwrap();
        let selected = reqwest::Url::parse(&snapshot.provenance.endpoint).unwrap();
        assert_eq!(selected.path(), "/yahoo-split/AAPL");
        assert!(selected
            .query_pairs()
            .any(|(key, value)| key == "events" && value == "div,splits"));
        assert_eq!(snapshot.provenance.provider, "configured_yahoo_endpoint");
        assert!(matches!(
            snapshot.stock_split_coverage,
            StockSplitCoverage::CrossesWindow { .. }
        ));
        server.abort();
    }

    #[tokio::test]
    async fn us_stock_relay_preserves_original_yahoo_source_and_split_coverage() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 8, 27, 0, 0, 0).unwrap();
        let snapshot = fetch_us_stock_snapshot_at(
            &client,
            YahooStockSources {
                primary: &format!("{base}/yahoo-unavailable"),
                fallback: &format!("{base}/yahoo-unavailable"),
                relay: Some(&format!("{base}/yahoo-relay")),
            },
            &format!("{base}/nasdaq"),
            "AAPL",
            since,
            1,
        )
        .await
        .unwrap();
        assert_eq!(snapshot.provenance.provider, "yahoo_via_restricted_relay");
        assert_eq!(snapshot.bars.len(), 1);
        let source = reqwest::Url::parse(&snapshot.provenance.endpoint).unwrap();
        assert_eq!(source.host_str(), Some("query2.finance.yahoo.com"));
        assert_eq!(source.path(), "/v8/finance/chart/AAPL");
        assert!(matches!(
            snapshot.stock_split_coverage,
            StockSplitCoverage::CrossesWindow { .. }
        ));
        server.abort();
    }

    #[tokio::test]
    async fn invalid_relay_source_falls_back_to_unverified_nasdaq_rows() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 8, 27, 0, 0, 0).unwrap();
        let snapshot = fetch_us_stock_snapshot_at(
            &client,
            YahooStockSources {
                primary: &format!("{base}/yahoo-unavailable"),
                fallback: &format!("{base}/yahoo-unavailable"),
                relay: Some(&format!("{base}/yahoo-relay-invalid")),
            },
            &format!("{base}/nasdaq"),
            "AAPL",
            since,
            5,
        )
        .await
        .unwrap();
        assert_eq!(snapshot.provenance.provider, "nasdaq");
        assert!(matches!(
            snapshot.stock_split_coverage,
            StockSplitCoverage::Unverified { .. }
        ));
        server.abort();
    }

    #[tokio::test]
    async fn nasdaq_fallback_retries_etf_asset_class_for_listed_funds() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let snapshot = fetch_us_stock_snapshot_at(
            &client,
            YahooStockSources {
                primary: &format!("{base}/yahoo"),
                fallback: &format!("{base}/yahoo"),
                relay: None,
            },
            &format!("{base}/nasdaq-etf"),
            "SPY",
            since,
            5,
        )
        .await
        .unwrap();
        let selected = reqwest::Url::parse(&snapshot.provenance.endpoint).unwrap();
        assert_eq!(selected.path(), "/nasdaq-etf/SPY/historical");
        assert!(selected
            .query_pairs()
            .any(|(key, value)| key == "assetclass" && value == "etf"));
        let bars = snapshot.bars;
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, 10.5);
        server.abort();
    }

    #[tokio::test]
    async fn current_primary_survives_unavailable_tencent_fallback() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let bars = fetch_a_share_at(
            &client,
            &format!("{base}/east-current"),
            &format!("{base}/tencent-unavailable"),
            "000001",
            since,
            5,
        )
        .await
        .unwrap();
        assert_eq!(bars[0].close, 21.0);
        server.abort();
    }

    #[tokio::test]
    async fn tencent_exchange_prefixes_are_selected_before_provider_parsing() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let sh = fetch_a_share_tencent_at(
            &client,
            &format!("{base}/tencent"),
            "600519",
            since,
            1,
            None,
        )
        .await
        .unwrap();
        let bj = fetch_a_share_tencent_at(
            &client,
            &format!("{base}/tencent"),
            "920001",
            since,
            1,
            None,
        )
        .await
        .unwrap();
        assert_eq!(sh[0].close, 11.0);
        assert_eq!(bj[0].close, 11.0);
        server.abort();
    }

    fn split_test_bar(day: u32, low: f64, high: f64) -> Bar {
        Bar {
            timestamp: Utc.with_ymd_and_hms(2024, 1, day, 0, 0, 0).unwrap(),
            open: low + 1.0,
            high,
            low,
            close: high - 1.0,
            volume: 1_000.0,
        }
    }

    #[test]
    fn yahoo_split_events_guard_only_the_returned_bar_window() {
        let bars = vec![
            split_test_bar(2, 99.0, 103.0),
            split_test_bar(3, 100.0, 104.0),
        ];
        let no_events = serde_json::json!({"chart":{"result":[{}]}});
        assert!(matches!(
            yahoo_stock_split_coverage(&no_events, &bars),
            StockSplitCoverage::VerifiedNoSplitInWindow { .. }
        ));

        let inside = Utc
            .with_ymd_and_hms(2024, 1, 3, 13, 30, 0)
            .unwrap()
            .timestamp();
        let split = serde_json::json!({"chart":{"result":[{"events":{"splits":{"event":{
            "date":inside,"numerator":2.0,"denominator":1.0,"splitRatio":"2:1"
        }}}}]}});
        let coverage = yahoo_stock_split_coverage(&split, &bars);
        assert!(matches!(coverage, StockSplitCoverage::CrossesWindow { .. }));
        assert_eq!(
            guard_stock_backtest_window("us_stock", &coverage, &bars)
                .unwrap_err()
                .code,
            "stock_split_crosses_window"
        );

        let outside = Utc
            .with_ymd_and_hms(2023, 12, 1, 13, 30, 0)
            .unwrap()
            .timestamp();
        let split = serde_json::json!({"chart":{"result":[{"events":{"splits":{"event":{
            "date":outside,"numerator":2.0,"denominator":1.0,"splitRatio":"2:1"
        }}}}]}});
        assert!(matches!(
            yahoo_stock_split_coverage(&split, &bars),
            StockSplitCoverage::VerifiedNoSplitInWindow { .. }
        ));
    }

    #[test]
    fn malformed_yahoo_split_event_fails_closed() {
        let bars = vec![split_test_bar(2, 99.0, 103.0)];
        let malformed = serde_json::json!({"chart":{"result":[{"events":{"splits":{"event":{
            "date":"not-a-timestamp","numerator":2.0,"denominator":1.0
        }}}}]}});
        let coverage = yahoo_stock_split_coverage(&malformed, &bars);
        assert!(matches!(coverage, StockSplitCoverage::Unverified { .. }));
        assert_eq!(
            guard_stock_backtest_window("us_stock", &coverage, &bars)
                .unwrap_err()
                .code,
            "stock_split_coverage_unverified"
        );
    }

    #[test]
    fn a_share_raw_qfq_scale_distinguishes_cash_offsets_from_share_actions() {
        let raw = vec![
            split_test_bar(1, 98.0, 102.0),
            split_test_bar(2, 99.0, 103.0),
            split_test_bar(3, 101.0, 105.0),
            split_test_bar(4, 102.0, 106.0),
        ];
        let cash_adjusted: Vec<_> = raw
            .iter()
            .map(|bar| Bar {
                open: bar.open - 20.0,
                high: bar.high - 20.0,
                low: bar.low - 20.0,
                close: bar.close - 20.0,
                ..*bar
            })
            .collect();
        assert!(matches!(
            a_share_stock_split_coverage(&raw[1..], &raw, &cash_adjusted),
            StockSplitCoverage::VerifiedNoSplitInWindow { .. }
        ));

        let mut split_adjusted = cash_adjusted;
        for bar in &mut split_adjusted[1..] {
            bar.open *= 0.5;
            bar.high *= 0.5;
            bar.low *= 0.5;
            bar.close *= 0.5;
        }
        let first_bar_split = a_share_stock_split_coverage(&raw[1..], &raw, &split_adjusted);
        assert!(matches!(
            &first_bar_split,
            StockSplitCoverage::CrossesWindow { event_dates, .. }
                if event_dates == &[raw[1].timestamp.date_naive()]
        ));
        assert_eq!(
            guard_stock_backtest_window("a_share", &first_bar_split, &raw[1..])
                .unwrap_err()
                .code,
            "stock_split_crosses_window"
        );
        assert!(matches!(
            a_share_stock_split_coverage(&raw, &raw[..2], &split_adjusted[..2]),
            StockSplitCoverage::Unverified { .. }
        ));
        let mut divergent_selected = raw.clone();
        divergent_selected[1].close += 0.03;
        assert!(matches!(
            a_share_stock_split_coverage(&divergent_selected[1..], &raw, &raw),
            StockSplitCoverage::Unverified { .. }
        ));
        assert!(matches!(
            a_share_stock_split_coverage(&raw, &raw, &raw),
            StockSplitCoverage::Unverified { .. }
        ));
    }

    #[test]
    fn stock_guard_rejects_not_applicable_stock_coverage_but_ignores_crypto() {
        let bars = vec![split_test_bar(2, 99.0, 103.0)];
        assert!(
            guard_stock_backtest_window("binance", &StockSplitCoverage::NotApplicable, &bars)
                .is_ok()
        );
        assert!(
            guard_stock_backtest_window("a_share", &StockSplitCoverage::NotApplicable, &bars)
                .is_err()
        );
        assert!(guard_stock_backtest_window(
            "a_share",
            &StockSplitCoverage::VerifiedNoSplitInWindow {
                evidence: "fixture".into()
            },
            &bars
        )
        .is_ok());
        assert!(matches!(
            guard_stock_backtest_window(
                "us_stock",
                &StockSplitCoverage::CrossesWindow {
                    evidence: "fixture".into(),
                    event_dates: vec![NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()]
                },
                &bars
            )
            .unwrap(),
            StockSplitCoverage::VerifiedNoSplitInWindow { .. }
        ));
    }
}

#[cfg(test)]
mod recent_trade_parser_tests {
    use super::*;
    use serde_json::{json, Value};
    #[test]
    fn trade_parser_preserves_id_order_and_decimal_prices_and_rejects_unusable_evidence() {
        let cutoff = Utc.timestamp_millis_opt(1_700_000_010_000).unwrap();
        let valid = json!([{"id":1,"price":"00100.00","qty":"2.00","time":1_700_000_000_001_i64},{"id":2,"price":"100.0","qty":"3","time":1_700_000_000_001_i64}]);
        let parsed = parse_binance_recent_trades(&valid, cutoff).unwrap();
        assert_eq!(parsed[0].price_decimal, "100");
        assert_eq!(parsed[0].timestamp, parsed[1].timestamp);
        let mut fractional = valid.clone();
        fractional[0]["price"] = json!("000.0100");
        fractional[1]["price"] = json!("0.01");
        assert_eq!(
            parse_binance_recent_trades(&fractional, cutoff).unwrap()[0].price_decimal,
            "0.01"
        );
        for invalid in [
            json!({}),
            json!([]),
            json!([valid[0]]),
            json!(vec![valid[0].clone(); 1001]),
        ] {
            assert!(parse_binance_recent_trades(&invalid, cutoff).is_err());
        }
        for (key, value) in [
            ("price", Value::Null),
            ("price", json!("")),
            ("price", json!("NaN")),
            ("price", json!("1.2.3")),
            ("price", json!(".")),
            ("price", json!("0")),
            ("qty", json!("-1")),
            ("qty", json!("0")),
            ("qty", json!("9".repeat(400))),
            ("id", json!(-1)),
            ("id", json!(1)),
            ("id", json!(4)),
            ("time", json!("bad")),
            ("time", json!(i64::MAX)),
            ("time", json!(0)),
            ("time", json!(1_700_000_011_000_i64)),
            ("time", json!(1_700_000_000_000_i64)),
        ] {
            let mut invalid = valid.clone();
            invalid[1][key] = value;
            assert!(
                parse_binance_recent_trades(&invalid, cutoff).is_err(),
                "{invalid}"
            );
        }
        let mut ambiguous = valid;
        ambiguous[0]["price"] = json!("100000000000000000000");
        ambiguous[1]["price"] = json!("100000000000000000001");
        assert!(parse_binance_recent_trades(&ambiguous, cutoff).is_err());
    }
}

#[cfg(test)]
mod bitcoin_block_snapshot_tests {
    use super::*;
    use serde_json::json;

    fn hash(n: u8) -> String {
        format!("{n:064x}")
    }
    fn rows() -> serde_json::Value {
        json!((0..10).map(|i| {
            let height = 1000 - i;
            json!({"id":hash(height as u8),"height":height,"previousblockhash":hash((height-1) as u8),"timestamp":1700000000 + (i as i64 * 17),"size":1000+i,"tx_count":100+i})
        }).collect::<Vec<_>>())
    }
    #[test]
    fn bitcoin_snapshot_normalizes_provider_newest_first_to_height_ascending() {
        let blocks = parse_bitcoin_block_snapshot(&rows()).unwrap();
        assert_eq!(blocks.len(), 10);
        assert_eq!(blocks[0].height, 991);
        assert_eq!(blocks.last().unwrap().height, 1000);
        assert_eq!(blocks[1].previous_hash, blocks[0].hash);
    }
    #[test]
    fn bitcoin_snapshot_rejects_malformed_unlinked_and_nonpositive_rows() {
        let mut cases = Vec::new();
        cases.push(json!({"not":"array"}));
        let mut missing = rows();
        missing.as_array_mut().unwrap().pop();
        cases.push(missing);
        let mut height = rows();
        height[1]["height"] = json!(997);
        cases.push(height);
        let mut link = rows();
        link[0]["previousblockhash"] = json!(hash(42));
        cases.push(link);
        let mut zero = rows();
        zero[0]["size"] = json!(0);
        cases.push(zero);
        let mut zero_transactions = rows();
        zero_transactions[0]["tx_count"] = json!(0);
        cases.push(zero_transactions);
        let mut bad_hash = rows();
        bad_hash[0]["id"] = json!("bad");
        cases.push(bad_hash);
        for value in cases {
            assert!(parse_bitcoin_block_snapshot(&value).is_err());
        }
    }
}

#[cfg(test)]
mod bitcoin_transaction_parser_tests {
    use super::*;
    use serde_json::json;
    fn h(n: u8) -> String {
        format!("{n:064x}")
    }
    fn block() -> BitcoinBlock {
        BitcoinBlock {
            height: 9,
            hash: h(9),
            previous_hash: h(8),
            timestamp: Utc::now(),
            size_bytes: 1,
            tx_count: 1,
        }
    }
    fn rows() -> serde_json::Value {
        json!([{"txid":h(1),"fee":0,"size":100,"vin":[{"is_coinbase":true}],"status":{"confirmed":true,"block_hash":h(9),"block_height":9}},{"txid":h(2),"fee":7,"size":101,"vin":[{"prevout":{"value":14,"scriptpubkey_type":"p2wpkh"}}],"vout":[{"value":7,"scriptpubkey_type":"p2wpkh"},{"value":0,"scriptpubkey_type":"op_return"}],"status":{"confirmed":true,"block_hash":h(9),"block_height":9}}])
    }
    #[test]
    fn transaction_parser_validates_scope_membership_and_sizes() {
        let b = block();
        let (_, coinbase, txs) = parse_bitcoin_transactions(&rows(), &b).unwrap();
        assert_eq!(coinbase, 1);
        assert_eq!(txs[0].fee_sats, 7);
        assert_eq!(txs[0].spent_prevout_values_sats, vec![14]);
        assert_eq!(txs[0].outputs.len(), 2);
        for f in [
            |v: &mut serde_json::Value| v[1]["status"]["confirmed"] = json!(false),
            |v: &mut serde_json::Value| v[1]["status"]["block_hash"] = json!(h(3)),
            |v: &mut serde_json::Value| v[1]["size"] = json!(0),
            |v: &mut serde_json::Value| v[1]["txid"] = json!(h(1)),
            |v: &mut serde_json::Value| v[1]["vin"][0]["prevout"]["value"] = json!("bad"),
            |v: &mut serde_json::Value| v[1]["vout"][0]["scriptpubkey_type"] = json!(""),
            |v: &mut serde_json::Value| v[1]["fee"] = json!(8),
        ] {
            let mut v = rows();
            f(&mut v);
            assert!(parse_bitcoin_transactions(&v, &b).is_err());
        }
    }

    #[test]
    fn transaction_parser_accepts_missing_addresses_and_rejects_malformed_labels() {
        let b = block();
        let (_, _, parsed) = parse_bitcoin_transactions(&rows(), &b).unwrap();
        assert_eq!(parsed[0].spent_prevout_addresses, vec![None]);
        assert_eq!(parsed[0].outputs[0].scriptpubkey_address, None);
        for address in [json!(""), json!("地址"), json!("a".repeat(129))] {
            let mut input = rows();
            input[1]["vin"][0]["prevout"]["scriptpubkey_address"] = address.clone();
            assert!(parse_bitcoin_transactions(&input, &b).is_err());
            let mut output = rows();
            output[1]["vout"][0]["scriptpubkey_address"] = address;
            assert!(parse_bitcoin_transactions(&output, &b).is_err());
        }
    }
}
