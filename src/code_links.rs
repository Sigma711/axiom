//! Stable AST-derived references to the exact source embedded in this build.
//! Clean builds use immutable GitHub commit links. Dirty/archived builds expose
//! the embedded-source viewer so line numbers never silently refer to old code.
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};
fn manifest() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/source_map.json")))
            .expect("build generated valid source map")
    })
}
#[derive(Debug, Clone, Serialize)]
pub struct CodeLocation {
    pub code_ref: String,
    pub path: String,
    pub symbol: String,
    pub line: usize,
    pub end_line: usize,
    pub kind: String,
    pub revision: Option<String>,
    pub dirty: bool,
    pub url: String,
    pub github_url: Option<String>,
    pub source_url: String,
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
/// Only embedded project source is readable; traversal and arbitrary files fail.
pub fn source(path: &str) -> Option<&'static str> {
    manifest()["sources"].get(path)?.as_str()
}
pub fn revision() -> Option<&'static str> {
    manifest()["revision"].as_str()
}
pub fn is_dirty() -> bool {
    manifest()["dirty"].as_bool().unwrap_or(true)
}
pub fn resolve(reference: &str) -> Option<CodeLocation> {
    let reference = reference.trim();
    let symbols = manifest()["symbols"].as_object()?;
    let (key, row) = if let Some(row) = symbols.get(reference) {
        (reference.to_string(), row)
    } else {
        // Unique legacy short method names can still resolve, never choose an
        // arbitrary overload. Concept references below always use canonical cases.
        let (path, symbol) = reference.split_once("::")?;
        let mut matches = symbols.iter().filter(|(_, row)| {
            row["path"] == path
                && row["symbol"]
                    .as_str()
                    .is_some_and(|s| s == symbol || s.ends_with(&format!("::{symbol}")))
        });
        let found = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        (found.0.clone(), found.1)
    };
    let path = row["path"].as_str()?.to_owned();
    let symbol = row["symbol"].as_str()?.to_owned();
    let line = row["line"].as_u64()? as usize;
    let end_line = row["end_line"].as_u64()? as usize;
    let source_url = format!(
        "/api/code/source?path={}&line={line}&end_line={end_line}#L{line}",
        encode(&path)
    );
    let dirty = is_dirty();
    let revision = revision().map(str::to_owned);
    let github_url = if !dirty {
        manifest()["repository"]
            .as_str()
            .zip(revision.as_deref())
            .map(|(repo, rev)| format!("{repo}/blob/{rev}/{path}#L{line}-L{end_line}"))
    } else {
        None
    };
    Some(CodeLocation {
        code_ref: key,
        path,
        symbol,
        line,
        end_line,
        kind: row["kind"].as_str().unwrap_or("symbol").into(),
        revision,
        dirty,
        url: github_url.clone().unwrap_or_else(|| source_url.clone()),
        github_url,
        source_url,
    })
}
fn first_existing(candidates: impl IntoIterator<Item = String>) -> Option<String> {
    candidates
        .into_iter()
        .find(|r| manifest()["symbols"].get(r).is_some())
}
fn concept_routes() -> &'static BTreeMap<String, String> {
    static ROUTES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    ROUTES.get_or_init(|| {
        let mut out = BTreeMap::new();
        for c in crate::practice::base_catalog() {
            let candidates = if c.id == "volume_profile" {
                vec!["src/book.rs::market_recent_trade_summary".to_string()]
            } else if c.id == "bid_ask_spread" {
                vec!["src/book.rs::market_binance_depth_summary".to_string()]
            } else if matches!(c.id.as_str(), "inside_outside" | "cvd") {
                vec!["src/book.rs::market_binance_aggressor_summary".to_string()]
            } else {
                vec![
                    format!("src/practice/market.rs::evaluate::{}", c.id),
                    format!("src/practice/independent.rs::evaluate::{}", c.id),
                ]
            };
            if let Some(r) = first_existing(candidates) {
                out.insert(c.id, r);
            }
        }
        for c in crate::book_technical::catalog() {
            let id = c.id.strip_prefix("book_").unwrap_or(&c.id);
            let candidates = if c.id == "book_pitfall_order_imbalance" {
                vec!["src/book.rs::market_binance_depth_summary".to_string()]
            } else if c.id == "book_net_volume" {
                vec!["src/book.rs::market_recent_trade_summary".to_string()]
            } else if c.id == "book_pitfall_open_candle" {
                vec!["src/book.rs::market_open_candle_summary".to_string()]
            } else if c.id == "book_pitfall_formula_variant" {
                vec!["src/book.rs::market_formula_variant_summary".to_string()]
            } else if c.id == "book_pitfall_timeframe" {
                vec!["src/book.rs::market_timeframe_summary".to_string()]
            } else {
                vec![
                    format!("src/book_technical/market.rs::evaluate::{id}"),
                    format!("src/book_technical/external.rs::evaluate::{id}"),
                ]
            };
            if let Some(r) = first_existing(candidates) {
                out.insert(c.id, r);
            }
        }
        for c in crate::book_charts::catalog() {
            if let Some(r) = first_existing([format!("src/book_charts.rs::evaluate::{}", c.id)]) {
                out.insert(c.id, r);
            }
        }
        for c in crate::book::catalog() {
            let references = match c.id.as_str() {
                "book_order_imbalance" => {
                    vec!["src/book.rs::market_binance_depth_summary".to_string()]
                }
                "book_nonstandard_bar" => vec!["src/book.rs::nonstandard_bar_ohlc4".to_string()],
                "book_period" => vec!["src/book.rs::market_period_summary".to_string()],
                "book_trade_volume" => {
                    vec!["src/book.rs::market_trade_volume_summary".to_string()]
                }
                "book_order_flow" => {
                    vec!["src/book.rs::market_binance_aggressor_summary".to_string()]
                }
                _ => vec![format!("src/book.rs::evaluate::{}", c.id)],
            };
            if let Some(r) = first_existing(references) {
                out.insert(c.id, r);
            }
        }
        for c in crate::workflows::catalog() {
            if let Some(r) = first_existing([format!("src/workflows.rs::evaluate::{}", c.id)]) {
                out.insert(c.id, r);
            }
        }
        for d in crate::supplement::definitions() {
            if let (Some(id), Some(op)) = (d["id"].as_str(), d["op"].as_str()) {
                let candidates = if matches!(
                    id,
                    "book_block_height"
                        | "book_block_size"
                        | "book_block_interval"
                        | "book_transaction_rate"
                        | "book_transaction_fees"
                        | "book_transaction_bytes"
                        | "book_utxo_value_stats"
                        | "book_utxo_counts"
                        | "book_utxo_totals"
                ) {
                    vec![
                        if matches!(id, "book_transaction_fees" | "book_transaction_bytes") {
                            "src/book.rs::market_bitcoin_transaction_summary".to_string()
                        } else if matches!(
                            id,
                            "book_utxo_value_stats" | "book_utxo_counts" | "book_utxo_totals"
                        ) {
                            "src/book.rs::market_bitcoin_utxo_summary".to_string()
                        } else {
                            "src/book.rs::market_bitcoin_block_summary".to_string()
                        },
                    ]
                } else {
                    vec![format!("src/supplement.rs::evaluate::{op}")]
                };
                if let Some(r) = first_existing(candidates) {
                    out.insert(id.into(), r);
                }
            }
        }
        out
    })
}
/// Canonical references include the actual evaluator case, not an entire file.
pub fn concept_ref(id: &str) -> Option<String> {
    concept_routes().get(id).cloned()
}
pub fn for_concept(id: &str) -> Option<CodeLocation> {
    resolve(&concept_ref(id)?)
}
/// Line-count and content are from this binary's build, not the mutable checkout.
pub fn excerpt(location: &CodeLocation) -> Option<String> {
    Some(
        source(&location.path)?
            .lines()
            .skip(location.line.saturating_sub(1))
            .take(location.end_line.saturating_sub(location.line) + 1)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}
