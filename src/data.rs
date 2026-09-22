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
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Timelike, Utc};
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
    Some(complete_daily_bar(
        fields.first()?.trim(),
        fields.get(1)?.trim().parse().ok()?,
        fields.get(3)?.trim().parse().ok()?,
        fields.get(4)?.trim().parse().ok()?,
        fields.get(2)?.trim().parse().ok()?,
        fields.get(5)?.trim().parse().ok()?,
    )?)
}

fn parse_yahoo_bars(raw: &serde_json::Value) -> Result<Vec<Bar>> {
    let result = raw
        .pointer("/chart/result/0")
        .context("Yahoo response has no result")?;
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
    for i in 0..times.len() {
        let Some(ts) = times[i].as_i64() else {
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
        if timestamp.date_naive() >= Utc::now().date_naive() {
            continue;
        }
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

fn json_number(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn parse_tencent_a_share_bars(raw: &serde_json::Value, market_symbol: &str) -> Result<Vec<Bar>> {
    let rows = raw
        .pointer(&format!("/data/{market_symbol}/qfqday"))
        .and_then(serde_json::Value::as_array)
        .context("Tencent A-share response has no adjusted daily rows")?;
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

async fn fetch_a_share_tencent(
    symbol: &str,
    since: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>> {
    let exchange = if symbol.starts_with(['6', '9']) {
        "sh"
    } else {
        "sz"
    };
    let market_symbol = format!("{exchange}{symbol}");
    let raw: serde_json::Value = reqwest::Client::new()
        .get("https://web.ifzq.gtimg.cn/appstock/app/fqkline/get")
        .header(
            reqwest::header::USER_AGENT,
            "AXIOM educational market reader/1.0",
        )
        .query(&[("param", format!("{market_symbol},day,,,{limit},qfq"))])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("Tencent A-share market request failed")?
        .error_for_status()
        .context("Tencent A-share market returned HTTP error")?
        .json()
        .await
        .context("invalid Tencent A-share market JSON")?;
    let mut bars = parse_tencent_a_share_bars(&raw, &market_symbol)?;
    bars.retain(|bar| bar.timestamp >= since);
    if bars.len() > limit {
        bars = bars.split_off(bars.len() - limit);
    }
    Ok(bars)
}

async fn fetch_a_share(symbol: &str, since: DateTime<Utc>, limit: usize) -> Result<Vec<Bar>> {
    anyhow::ensure!(
        symbol.len() == 6 && symbol.bytes().all(|byte| byte.is_ascii_digit()),
        "A-share symbol must be a six digit code"
    );
    let exchange = if symbol.starts_with(['6', '9']) {
        "1"
    } else {
        "0"
    };
    let request = reqwest::Client::new()
        .get("https://push2his.eastmoney.com/api/qt/stock/kline/get")
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
    let primary = async {
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
        Ok::<Vec<Bar>, anyhow::Error>(
            bars.into_iter()
                .filter(|bar| bar.timestamp >= since)
                .take(limit)
                .collect(),
        )
    }
    .await;
    match primary {
        Ok(bars) if !bars.is_empty() => Ok(bars),
        Ok(_) | Err(_) => fetch_a_share_tencent(symbol, since, limit).await,
    }
}

async fn fetch_us_stock(symbol: &str, since: DateTime<Utc>, limit: usize) -> Result<Vec<Bar>> {
    anyhow::ensure!(
        !symbol.is_empty()
            && symbol.len() <= 12
            && symbol
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte == b'.' || byte == b'-'),
        "US symbol must contain uppercase letters, digits, dot or dash"
    );
    let raw: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}"
        ))
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
    Ok(bars)
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
    fn tencent_adjusted_rows_map_ohlcv_columns() {
        let raw = serde_json::json!({"data":{"sh600519":{"qfqday":[["2024-01-02","10","11","12","9","100"]]}}});
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
    fn yahoo_parser_discards_current_and_incomplete_sessions() {
        let raw = serde_json::json!({"chart":{"result":[{"timestamp":[1704067200, 4102444800i64],"indicators":{"quote":[{"open":[10.0,null],"high":[12.0,null],"low":[9.0,null],"close":[11.0,null],"volume":[100.0,null]}]}}]}});
        let bars = parse_yahoo_bars(&raw).unwrap();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, 11.0);
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
}
