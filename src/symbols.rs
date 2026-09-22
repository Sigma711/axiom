//! 可检索的公开证券目录。
//!
//! 这个模块把供应商目录的分页、去重、缓存和搜索放在一个深模块后面；
//! 调用者只给出市场、关键词与页码，不需要知道供应商的文件格式或市场代码。

use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};

const PAGE_SIZE: usize = 100;
const MAX_SEARCH_LIMIT: usize = 100;
const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24);
const SNAPSHOT_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SymbolItem {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
}

#[derive(Clone, Debug)]
struct CatalogSnapshot {
    items: Vec<SymbolItem>,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedCatalog {
    version: u8,
    fetched_at_unix: i64,
    items: Vec<SymbolItem>,
    #[serde(default)]
    complete: Option<bool>,
}

static CATALOGS: OnceLock<RwLock<HashMap<String, CatalogSnapshot>>> = OnceLock::new();
static REFRESHING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn catalogs() -> &'static RwLock<HashMap<String, CatalogSnapshot>> {
    CATALOGS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn refreshing() -> &'static Mutex<HashSet<String>> {
    REFRESHING.get_or_init(|| Mutex::new(HashSet::new()))
}

fn symbol_data_dir() -> PathBuf {
    std::env::var_os("AXIOM_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"))
        .join("symbols")
}

fn snapshot_path(data_dir: &Path, source: &str) -> PathBuf {
    data_dir.join(format!("{source}.json"))
}

fn minimum_snapshot_items(source: &str) -> usize {
    match source {
        // A complete Eastmoney catalog currently contains several thousand
        // rows; this floor prevents a seed or partial response being persisted.
        "a_share" => 1_000,
        "us_stock" => 5_000,
        _ => usize::MAX,
    }
}

fn complete_snapshot(source: &str, items: &[SymbolItem], complete: Option<bool>) -> bool {
    complete != Some(false) && items.len() >= minimum_snapshot_items(source)
}

fn load_persisted_catalog_from(data_dir: &Path, source: &str) -> Option<CatalogSnapshot> {
    let raw = std::fs::read(snapshot_path(data_dir, source)).ok()?;
    let persisted: PersistedCatalog = serde_json::from_slice(&raw).ok()?;
    if persisted.version != SNAPSHOT_VERSION
        || !complete_snapshot(source, &persisted.items, persisted.complete)
        || persisted.fetched_at_unix < 0
    {
        return None;
    }
    let fetched_at = UNIX_EPOCH + Duration::from_secs(persisted.fetched_at_unix as u64);
    let age = SystemTime::now()
        .duration_since(fetched_at)
        .unwrap_or_default();
    Some(CatalogSnapshot {
        items: persisted.items,
        fetched_at: Instant::now().checked_sub(age).unwrap_or_else(Instant::now),
    })
}

fn load_persisted_catalog(source: &str) -> Option<CatalogSnapshot> {
    load_persisted_catalog_from(&symbol_data_dir(), source)
}

fn persist_catalog_snapshot_at(
    data_dir: &Path,
    source: &str,
    items: &[SymbolItem],
    fetched_at: SystemTime,
) -> Result<()> {
    anyhow::ensure!(
        complete_snapshot(source, items, Some(true)),
        "refusing to persist incomplete symbol directory"
    );
    let fetched_at_unix = fetched_at
        .duration_since(UNIX_EPOCH)
        .context("symbol snapshot timestamp is before Unix epoch")?
        .as_secs() as i64;
    let persisted = PersistedCatalog {
        version: SNAPSHOT_VERSION,
        fetched_at_unix,
        items: items.to_vec(),
        complete: Some(true),
    };
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("creating symbol snapshot directory {data_dir:?}"))?;
    let path = snapshot_path(data_dir, source);
    let temp = data_dir.join(format!(".{source}.{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut file = std::fs::File::create(&temp)
            .with_context(|| format!("creating symbol snapshot temp file {temp:?}"))?;
        let bytes = serde_json::to_vec_pretty(&persisted)?;
        std::io::Write::write_all(&mut file, &bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, &path)
            .with_context(|| format!("installing symbol snapshot {path:?}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn persist_catalog_snapshot(source: &str, items: &[SymbolItem]) -> Result<()> {
    persist_catalog_snapshot_at(&symbol_data_dir(), source, items, SystemTime::now())
}

fn seed(source: &str) -> Vec<SymbolItem> {
    let rows: &[(&str, &str, &str)] = match source {
        "a_share" => &[
            ("600519", "贵州茅台", "SSE"),
            ("000001", "平安银行", "SZSE"),
            ("300750", "宁德时代", "SZSE"),
            ("601318", "中国平安", "SSE"),
        ],
        "us_stock" => &[
            ("AAPL", "Apple Inc. Common Stock", "NASDAQ"),
            ("MSFT", "Microsoft Corporation Common Stock", "NASDAQ"),
            ("NVDA", "NVIDIA Corporation Common Stock", "NASDAQ"),
            ("SPY", "SPDR S&P 500 ETF Trust", "NYSE Arca"),
        ],
        _ => &[],
    };
    rows.iter()
        .map(|(symbol, name, exchange)| SymbolItem {
            symbol: (*symbol).into(),
            name: (*name).into(),
            exchange: (*exchange).into(),
        })
        .collect()
}

fn normalize_query(query: &str) -> String {
    query.trim().to_uppercase()
}

fn sort_and_filter(mut items: Vec<SymbolItem>, query: &str) -> Vec<SymbolItem> {
    let query = normalize_query(query);
    if !query.is_empty() {
        items.retain(|item| {
            item.symbol.to_uppercase().contains(&query) || item.name.to_uppercase().contains(&query)
        });
    }
    items.sort_by(|a, b| {
        let rank = |item: &SymbolItem| {
            if query.is_empty() {
                2
            } else if item.symbol.eq_ignore_ascii_case(&query) {
                0
            } else if item.symbol.to_uppercase().starts_with(&query) {
                1
            } else {
                2
            }
        };
        rank(a).cmp(&rank(b)).then_with(|| a.symbol.cmp(&b.symbol))
    });
    items
}

fn limited_page(items: Vec<SymbolItem>, offset: usize, limit: usize) -> Vec<SymbolItem> {
    items.into_iter().skip(offset).take(limit).collect()
}

fn eastmoney_exchange(code: i64, symbol: &str) -> &'static str {
    match code {
        1 => "SSE",
        0 if symbol.starts_with(['4', '8', '9']) => "BSE",
        _ => "SZSE",
    }
}

fn parse_eastmoney_page(raw: &serde_json::Value) -> Result<(usize, Vec<SymbolItem>)> {
    let data = raw
        .get("data")
        .context("Eastmoney directory data missing")?;
    let total = data["total"]
        .as_u64()
        .context("Eastmoney directory total missing")? as usize;
    let rows = data["diff"]
        .as_array()
        .context("Eastmoney directory rows missing")?;
    let items = rows
        .iter()
        .filter_map(|row| {
            let symbol = row["f12"].as_str()?.trim();
            let name = row["f14"].as_str()?.trim();
            (!symbol.is_empty() && !name.is_empty()).then(|| SymbolItem {
                symbol: symbol.into(),
                name: name.into(),
                exchange: eastmoney_exchange(row["f13"].as_i64().unwrap_or(0), symbol).into(),
            })
        })
        .collect();
    Ok((total, items))
}

async fn eastmoney_page_at(
    client: &reqwest::Client,
    endpoint: &str,
    page: usize,
) -> Result<(usize, Vec<SymbolItem>)> {
    let raw: serde_json::Value = client
        .get(endpoint)
        .query(&[
            ("pn", page.to_string()),
            ("pz", PAGE_SIZE.to_string()),
            ("po", "1".into()),
            ("np", "1".into()),
            ("fltt", "2".into()),
            ("invt", "2".into()),
            ("fid", "f3".into()),
            ("ut", "bd1d9ddb04089700cf9c27f6f7426281".into()),
            (
                "fs",
                "m:0+t:6,m:0+t:80,m:1+t:2,m:1+t:23,m:0+t:81+s:2048".into(),
            ),
            ("fields", "f12,f13,f14".into()),
        ])
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (compatible; AXIOM/1.0)",
        )
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("A-share directory request failed")?
        .error_for_status()
        .context("A-share directory returned HTTP error")?
        .json()
        .await
        .context("invalid A-share directory JSON")?;
    parse_eastmoney_page(&raw)
}

async fn fetch_a_share_catalog_at(
    client: &reqwest::Client,
    endpoint: &str,
) -> Result<Vec<SymbolItem>> {
    let (total, first) = eastmoney_page_at(client, endpoint, 1).await?;
    anyhow::ensure!(
        total >= first.len() && !first.is_empty(),
        "A-share directory is incomplete"
    );
    let pages = total.div_ceil(PAGE_SIZE);
    let rest = stream::iter(2..=pages)
        .map(|page| eastmoney_page_at(client, endpoint, page))
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;
    let mut items = first;
    for page in rest {
        let (page_total, mut page_items) = page?;
        anyhow::ensure!(
            page_total == total,
            "A-share directory changed during pagination"
        );
        items.append(&mut page_items);
    }
    dedupe_catalog(items, total)
}

async fn fetch_a_share_github_catalog_at(
    client: &reqwest::Client,
    endpoint: &str,
) -> Result<Vec<SymbolItem>> {
    let raw: serde_json::Value = client
        .get(endpoint)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (compatible; AXIOM/1.0)",
        )
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("A-share mirror directory request failed")?
        .error_for_status()
        .context("A-share mirror directory returned HTTP error")?
        .json()
        .await
        .context("invalid A-share mirror directory JSON")?;
    let items = raw
        .as_array()
        .context("A-share mirror directory is not an array")?
        .iter()
        .filter_map(|row| {
            let symbol = row["code"].as_str()?.trim();
            let name = row["name"].as_str()?.trim();
            let stock_type = row["stock_type"].as_str().unwrap_or_default();
            (!symbol.is_empty() && !name.is_empty()).then(|| SymbolItem {
                symbol: symbol.into(),
                name: name.into(),
                exchange: match stock_type {
                    "sh_a" | "sh_b" | "kcb" => "SSE",
                    "hs_bjs" => "BSE",
                    _ => "SZSE",
                }
                .into(),
            })
        })
        .collect();
    dedupe_catalog(items, 5000)
}

async fn fetch_resilient_a_share_catalog_at(
    client: &reqwest::Client,
    primary_endpoint: &str,
    fallback_endpoint: &str,
) -> Result<Vec<SymbolItem>> {
    match fetch_a_share_catalog_at(client, primary_endpoint).await {
        Ok(items) => Ok(items),
        Err(primary) => fetch_a_share_github_catalog_at(client, fallback_endpoint)
            .await
            .with_context(|| format!("Eastmoney directory failed: {primary}")),
    }
}

fn parse_pipe_directory(raw: &str, source: &str) -> Vec<SymbolItem> {
    let mut lines = raw.lines();
    let header: Vec<_> = lines.next().unwrap_or_default().split('|').collect();
    let position = |name: &str| header.iter().position(|value| *value == name);
    let symbol_col = position(if source == "nasdaq" {
        "Symbol"
    } else {
        "ACT Symbol"
    });
    let name_col = position("Security Name");
    let exchange_col = position("Exchange");
    let test_col = position("Test Issue");
    lines
        .filter_map(|line| {
            let row: Vec<_> = line.split('|').collect();
            let symbol = row.get(symbol_col?)?.trim();
            let name = row.get(name_col?)?.trim();
            if symbol.is_empty()
                || name.is_empty()
                || symbol == "File Creation Time"
                || test_col
                    .and_then(|i| row.get(i))
                    .is_some_and(|value| value.trim() == "Y")
            {
                return None;
            }
            let exchange = exchange_col
                .and_then(|index| row.get(index))
                .map(|value| match value.trim() {
                    "N" => "NYSE",
                    "A" => "NYSE American",
                    "P" => "NYSE Arca",
                    "Z" => "BATS",
                    other => other,
                })
                .unwrap_or("NASDAQ");
            Some(SymbolItem {
                symbol: symbol.into(),
                name: name.into(),
                exchange: exchange.into(),
            })
        })
        .collect()
}

async fn fetch_us_stock_catalog_at(
    client: &reqwest::Client,
    nasdaq_endpoint: &str,
    other_endpoint: &str,
) -> Result<Vec<SymbolItem>> {
    let urls = [("nasdaq", nasdaq_endpoint), ("other", other_endpoint)];
    let mut items = Vec::new();
    for (source, url) in urls {
        let text = client
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (compatible; AXIOM/1.0)",
            )
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .context("US directory request failed")?
            .error_for_status()
            .context("US directory returned HTTP error")?
            .text()
            .await
            .context("invalid US directory response")?;
        let rows = parse_pipe_directory(&text, source);
        anyhow::ensure!(!rows.is_empty(), "US directory response had no symbols");
        items.extend(rows);
    }
    let minimum_expected = 5000;
    dedupe_catalog(items, minimum_expected)
}

fn dedupe_catalog(mut items: Vec<SymbolItem>, minimum_expected: usize) -> Result<Vec<SymbolItem>> {
    items.sort_by(|a, b| {
        a.symbol
            .cmp(&b.symbol)
            .then_with(|| a.exchange.cmp(&b.exchange))
    });
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(item.symbol.clone()));
    anyhow::ensure!(
        items.len() >= minimum_expected,
        "symbol directory was incomplete"
    );
    Ok(items)
}

async fn live_catalog(source: &str) -> Result<Vec<SymbolItem>> {
    match source {
        "a_share" => {
            let client = reqwest::Client::new();
            fetch_resilient_a_share_catalog_at(
                &client,
                "https://82.push2.eastmoney.com/api/qt/clist/get",
                "https://raw.githubusercontent.com/guidebee/china-stock-data/main/data/company/companies.json",
            )
            .await
        }
        "us_stock" => {
            let client = reqwest::Client::new();
            fetch_us_stock_catalog_at(
                &client,
                "https://www.nasdaqtrader.com/dynamic/SymDir/nasdaqlisted.txt",
                "https://www.nasdaqtrader.com/dynamic/SymDir/otherlisted.txt",
            )
            .await
        }
        _ => anyhow::bail!("unsupported symbol catalog source"),
    }
}

async fn refresh_catalog_in_background(source: &str) {
    let mut active = refreshing().lock().await;
    if !active.insert(source.to_owned()) {
        return;
    }
    let source = source.to_owned();
    let load_source = source.clone();
    spawn_catalog_refresh(source, async move { live_catalog(&load_source).await });
}

fn spawn_catalog_refresh<F>(source: String, load: F) -> tokio::task::JoinHandle<()>
where
    F: Future<Output = Result<Vec<SymbolItem>>> + Send + 'static,
{
    tokio::spawn(async move {
        match load.await {
            Ok(items) => {
                if let Err(error) = persist_catalog_snapshot(&source, &items) {
                    tracing::warn!(source, %error, "symbol directory snapshot write failed");
                }
                catalogs().write().await.insert(
                    source.clone(),
                    CatalogSnapshot {
                        items,
                        fetched_at: Instant::now(),
                    },
                );
            }
            Err(error) => tracing::warn!(source, %error, "symbol directory refresh failed"),
        }
        refreshing().lock().await.remove(&source);
    })
}

/// Search a complete current market directory. A successful result is cached for
/// one day. If a refresh fails, the last complete snapshot stays usable.
pub async fn search(
    source: &str,
    query: &str,
    offset: usize,
    limit: usize,
) -> Result<(Vec<SymbolItem>, usize, usize, &'static str, bool)> {
    anyhow::ensure!(
        matches!(source, "a_share" | "us_stock"),
        "unsupported symbol catalog source"
    );
    anyhow::ensure!(query.chars().count() <= 64, "symbol query is too long");
    let limit = limit.clamp(1, MAX_SEARCH_LIMIT);

    if catalogs().read().await.get(source).is_none() {
        if let Some(snapshot) = load_persisted_catalog(source) {
            catalogs().write().await.insert(source.to_owned(), snapshot);
        }
    }

    if let Some(snapshot) = catalogs().read().await.get(source).cloned() {
        let universe_count = snapshot.items.len();
        let filtered = sort_and_filter(snapshot.items, query);
        let total = filtered.len();
        let fresh = snapshot.fetched_at.elapsed() <= CACHE_TTL;
        if !fresh {
            refresh_catalog_in_background(source).await;
        }
        return Ok((
            limited_page(filtered, offset, limit),
            total,
            universe_count,
            if fresh { "cached" } else { "stale" },
            true,
        ));
    }

    // A complete public directory can take seconds to download. Return the
    // useful seed now, and replace it in the same process as soon as the
    // single background refresh completes.
    refresh_catalog_in_background(source).await;
    let items = sort_and_filter(seed(source), query);
    let total = items.len();
    Ok((
        limited_page(items, offset, limit),
        total,
        seed(source).len(),
        "refreshing",
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Query, http::StatusCode, routing::get, Json, Router};
    use serde_json::Value;
    use std::collections::HashMap;

    async fn eastmoney_fixture(Query(query): Query<HashMap<String, String>>) -> Json<Value> {
        let page = query
            .get("pn")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1);
        let rows: Vec<Value> = if page == 1 {
            (0..100)
                .map(|index| {
                    serde_json::json!({
                        "f12": format!("600{index:03}"),
                        "f13": 1,
                        "f14": format!("沪市 {index}")
                    })
                })
                .collect()
        } else {
            vec![serde_json::json!({
                "f12": "430047",
                "f13": 0,
                "f14": "北交所样本"
            })]
        };
        Json(serde_json::json!({"data": {"total": 101, "diff": rows}}))
    }

    async fn mirror_fixture() -> Json<Value> {
        Json(Value::Array(
            (0..5000)
                .map(|index| {
                    serde_json::json!({
                        "code": format!("{index:06}"),
                        "name": format!("镜像样本 {index}"),
                        "stock_type": if index == 0 { "hs_bjs" } else { "sh_a" }
                    })
                })
                .collect(),
        ))
    }

    async fn unavailable_fixture() -> StatusCode {
        StatusCode::SERVICE_UNAVAILABLE
    }

    async fn nasdaq_fixture() -> String {
        let mut body = String::from("Symbol|Security Name|Market Category|Test Issue\n");
        for index in 0..4999 {
            body.push_str(&format!("N{index:04}|Nasdaq {index}|Q|N\n"));
        }
        body
    }

    async fn other_fixture() -> String {
        "ACT Symbol|Security Name|Exchange|Test Issue\nSPY|SPDR|P|N\n".into()
    }

    async fn fixture_server() -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/east", get(eastmoney_fixture))
                    .route("/mirror", get(mirror_fixture))
                    .route("/unavailable", get(unavailable_fixture))
                    .route("/nasdaq", get(nasdaq_fixture))
                    .route("/other", get(other_fixture)),
            )
            .await
            .unwrap();
        });
        (format!("http://{address}"), server)
    }

    #[test]
    fn search_orders_exact_then_prefix_then_name_and_pages() {
        let items = vec![
            SymbolItem {
                symbol: "600001".into(),
                name: "甲".into(),
                exchange: "SSE".into(),
            },
            SymbolItem {
                symbol: "600519".into(),
                name: "贵州茅台".into(),
                exchange: "SSE".into(),
            },
            SymbolItem {
                symbol: "000519".into(),
                name: "贵州测试".into(),
                exchange: "SZSE".into(),
            },
        ];
        let found = sort_and_filter(items, "600519");
        assert_eq!(
            found
                .into_iter()
                .map(|item| item.symbol)
                .collect::<Vec<_>>(),
            ["600519"]
        );
        let ranked = sort_and_filter(
            vec![
                SymbolItem {
                    symbol: "ABC".into(),
                    name: "exact".into(),
                    exchange: "X".into(),
                },
                SymbolItem {
                    symbol: "ABCD".into(),
                    name: "prefix".into(),
                    exchange: "X".into(),
                },
                SymbolItem {
                    symbol: "Z".into(),
                    name: "abc name".into(),
                    exchange: "X".into(),
                },
            ],
            "abc",
        );
        assert_eq!(
            ranked
                .into_iter()
                .map(|item| item.symbol)
                .collect::<Vec<_>>(),
            ["ABC", "ABCD", "Z"]
        );
        let page = limited_page(
            (0..200)
                .map(|n| SymbolItem {
                    symbol: format!("{n:06}"),
                    name: String::new(),
                    exchange: String::new(),
                })
                .collect(),
            100,
            50,
        );
        assert_eq!(page.len(), 50);
        assert_eq!(page[0].symbol, "000100");
    }

    #[test]
    fn parses_nasdaq_and_other_listed_rows_without_test_or_trailer() {
        let nasdaq = "Symbol|Security Name|Market Category|Test Issue\nAAPL|Apple Inc.|Q|N\nTEST|No|Q|Y\nFile Creation Time|x|x|N\n";
        let other = "ACT Symbol|Security Name|Exchange|CQS Symbol|ETF|Round Lot Size|Test Issue|NASDAQ Symbol\nBRK.B|Berkshire|N|BRK.B|N|100|N|BRK.B\n";
        assert_eq!(parse_pipe_directory(nasdaq, "nasdaq").len(), 1);
        let rows = parse_pipe_directory(other, "other");
        assert_eq!(rows[0].symbol, "BRK.B");
        assert_eq!(rows[0].exchange, "NYSE");
    }

    #[test]
    fn parses_a_share_market_and_keeps_beijing_exchange() {
        let raw = serde_json::json!({"data":{"total":2,"diff":[{"f12":"600519","f13":1,"f14":"贵州茅台"},{"f12":"430047","f13":0,"f14":"诺思兰德"}]}});
        let (_, rows) = parse_eastmoney_page(&raw).unwrap();
        assert_eq!(rows[0].exchange, "SSE");
        assert_eq!(rows[1].exchange, "BSE");
    }

    #[test]
    fn directory_parsers_reject_incomplete_provider_payloads() {
        assert!(parse_eastmoney_page(&serde_json::json!({})).is_err());
        assert!(parse_eastmoney_page(&serde_json::json!({
            "data": {"total": 1}
        }))
        .is_err());
        let raw = serde_json::json!({
            "data": {"total": 3, "diff": [
                {"f12": "", "f13": 1, "f14": "ignored"},
                {"f12": "600000", "f13": 1, "f14": "浦发银行"},
                {"f12": "000001", "f14": "平安银行"}
            ]}
        });
        let (total, rows) = parse_eastmoney_page(&raw).unwrap();
        assert_eq!(total, 3);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].exchange, "SZSE");
    }

    #[test]
    fn pipe_directory_maps_all_provider_exchange_codes_and_missing_columns() {
        let raw = concat!(
            "ACT Symbol|Security Name|Exchange|Test Issue\n",
            "AAA|A|A|N\n",
            "AAP|P|P|N\n",
            "AAZ|Z|Z|N\n",
            "AAX|X|X|N\n",
            "BAD||N|N\n",
        );
        let rows = parse_pipe_directory(raw, "other");
        assert_eq!(
            rows.iter()
                .map(|row| row.exchange.as_str())
                .collect::<Vec<_>>(),
            ["NYSE American", "NYSE Arca", "BATS", "X"]
        );
        assert!(parse_pipe_directory("Symbol|Security Name\nA|A\n", "nasdaq").len() == 1);
        assert!(parse_pipe_directory("\nA|A\n", "nasdaq").is_empty());
    }

    #[test]
    fn dedupe_requires_a_complete_catalog_and_keeps_sorted_first_symbols() {
        let rows = vec![
            SymbolItem {
                symbol: "B".into(),
                name: "b".into(),
                exchange: "X".into(),
            },
            SymbolItem {
                symbol: "A".into(),
                name: "a".into(),
                exchange: "Y".into(),
            },
            SymbolItem {
                symbol: "A".into(),
                name: "duplicate".into(),
                exchange: "Z".into(),
            },
        ];
        assert!(dedupe_catalog(rows.clone(), 3).is_err());
        let result = dedupe_catalog(rows, 2).unwrap();
        assert_eq!(
            result
                .iter()
                .map(|row| row.symbol.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"]
        );
        assert_eq!(result[0].name, "a");
    }

    #[tokio::test]
    async fn search_uses_fixed_snapshot_for_trimmed_queries_pages_and_stale_refresh() {
        catalogs().write().await.clear();
        catalogs().write().await.insert(
            "a_share".into(),
            CatalogSnapshot {
                items: vec![
                    SymbolItem {
                        symbol: "600519".into(),
                        name: "贵州茅台".into(),
                        exchange: "SSE".into(),
                    },
                    SymbolItem {
                        symbol: "600001".into(),
                        name: "浦发银行".into(),
                        exchange: "SSE".into(),
                    },
                    SymbolItem {
                        symbol: "000519".into(),
                        name: "贵州测试".into(),
                        exchange: "SZSE".into(),
                    },
                ],
                fetched_at: Instant::now(),
            },
        );
        let (rows, total, universe, source, cached) =
            search("a_share", " 600 ", 1, 0).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "600519");
        assert_eq!((total, universe, source, cached), (2, 3, "cached", true));

        catalogs().write().await.insert(
            "us_stock".into(),
            CatalogSnapshot {
                items: seed("us_stock"),
                fetched_at: Instant::now() - CACHE_TTL - Duration::from_secs(1),
            },
        );
        // Claim the refresh slot so this contract test never reaches the live
        // provider; the stale branch itself remains fully exercised.
        refreshing().lock().await.insert("us_stock".into());
        let (_, _, _, source, cached) = search("us_stock", "", 0, 1).await.unwrap();
        assert_eq!((source, cached), ("stale", true));
        refreshing().lock().await.clear();
        catalogs().write().await.clear();
    }

    #[tokio::test]
    async fn unsupported_live_catalog_and_seed_are_explicit() {
        assert!(live_catalog("crypto").await.is_err());
        assert!(seed("crypto").is_empty());
        assert!(search("crypto", "A", 0, 1).await.is_err());
    }

    #[tokio::test]
    async fn local_provider_fixtures_cover_catalog_pagination_fallback_and_us_sources() {
        let (base, server) = fixture_server().await;
        let client = reqwest::Client::new();
        let paged = fetch_a_share_catalog_at(&client, &format!("{base}/east"))
            .await
            .unwrap();
        assert_eq!(paged.len(), 101);
        assert!(paged
            .iter()
            .any(|item| item.symbol == "430047" && item.exchange == "BSE"));

        let fallback = fetch_resilient_a_share_catalog_at(
            &client,
            &format!("{base}/unavailable"),
            &format!("{base}/mirror"),
        )
        .await
        .unwrap();
        assert_eq!(fallback.len(), 5000);
        assert_eq!(fallback[0].exchange, "BSE");

        let us =
            fetch_us_stock_catalog_at(&client, &format!("{base}/nasdaq"), &format!("{base}/other"))
                .await
                .unwrap();
        assert_eq!(us.len(), 5000);
        assert_eq!(us.last().unwrap().exchange, "NYSE Arca");
        server.abort();
    }

    #[tokio::test]
    async fn failed_refresh_keeps_the_last_complete_snapshot() {
        let original = vec![SymbolItem {
            symbol: "600519".into(),
            name: "贵州茅台".into(),
            exchange: "SSE".into(),
        }];
        catalogs().write().await.insert(
            "refresh_fixture".into(),
            CatalogSnapshot {
                items: original.clone(),
                fetched_at: Instant::now() - CACHE_TTL - Duration::from_secs(1),
            },
        );
        let refresh = spawn_catalog_refresh("refresh_fixture".into(), async {
            Err(anyhow::anyhow!("fixture provider unavailable"))
        });
        refresh.await.unwrap();
        let snapshot = catalogs()
            .read()
            .await
            .get("refresh_fixture")
            .cloned()
            .unwrap();
        assert_eq!(snapshot.items, original);
        catalogs().write().await.remove("refresh_fixture");
        refreshing().lock().await.clear();

        let refreshed = vec![SymbolItem {
            symbol: "000001".into(),
            name: "平安银行".into(),
            exchange: "SZSE".into(),
        }];
        let refresh = spawn_catalog_refresh("refresh_success".into(), {
            let refreshed = refreshed.clone();
            async move { Ok(refreshed) }
        });
        refresh.await.unwrap();
        assert_eq!(
            catalogs()
                .read()
                .await
                .get("refresh_success")
                .unwrap()
                .items,
            refreshed
        );
        catalogs().write().await.remove("refresh_success");
    }

    fn complete_fixture_items(count: usize) -> Vec<SymbolItem> {
        (0..count)
            .map(|index| SymbolItem {
                symbol: format!("{index:06}"),
                name: format!("fixture {index}"),
                exchange: "SZSE".into(),
            })
            .collect()
    }

    #[test]
    fn durable_catalog_snapshots_round_trip_atomically_and_reject_incomplete_files() {
        let dir = PathBuf::from(format!(
            "target/test-symbol-snapshot-{}",
            uuid::Uuid::new_v4()
        ));
        let items = complete_fixture_items(1_000);
        persist_catalog_snapshot_at(&dir, "a_share", &items, SystemTime::now()).unwrap();
        let loaded = load_persisted_catalog_from(&dir, "a_share").unwrap();
        assert_eq!(loaded.items, items);
        assert!(loaded.fetched_at.elapsed() < Duration::from_secs(5));
        let raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(snapshot_path(&dir, "a_share")).unwrap())
                .unwrap();
        assert_eq!(raw["version"], SNAPSHOT_VERSION);
        assert_eq!(raw["complete"], true);
        assert!(!std::fs::read_dir(&dir).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")));

        assert!(
            persist_catalog_snapshot_at(&dir, "a_share", &seed("a_share"), SystemTime::now())
                .is_err()
        );
        std::fs::write(
            snapshot_path(&dir, "us_stock"),
            serde_json::json!({
                "version": SNAPSHOT_VERSION,
                "fetched_at_unix": 0,
                "complete": false,
                "items": complete_fixture_items(5_000)
            })
            .to_string(),
        )
        .unwrap();
        assert!(load_persisted_catalog_from(&dir, "us_stock").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn durable_stale_snapshot_remains_complete_and_reports_age() {
        let dir = PathBuf::from(format!("target/test-symbol-stale-{}", uuid::Uuid::new_v4()));
        let items = complete_fixture_items(1_000);
        persist_catalog_snapshot_at(&dir, "a_share", &items, UNIX_EPOCH).unwrap();
        let loaded = load_persisted_catalog_from(&dir, "a_share").unwrap();
        assert!(loaded.fetched_at.elapsed() > CACHE_TTL);
        assert!(complete_snapshot("a_share", &loaded.items, Some(true)));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
