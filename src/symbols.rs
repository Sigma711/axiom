//! 可检索的公开证券目录。
//!
//! 这个模块把供应商目录的分页、去重、缓存和搜索放在一个深模块后面；
//! 调用者只给出市场、关键词与页码，不需要知道供应商的文件格式或市场代码。

use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

const PAGE_SIZE: usize = 100;
const MAX_SEARCH_LIMIT: usize = 100;
const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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

static CATALOGS: OnceLock<RwLock<HashMap<String, CatalogSnapshot>>> = OnceLock::new();
static REFRESHING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn catalogs() -> &'static RwLock<HashMap<String, CatalogSnapshot>> {
    CATALOGS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn refreshing() -> &'static Mutex<HashSet<String>> {
    REFRESHING.get_or_init(|| Mutex::new(HashSet::new()))
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

async fn eastmoney_page(client: &reqwest::Client, page: usize) -> Result<(usize, Vec<SymbolItem>)> {
    let raw: serde_json::Value = client
        .get("https://82.push2.eastmoney.com/api/qt/clist/get")
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

async fn fetch_a_share_catalog() -> Result<Vec<SymbolItem>> {
    let client = reqwest::Client::new();
    let (total, first) = eastmoney_page(&client, 1).await?;
    anyhow::ensure!(
        total >= first.len() && !first.is_empty(),
        "A-share directory is incomplete"
    );
    let pages = total.div_ceil(PAGE_SIZE);
    let rest = stream::iter(2..=pages)
        .map(|page| eastmoney_page(&client, page))
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

async fn fetch_a_share_github_catalog() -> Result<Vec<SymbolItem>> {
    let raw: serde_json::Value = reqwest::Client::new()
        .get("https://raw.githubusercontent.com/guidebee/china-stock-data/main/data/company/companies.json")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (compatible; AXIOM/1.0)")
        .timeout(Duration::from_secs(30))
        .send().await.context("A-share mirror directory request failed")?
        .error_for_status().context("A-share mirror directory returned HTTP error")?
        .json().await.context("invalid A-share mirror directory JSON")?;
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

async fn fetch_resilient_a_share_catalog() -> Result<Vec<SymbolItem>> {
    match fetch_a_share_catalog().await {
        Ok(items) => Ok(items),
        Err(primary) => fetch_a_share_github_catalog()
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

async fn fetch_us_stock_catalog() -> Result<Vec<SymbolItem>> {
    let client = reqwest::Client::new();
    let urls = [
        (
            "nasdaq",
            "https://www.nasdaqtrader.com/dynamic/SymDir/nasdaqlisted.txt",
        ),
        (
            "other",
            "https://www.nasdaqtrader.com/dynamic/SymDir/otherlisted.txt",
        ),
    ];
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
        "a_share" => fetch_resilient_a_share_catalog().await,
        "us_stock" => fetch_us_stock_catalog().await,
        _ => anyhow::bail!("unsupported symbol catalog source"),
    }
}

async fn refresh_catalog_in_background(source: &str) {
    let mut active = refreshing().lock().await;
    if !active.insert(source.to_owned()) {
        return;
    }
    let source = source.to_owned();
    tokio::spawn(async move {
        match live_catalog(&source).await {
            Ok(items) => {
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
    });
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
}
