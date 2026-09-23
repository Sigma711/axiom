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
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

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

pub struct HttpFeed {
    pub base_url: String,
    pub csv: CsvFeed,
    client: reqwest::Client,
}

impl HttpFeed {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_url: "https://data-api.binance.vision".to_string(),
            csv: CsvFeed::new(cache_dir),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("无法创建 HTTP client"),
        }
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

#[async_trait]
impl AsyncDataFeed for HttpFeed {
    async fn fetch_historical_async(
        &self,
        symbol: &str,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Bar>> {
        // 1. 先看本地缓存
        if let Ok(cached) = self.csv.load(symbol, "1h") {
            let cached_after: Vec<Bar> = cached
                .iter()
                .filter(|b| b.timestamp >= since && b.timestamp + Duration::hours(1) <= Utc::now())
                .cloned()
                .collect();
            if cached_after.len() >= limit {
                return Ok(cached_after.into_iter().take(limit).collect());
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

        Ok(bars)
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

fn parse_yahoo_bars(raw: &serde_json::Value) -> Result<Vec<Bar>> {
    parse_yahoo_bars_at(raw, Utc::now())
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
    let rows = raw
        .pointer(&format!("/data/{market_symbol}/day"))
        .or_else(|| raw.pointer(&format!("/data/{market_symbol}/qfqday")))
        .and_then(serde_json::Value::as_array)
        .context("Tencent A-share response has no completed daily rows")?;
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
) -> Result<Vec<Bar>> {
    let exchange = match symbol.as_bytes().first() {
        Some(b'6') | Some(b'9') if !symbol.starts_with("92") => "sh",
        Some(b'4') | Some(b'8') | Some(b'9') => "bj",
        _ => "sz",
    };
    let market_symbol = format!("{exchange}{symbol}");
    let raw: serde_json::Value = client
        .get(endpoint)
        .header(
            reqwest::header::USER_AGENT,
            "AXIOM educational market reader/1.0",
        )
        .query(&[("param", format!("{market_symbol},day,,,{limit},"))])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("Tencent A-share market request failed")?
        .error_for_status()
        .context("Tencent A-share market returned HTTP error")?
        .json()
        .await
        .context("invalid Tencent A-share market JSON")?;
    Ok(latest_daily_bars(
        parse_tencent_a_share_bars(&raw, &market_symbol)?,
        since,
        limit,
    ))
}

fn latest_daily_bars(mut bars: Vec<Bar>, since: DateTime<Utc>, limit: usize) -> Vec<Bar> {
    bars.retain(|bar| bar.timestamp >= since);
    if bars.len() > limit {
        bars = bars.split_off(bars.len() - limit);
    }
    bars
}

async fn fetch_a_share(symbol: &str, since: DateTime<Utc>, limit: usize) -> Result<Vec<Bar>> {
    let client = reqwest::Client::new();
    fetch_a_share_at(
        &client,
        "https://push2his.eastmoney.com/api/qt/stock/kline/get",
        "https://web.ifzq.gtimg.cn/appstock/app/fqkline/get",
        symbol,
        since,
        limit,
    )
    .await
}

async fn fetch_a_share_at(
    client: &reqwest::Client,
    eastmoney_endpoint: &str,
    tencent_endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>> {
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
    let (primary, tencent) = tokio::join!(
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
            Ok::<Vec<Bar>, anyhow::Error>(latest_daily_bars(bars, since, limit))
        },
        fetch_a_share_tencent_at(client, tencent_endpoint, symbol, since, limit)
    );
    select_a_share_daily_bars(primary, tencent, Utc::now())
}

fn select_a_share_daily_bars(
    primary: Result<Vec<Bar>>,
    fallback: Result<Vec<Bar>>,
    now: DateTime<Utc>,
) -> Result<Vec<Bar>> {
    let primary_current = primary
        .as_ref()
        .is_ok_and(|bars| has_current_daily_bars(bars, now));
    let fallback_current = fallback
        .as_ref()
        .is_ok_and(|bars| has_current_daily_bars(bars, now));
    match (primary_current, fallback_current) {
        (true, true) => {
            let primary = primary.expect("current primary result was checked above");
            let fallback = fallback.expect("current fallback result was checked above");
            if fallback.last().map(|bar| bar.timestamp) > primary.last().map(|bar| bar.timestamp) {
                Ok(fallback)
            } else {
                Ok(primary)
            }
        }
        (true, false) => primary,
        (false, true) => fallback,
        (false, false) => {
            let primary_error = primary
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "primary daily series is stale".into());
            let fallback_error = fallback
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "Tencent daily series is stale".into());
            anyhow::bail!(
                "no current A-share daily series; primary: {primary_error}; Tencent: {fallback_error}"
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
) -> Result<Vec<Bar>> {
    // Nasdaq separates common shares and exchange-traded funds by asset class.
    // The catalog includes both, so a missing stock result must retry as an ETF.
    let mut last_error = String::new();
    for assetclass in ["stocks", "etf"] {
        let attempt: Result<Vec<Bar>> = async {
            let raw: serde_json::Value = client
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
                .context("Nasdaq market returned HTTP error")?
                .json()
                .await
                .context("invalid Nasdaq market JSON")?;
            Ok(latest_daily_bars(parse_nasdaq_bars(&raw)?, since, limit))
        }
        .await;
        match attempt {
            Ok(bars) if !bars.is_empty() => return Ok(bars),
            Ok(_) => last_error = format!("{assetclass} returned no daily rows"),
            Err(error) => last_error = format!("{assetclass}: {error}"),
        }
    }
    anyhow::bail!("Nasdaq market has no usable daily rows for {symbol}: {last_error}")
}

async fn fetch_us_stock(symbol: &str, since: DateTime<Utc>, limit: usize) -> Result<Vec<Bar>> {
    let client = reqwest::Client::new();
    fetch_us_stock_at(
        &client,
        "https://query1.finance.yahoo.com/v8/finance/chart",
        "https://api.nasdaq.com/api/quote",
        symbol,
        since,
        limit,
    )
    .await
}

async fn fetch_us_stock_at(
    client: &reqwest::Client,
    yahoo_endpoint: &str,
    nasdaq_endpoint: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>> {
    anyhow::ensure!(
        !symbol.is_empty()
            && symbol.len() <= 12
            && symbol.bytes().all(|byte| byte.is_ascii_uppercase()
                || byte.is_ascii_digit()
                || byte == b'.'
                || byte == b'-'),
        "US symbol must contain uppercase letters, digits, dot or dash"
    );
    let primary = async {
        let raw: serde_json::Value = client
            .get(format!("{yahoo_endpoint}/{}", symbol.replace('.', "-")))
            .header(
                reqwest::header::USER_AGENT,
                "AXIOM educational market reader/1.0",
            )
            .query(&[
                ("period1", since.timestamp().to_string()),
                ("period2", Utc::now().timestamp().to_string()),
                ("interval", "1d".to_owned()),
                ("includePrePost", "false".to_owned()),
                ("events", "div,splits".to_owned()),
            ])
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .context("US market request failed")?
            .error_for_status()
            .context("US market returned HTTP error")?
            .json()
            .await
            .context("invalid US market JSON")?;
        let mut bars = parse_yahoo_bars(&raw)?;
        bars.retain(|bar| bar.timestamp >= since);
        if bars.len() > limit {
            bars = bars.split_off(bars.len() - limit);
        }
        Ok::<Vec<Bar>, anyhow::Error>(bars)
    }
    .await;
    match primary {
        Ok(bars) if !bars.is_empty() => Ok(bars),
        Ok(_) | Err(_) => {
            fetch_us_stock_nasdaq_at(client, nasdaq_endpoint, symbol, since, limit).await
        }
    }
}

/// Fetch completed candles from a named free public source. Source-specific
/// response peculiarities, daily session rules and parsers stay behind this seam.
pub async fn fetch_public_market_bars(
    feed: &HttpFeed,
    source: &str,
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>> {
    anyhow::ensure!(
        is_public_market_source(source),
        "unknown public market source"
    );
    match source {
        "binance" => feed.fetch_historical_async(symbol, since, limit).await,
        "a_share" => fetch_a_share(symbol, since, limit).await,
        "us_stock" => fetch_us_stock(symbol, since, limit).await,
        _ => unreachable!("source is validated above"),
    }
}

#[cfg(test)]
mod deployment_endpoint_tests {
    use super::HttpFeed;

    #[test]
    fn default_http_feed_uses_the_public_binance_data_endpoint() {
        assert_eq!(
            HttpFeed::new("target/test-market-cache").base_url,
            "https://data-api.binance.vision"
        );
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
                    .route("/nasdaq/*symbol", get(nasdaq_fallback_fixture))
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
        assert!(select_a_share_daily_bars(
            Ok(vec![Bar {
                timestamp: Utc.with_ymd_and_hms(2026, 9, 14, 0, 0, 0).unwrap(),
                ..primary
            }]),
            Err(anyhow::anyhow!("fixture unavailable")),
            now
        )
        .is_err());
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
        let a_share = fetch_a_share_at(
            &client,
            &format!("{base}/east"),
            &format!("{base}/tencent"),
            "000001",
            since,
            5,
        )
        .await
        .unwrap();
        assert_eq!(a_share.len(), 1);
        assert_eq!(a_share[0].close, 11.0);
        assert_eq!(a_share[0].timestamp.date_naive(), Utc::now().date_naive());

        let us_stock = fetch_us_stock_at(
            &client,
            &format!("{base}/yahoo"),
            &format!("{base}/nasdaq"),
            "AAPL",
            since,
            5,
        )
        .await
        .unwrap();
        assert_eq!(us_stock.len(), 1);
        assert_eq!(us_stock[0].close, 10.5);
        server.abort();
    }

    #[tokio::test]
    async fn nasdaq_fallback_retries_etf_asset_class_for_listed_funds() {
        let (base, server) = provider_fixture_server().await;
        let client = reqwest::Client::new();
        let since = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let bars = fetch_us_stock_at(
            &client,
            &format!("{base}/yahoo"),
            &format!("{base}/nasdaq-etf"),
            "SPY",
            since,
            5,
        )
        .await
        .unwrap();
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
        let sh = fetch_a_share_tencent_at(&client, &format!("{base}/tencent"), "600519", since, 1)
            .await
            .unwrap();
        let bj = fetch_a_share_tencent_at(&client, &format!("{base}/tencent"), "920001", since, 1)
            .await
            .unwrap();
        assert_eq!(sh[0].close, 11.0);
        assert_eq!(bj[0].close, 11.0);
        server.abort();
    }
}
