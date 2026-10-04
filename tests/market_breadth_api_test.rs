use axiom::{
    data::MarketProvenance,
    market_breadth::{build_snapshot, MemberDailySeries, DOW_30},
    types::Bar,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::{Duration, TimeZone, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

fn environment_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn fixture(missing_latest: usize) -> Vec<MemberDailySeries> {
    DOW_30
        .iter()
        .enumerate()
        .map(|(member, symbol)| {
            let rising = member < 20;
            let take = if member < missing_latest { 259 } else { 260 };
            let bars = (0..take)
                .map(|day| {
                    let close = if rising {
                        100.0 + day as f64
                    } else {
                        400.0 - day as f64
                    };
                    Bar {
                        timestamp: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()
                            + Duration::days(day as i64),
                        open: close,
                        high: close + 1.0,
                        low: close - 1.0,
                        close,
                        volume: if rising { 1_000.0 } else { 500.0 },
                    }
                })
                .collect();
            MemberDailySeries {
                symbol: (*symbol).to_owned(),
                bars,
                provenance: MarketProvenance {
                    provider: "fixture".into(),
                    endpoint: format!("https://example.test/{symbol}"),
                    price_basis: "provider_adjustment_unverified".into(),
                    corporate_actions: "not_simulated".into(),
                },
                split_event_dates: Vec::new(),
            }
        })
        .collect()
}

#[test]
fn fixed_universe_snapshot_calculates_only_daily_supported_breadth_concepts() {
    let mut input = fixture(0);
    input[0].split_event_dates = vec![chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()];
    input[0].provenance.corporate_actions = "provider split event disclosed".into();
    let result = build_snapshot(input).unwrap();
    assert_eq!(result.universe.id, "dow_30_2024_11_08");
    assert_eq!(result.universe.member_count, 30);
    assert_eq!(result.coverage.latest_eligible_members, 30);
    assert_eq!(result.concepts.len(), 8);

    let concept = |id| result.concepts.iter().find(|item| item.id == id).unwrap();
    assert_eq!(concept("ad_line").latest, Some(2_590.0));
    assert_eq!(concept("trin").latest, Some(0.5));
    assert_eq!(concept("mcclellan").latest, Some(0.0));
    assert_eq!(concept("new_high_low").latest, Some(10.0));
    assert_eq!(concept("up_down_volume").latest, Some(4.0));
    assert_eq!(concept("breadth_thrust").triggered, Some(false));
    assert_eq!(concept("tick").status, "unavailable");
    assert!(concept("tick")
        .reason
        .as_deref()
        .unwrap()
        .contains("intraday"));
    assert_eq!(concept("bullish_percent").status, "unavailable");
    assert!(concept("bullish_percent")
        .reason
        .as_deref()
        .unwrap()
        .contains("Point & Figure"));
    assert_eq!(result.source.members[0].split_event_dates.len(), 1);
    assert!(result.source.members[0]
        .corporate_actions
        .contains("split event"));

    let summed = axiom::market_breadth::calculate_mcclellan_summation(&concept("mcclellan").series);
    assert_eq!(summed.len(), concept("mcclellan").series.len());
    assert_eq!(summed.last().unwrap().value, 0.0);
}

#[test]
fn sessions_below_ninety_percent_coverage_are_excluded_instead_of_counted_as_declines() {
    let result = build_snapshot(fixture(4)).unwrap();
    assert_eq!(result.coverage.minimum_required, 27);
    assert_eq!(result.coverage.latest_eligible_members, 30);
    assert_eq!(result.observations.last().unwrap().eligible_members, 30);
    assert_eq!(result.observations.len(), 258);
}

#[test]
fn fixed_universe_rejects_missing_or_duplicate_members() {
    let mut missing = fixture(0);
    missing.pop();
    assert!(build_snapshot(missing)
        .unwrap_err()
        .contains("fixed universe"));

    let mut duplicate = fixture(0);
    duplicate[1].symbol = duplicate[0].symbol.clone();
    assert!(build_snapshot(duplicate)
        .unwrap_err()
        .contains("fixed universe"));
}

#[tokio::test]
async fn http_snapshot_fails_closed_in_offline_mode() {
    let _guard = environment_lock().lock().await;
    std::env::set_var("AXIOM_OFFLINE", "1");
    let app = axiom::api::router(Arc::new(axiom::app_state::AppState::new(
        axiom::config::default_config(),
        PathBuf::from("target/market-breadth-test-cache"),
    )));
    let response = app
        .oneshot(
            Request::get("/api/market-breadth/snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    std::env::remove_var("AXIOM_OFFLINE");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

async fn yahoo_member_fixture(
    State(calls): State<Arc<AtomicUsize>>,
    Path(symbol): Path<String>,
) -> Json<Value> {
    calls.fetch_add(1, Ordering::SeqCst);
    let first = Utc::now() - Duration::days(262);
    let timestamps: Vec<_> = (0..260)
        .map(|day| (first + Duration::days(day)).timestamp())
        .collect();
    let closes: Vec<_> = (0..260).map(|day| 100.0 + day as f64).collect();
    let opens = closes.clone();
    let highs: Vec<_> = closes.iter().map(|close| close + 1.0).collect();
    let lows: Vec<_> = closes.iter().map(|close| close - 1.0).collect();
    let volumes = vec![1_000.0; 260];
    Json(json!({"chart":{"error":null,"result":[{
        "meta":{"symbol":symbol,"currency":"USD","instrumentType":"EQUITY"},
        "timestamp":timestamps,
        "events":{},
        "indicators":{"quote":[{"open":opens,"high":highs,"low":lows,"close":closes,"volume":volumes}]}
    }]}}))
}

async fn exchange_time_fixture() -> Json<Value> {
    Json(json!({"serverTime":1_600_000_103_000_i64}))
}

async fn aggregate_trade_fixture(Query(query): Query<HashMap<String, String>>) -> Json<Value> {
    let symbol = query.get("symbol").unwrap();
    let (before, after) = match symbol.as_str() {
        "BTCUSDT" => ("100", "101"),
        "ETHUSDT" => ("200", "199"),
        _ => ("300", "300"),
    };
    Json(json!([
        {"a":1,"p":before,"T":1_600_000_098_000_i64},
        {"a":2,"p":after,"T":1_600_000_099_000_i64}
    ]))
}

#[tokio::test]
async fn live_fixed_basket_fetch_is_source_bound_and_cached_for_fifteen_minutes() {
    let _guard = environment_lock().lock().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route("/v8/finance/chart/:symbol", get(yahoo_member_fixture))
        .route("/api/v3/time", get(exchange_time_fixture))
        .route("/api/v3/aggTrades", get(aggregate_trade_fixture))
        .with_state(calls.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut feed = axiom::data::HttpFeed::new("target/market-breadth-live-cache");
    feed.us_stock_adjustment_url = format!("http://{address}/v8/finance/chart");
    feed.us_stock_adjustment_fallback_url = feed.us_stock_adjustment_url.clone();
    feed.us_stock_relay_url = None;

    let fresh = axiom::market_breadth::fetch_live_snapshot(&feed)
        .await
        .unwrap();
    assert_eq!(fresh.source.cache_status, "fetched_live");
    assert_eq!(fresh.source.members.len(), 30);
    assert_eq!(calls.load(Ordering::SeqCst), 30);
    let cached = axiom::market_breadth::fetch_live_snapshot(&feed)
        .await
        .unwrap();
    assert_eq!(cached.source.cache_status, "memory_cache_15m");
    assert_eq!(cached.source.retrieved_at, fresh.source.retrieved_at);
    assert_eq!(calls.load(Ordering::SeqCst), 30);

    let root = std::env::temp_dir().join(format!("axiom-breadth-{}", uuid::Uuid::new_v4()));
    let mut state = axiom::app_state::AppState::new(axiom::config::default_config(), root.clone());
    state.feed = Arc::new(feed);
    let api = axiom::api::router(Arc::new(state));
    let catalog = api
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value = serde_json::from_slice(
        &axum::body::to_bytes(catalog.into_body(), 2_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    for id in axiom::market_breadth::MARKET_BREADTH_IDS {
        let item = catalog["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == id)
            .unwrap();
        assert_eq!(item["input_kind"], "market_breadth_case");
        assert_eq!(item["inputs"], json!([]));
        assert_eq!(item["plan"]["source_policy"], "real_required");
        if id == "tick" {
            assert_eq!(item["plan"]["markets"], json!(["crypto"]));
            assert_eq!(item["plan"]["universe"], "fixed_binance_spot_btc_eth_bnb");
        } else {
            assert_eq!(item["plan"]["markets"], json!(["us_equity"]));
            assert_eq!(item["plan"]["universe"], "fixed_dow30_2024_11_08");
        }
    }
    let book_sum = catalog["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "book_mcclellan_sum")
        .unwrap();
    assert_eq!(book_sum["input_kind"], "market_breadth_case");
    assert_eq!(book_sum["inputs"], json!([]));
    let request =
        json!({"concept_id":"ad_line","module":"data","source":"market_breadth","inputs":{}});
    let response = api
        .clone()
        .oneshot(
            Request::post("/api/practice")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 10_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["input_kind"], "market_breadth_case");
    assert_eq!(
        body["market_breadth"]["concepts"].as_array().unwrap().len(),
        8
    );
    assert!(body["values"]["ad_line"].is_number());
    assert_eq!(body["source_markets"], json!({"daily":"us_equity"}));

    let book_response = api
        .clone()
        .oneshot(
            Request::post("/api/practice")
                .header("content-type", "application/json")
                .body(Body::from(json!({"concept_id":"book_mcclellan_sum","module":"data","source":"market_breadth","inputs":{}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(book_response.status(), StatusCode::OK);
    let book_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(book_response.into_body(), 10_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(book_body["values"]["book_mcclellan_sum"].is_number());
    assert_eq!(
        book_body["provenance"],
        "server_fetched_fixed_market_breadth_snapshot"
    );

    // A TICK practice request depends only on the Binance aggregate-trade
    // source. Deliberately make both Yahoo endpoints unreachable and verify
    // that no equity request is attempted.
    let mut tick_only_feed = axiom::data::HttpFeed::new(
        std::env::temp_dir().join(format!("axiom-tick-only-{}", uuid::Uuid::new_v4())),
    );
    tick_only_feed.base_url = format!("http://{address}");
    tick_only_feed.us_stock_adjustment_url = "http://127.0.0.1:9/never-yahoo".into();
    tick_only_feed.us_stock_adjustment_fallback_url =
        "http://127.0.0.1:9/never-yahoo-fallback".into();
    tick_only_feed.us_stock_relay_url = None;
    let mut tick_only_state = axiom::app_state::AppState::new(
        axiom::config::default_config(),
        std::env::temp_dir().join(format!("axiom-tick-state-{}", uuid::Uuid::new_v4())),
    );
    tick_only_state.feed = Arc::new(tick_only_feed);
    let tick_only_api = axiom::api::router(Arc::new(tick_only_state));
    let yahoo_calls_before_tick = calls.load(Ordering::SeqCst);
    let tick_response = tick_only_api
        .oneshot(
            Request::post("/api/practice")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"concept_id":"tick","module":"data","source":"market_breadth","inputs":{}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tick_response.status(), StatusCode::OK);
    let tick_body: Value = serde_json::from_slice(
        &axum::body::to_bytes(tick_response.into_body(), 2_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(tick_body["values"]["tick"], 0);
    assert_eq!(tick_body["source_markets"], json!({"tick":"crypto_spot"}));
    assert_eq!(
        tick_body["provenance"],
        "server_fetched_fixed_crypto_tick_snapshot"
    );
    assert!(tick_body.get("market_tick").is_some());
    assert!(tick_body.get("market_breadth").is_none());
    assert_eq!(calls.load(Ordering::SeqCst), yahoo_calls_before_tick);

    let invalid = api
        .oneshot(
            Request::post("/api/practice")
                .header("content-type", "application/json")
                .body(Body::from(json!({"concept_id":"trin","module":"data","source":"binance","symbol":"BTCUSDT","inputs":{"advances":1}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    server.abort();
    let _ = tokio::fs::remove_dir_all(root).await;
}

#[test]
fn undefined_daily_inputs_and_tick_attachment_are_explicit() {
    let mut flat = fixture(0);
    for member in &mut flat {
        for bar in &mut member.bars {
            bar.open = 100.0;
            bar.high = 101.0;
            bar.low = 99.0;
            bar.close = 100.0;
        }
    }
    let result = build_snapshot(flat).unwrap();
    assert_eq!(
        result
            .concepts
            .iter()
            .find(|x| x.id == "breadth_thrust")
            .unwrap()
            .status,
        "unavailable"
    );

    let mut gapped = fixture(0);
    for member in &mut gapped {
        let previous = member.bars[19].close;
        member.bars[20].open = previous;
        member.bars[20].high = previous + 1.0;
        member.bars[20].low = previous - 1.0;
        member.bars[20].close = previous;
    }
    let gapped = build_snapshot(gapped).unwrap();
    assert_eq!(
        gapped
            .concepts
            .iter()
            .find(|x| x.id == "breadth_thrust")
            .unwrap()
            .status,
        "available"
    );

    let one_bar: Vec<_> = fixture(0)
        .into_iter()
        .map(|mut member| {
            member.bars.truncate(1);
            member
        })
        .collect();
    assert!(build_snapshot(one_bar)
        .unwrap_err()
        .contains("90% coverage"));

    let sample_at = Utc.timestamp_millis_opt(1_600_000_100_000).unwrap();
    let payloads: Vec<_> = [
        ("BTCUSDT", "100", "101"),
        ("ETHUSDT", "200", "199"),
        ("BNBUSDT", "300", "300"),
    ]
    .into_iter()
    .map(|(symbol, before, after)| {
        (
            symbol.into(),
            json!([
                {"a":1,"p":before,"T":1_600_000_098_000_i64},
                {"a":2,"p":after,"T":1_600_000_099_000_i64}
            ]),
        )
    })
    .collect();
    let tick = axiom::market_tick::calculate_crypto_tick(sample_at, &payloads).unwrap();
    let attached = build_snapshot(fixture(0))
        .unwrap()
        .attach_crypto_tick(Ok(tick));
    assert_eq!(
        attached
            .concepts
            .iter()
            .find(|x| x.id == "tick")
            .unwrap()
            .status,
        "available"
    );
    let failed = build_snapshot(fixture(0))
        .unwrap()
        .attach_crypto_tick(Err("upstream".into()));
    assert!(failed
        .concepts
        .iter()
        .find(|x| x.id == "tick")
        .unwrap()
        .reason
        .as_deref()
        .unwrap()
        .contains("upstream"));
}

#[test]
fn bullish_percent_counts_only_verified_persistent_point_and_figure_signals() {
    let mut members = fixture(0);
    for (index, member) in members.iter_mut().enumerate() {
        let pattern = if index < 20 {
            [100.0, 110.0, 100.0, 112.0, 108.0]
        } else {
            [100.0, 90.0, 100.0, 88.0, 92.0]
        };
        let offset = member.bars.len() - pattern.len();
        for (bar, close) in member.bars[offset..].iter_mut().zip(pattern) {
            bar.open = close;
            bar.high = close + 1.0;
            bar.low = close - 1.0;
            bar.close = close;
        }
    }
    let result = build_snapshot(members).unwrap();
    let bpi = result
        .concepts
        .iter()
        .find(|item| item.id == "bullish_percent")
        .unwrap();
    assert_eq!(bpi.status, "available");
    assert!((bpi.latest.unwrap() - 200.0 / 3.0).abs() < 1e-10);
    assert_eq!(bpi.inputs["eligible_signals"], 30);
}
