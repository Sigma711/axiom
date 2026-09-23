use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    data::{DataFeed, HttpFeed, SyntheticFeed},
    practice,
};
use axum::{
    body::{to_bytes, Body},
    extract::Query,
    http::{Request, StatusCode},
    routing::get,
    Json,
};
use chrono::{TimeZone, Timelike, Utc};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tower::ServiceExt;

fn app() -> axum::Router {
    api::router(Arc::new(AppState::new(
        default_config(),
        PathBuf::from("target/practice-api-data"),
    )))
}
async fn request(app: &axum::Router, path: &str, value: Value) -> (StatusCode, Value) {
    let r = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let b = to_bytes(r.into_body(), 10_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&b).unwrap_or_else(|_| json!(String::from_utf8_lossy(&b))),
    )
}

#[tokio::test]
async fn data_practice_accepts_only_concepts_with_a_data_plan() {
    let app = app();
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            300,
        )
        .unwrap();
    for concept in practice::catalog().into_iter().filter(|concept| {
        concept.input_kind == "market_bars"
            && !matches!(
                concept.id.as_str(),
                "book_cdp"
                    | "book_pitfall_repainting"
                    | "book_pitfall_timeframe"
                    | "book_pitfall_formula_variant"
                    | "book_pitfall_open_candle"
                    | "book_trade_volume"
            )
    }) {
        let mut first: Option<Value> = None;
        for module in ["data"] {
            let(status,out)=request(&app,"/api/practice",json!({"concept_id":concept.id,"module":module,"symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{}})).await;
            assert_eq!(status, StatusCode::OK, "{} {module}: {out}", concept.id);
            assert_eq!(out["concept_id"], concept.id);
            assert_eq!(out["module"], module);
            assert!(
                ["computed", "undefined"].contains(&out["status"].as_str().unwrap()),
                "{}: {out}",
                concept.id
            );
            if out["status"] == "undefined" {
                assert!(
                    !out["reason"].as_str().unwrap_or("").is_empty(),
                    "{} needs an explanation",
                    concept.id
                );
            }
            assert!(
                !out["values"].as_object().unwrap().is_empty(),
                "{} has no output",
                concept.id
            );
            for (key, value) in out["values"].as_object().unwrap() {
                assert!(
                    out["units"][key].is_string(),
                    "{} {key} missing unit",
                    concept.id
                );
                assert!(
                    value.is_null() || value.as_f64().is_some_and(f64::is_finite),
                    "{} {key} invalid",
                    concept.id
                );
            }
            if let Some(first) = &first {
                assert_eq!(
                    first, &out["values"],
                    "{} differs across identical module inputs",
                    concept.id
                );
            } else {
                first = Some(out["values"].clone());
            }
            if concept.input_kind == "market_bars" {
                assert_eq!(out["context"], "module_snapshot");
                assert_eq!(out["provenance"], "provided_market_bars");
            }
        }
    }
}

#[tokio::test]
async fn teaching_and_result_practice_contexts_are_explicit_and_validated() {
    let app = app();
    let (status,out)=request(&app,"/api/practice",json!({"concept_id":"book_funding","module":"data","source":"real","inputs":{"is_long":false}})).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["values"]["payment"], -1.0);
    assert_eq!(out["bars"], json!([]));
    assert_eq!(out["context"], "editable_teaching_inputs");
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            3,
        )
        .unwrap();
    let result_inputs = json!({
        "equity":[100.0,101.0,102.0,103.0],
        "returns":[0.01,0.0099009901,0.0098039216],
        "elapsed_days":3.0,
        "periods_per_year":365.0,
        "risk_free_annual":0.0
    });
    let (status, out) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"total_return","module":"backtest","symbol":"BTCUSDT","source":"synthetic","bars":bars,"inputs":result_inputs}),
    ).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["context"], "provided_result_context");
    assert_eq!(out["provenance"], "provided_result_context");
    for body in [
        json!({"concept_id":"sma","module":"unknown"}),
        json!({"concept_id":"not_registered","module":"data"}),
        json!({"concept_id":"rsi","module":"data","inputs":{"period":0}}),
        json!({"concept_id":"book_funding","module":"paper","inputs":{"rate":0.1}}),
        json!({"concept_id":"eps","module":"compare","inputs":[]}),
        json!({"concept_id":"total_return","module":"data","source":"synthetic","bars":[{"timestamp":"2024-01-01T00:00:00Z","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":0.0}],"inputs":{"equity":[100.0,101.0],"elapsed_days":1.0}}),
        json!({"concept_id":"total_return","module":"backtest","source":"synthetic","inputs":{"equity":[100.0,101.0],"elapsed_days":1.0}}),
        json!({"concept_id":"sharpe","module":"backtest","source":"synthetic","bars":[{"timestamp":"2024-01-01T00:00:00Z","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":0.0}],"inputs":{}}),
        json!({"concept_id":"beta","module":"backtest","source":"synthetic","bars":[{"timestamp":"2024-01-01T00:00:00Z","open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":0.0}],"inputs":{"strategy_returns":[0.01,0.02],"benchmark_returns":[0.01]}}),
        json!({"concept_id":"eps","module":"data","source":"binance","inputs":{}}),
    ] {
        let (status, _) = request(&app, "/api/practice", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, _) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"rsi","module":"data","bars":[]}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn repainting_practice_has_no_manual_event_or_bar_override() {
    let app = app();
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    let concept = catalog["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "book_pitfall_repainting")
        .unwrap();
    assert_eq!(concept["input_kind"], "market_bars");
    assert_eq!(concept["inputs"], json!([]));
    assert_eq!(concept["plan"]["modules"], json!(["data"]));
    assert_eq!(concept["plan"]["source_policy"], "real_required");
    assert!(concept["plan"]["goal"].as_str().unwrap().contains("t+2"));

    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            8,
        )
        .unwrap();
    let (status, body) = request(
        &app,
        "/api/practice",
        json!({
            "concept_id":"book_pitfall_repainting", "module":"data", "symbol":"BTCUSDT",
            "source":"binance", "bars":bars, "inputs":{"pivot_index":0}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body
        .to_string()
        .contains("does not accept caller-supplied bars"));

    let (status, _) = request(&app, "/api/practice", json!({
        "concept_id":"book_pitfall_repainting", "module":"data", "symbol":"BTCUSDT", "source":"synthetic"
    })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn timeframe_practice_requires_server_fetched_real_source_bars() {
    let app = app();
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    let concept = catalog["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "book_pitfall_timeframe")
        .unwrap();
    assert_eq!(concept["input_kind"], "market_bars");
    assert_eq!(concept["inputs"], json!([]));
    assert_eq!(concept["plan"]["modules"], json!(["data"]));
    assert_eq!(concept["plan"]["source_policy"], "real_required");
    assert!(concept["plan"]["goal"]
        .as_str()
        .unwrap()
        .contains("不同时间尺度"));

    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            21,
        )
        .unwrap();
    let (status, body) = request(
        &app,
        "/api/practice",
        json!({
            "concept_id":"book_pitfall_timeframe", "module":"data", "symbol":"BTCUSDT",
            "source":"binance", "bars":bars, "inputs":{}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body
        .to_string()
        .contains("server-fetched completed source bars"));
}

#[tokio::test]
async fn open_candle_practice_requires_server_fetched_provisional_binance_snapshot() {
    let app = app();
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    let concept = catalog["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "book_pitfall_open_candle")
        .unwrap();
    assert_eq!(concept["input_kind"], "market_bars");
    assert_eq!(concept["inputs"], json!([]));
    assert_eq!(concept["plan"]["markets"], json!(["crypto"]));
    assert_eq!(concept["plan"]["source_policy"], "real_required");
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            2,
        )
        .unwrap();
    let (status, body) = request(&app, "/api/practice", json!({"concept_id":"book_pitfall_open_candle","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{"final_close":99}})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.to_string().contains("server-fetched"));
}

#[tokio::test]
async fn formula_variant_practice_requires_server_fetched_completed_market_bars() {
    let app = app();
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    let concept = catalog["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "book_pitfall_formula_variant")
        .unwrap();
    assert_eq!(concept["input_kind"], "market_bars");
    assert_eq!(concept["inputs"], json!([]));
    assert_eq!(concept["plan"]["modules"], json!(["data"]));
    assert_eq!(concept["plan"]["source_policy"], "real_required");
    assert!(concept["plan"]["goal"].as_str().unwrap().contains("MACD"));
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            45,
        )
        .unwrap();
    let (status, body) = request(&app, "/api/practice", json!({"concept_id":"book_pitfall_formula_variant","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{}})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body
        .to_string()
        .contains("server-fetched completed source bars"));
}

async fn mock_completed_klines(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let start = q["startTime"].parse::<i64>().unwrap();
    let limit = q["limit"].parse::<usize>().unwrap();
    Json(Value::Array(
        (0..limit)
            .map(|index| {
                let close = 100.0 + index as f64 * 0.1 + (index as f64 / 5.0).sin();
                json!([
                    start + index as i64 * 3_600_000,
                    format!("{close}"),
                    format!("{}", close + 2.0),
                    format!("{}", close - 2.0),
                    format!("{close}"),
                    "10",
                    start + (index as i64 + 1) * 3_600_000 - 1
                ])
            })
            .collect(),
    ))
}

async fn mock_binance_time() -> Json<Value> {
    Json(json!({"serverTime": Utc::now().timestamp_millis()}))
}

async fn mock_trade_volume_klines(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    assert_eq!(q.get("symbol").map(String::as_str), Some("BTCUSDT"));
    assert_eq!(q.get("interval").map(String::as_str), Some("1h"));
    assert_eq!(q.get("limit").map(String::as_str), Some("25"));
    let start = q["startTime"].parse::<i64>().unwrap();
    Json(Value::Array(
        (1..=25)
            .map(|index| {
                json!([
                    start + index as i64 * 3_600_000,
                    "100",
                    "120",
                    "90",
                    "110",
                    "10",
                    start + (index as i64 + 1) * 3_600_000 - 1,
                    if index == 24 { "1017" } else { "1000" }
                ])
            })
            .collect(),
    ))
}

#[tokio::test]
async fn open_candle_practice_reports_upstream_time_failure_without_a_stale_snapshot() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route("/api/v3/time", get(|| async { StatusCode::BAD_GATEWAY })),
        )
        .await
        .unwrap()
    });
    let dir = PathBuf::from(format!(
        "target/practice-open-failure-{}",
        uuid::Uuid::new_v4()
    ));
    let mut state = AppState::new(default_config(), dir.clone());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    state.feed = Arc::new(feed);
    let app = api::router(Arc::new(state));
    let (status, body) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_pitfall_open_candle","module":"data","symbol":"BTCUSDT","source":"binance","limit":2,"inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body
        .to_string()
        .contains("Binance server time returned HTTP error"));
    assert!(body.get("provisional_snapshot").is_none());
    server.abort();
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn server_fetched_knowledge_practices_execute_against_completed_mock_market() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new()
                .route("/api/v3/klines", get(mock_completed_klines))
                .route("/api/v3/time", get(mock_binance_time)),
        )
        .await
        .unwrap()
    });
    let dir = PathBuf::from(format!("target/practice-real-{}", uuid::Uuid::new_v4()));
    let mut state = AppState::new(default_config(), dir.clone());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    state.feed = Arc::new(feed);
    let app = api::router(Arc::new(state));
    for concept_id in [
        "book_pitfall_repainting",
        "book_pitfall_timeframe",
        "book_pitfall_formula_variant",
        "book_pitfall_open_candle",
    ] {
        let (status, body) = request(
            &app,
            "/api/practice",
            json!({
                "concept_id": concept_id, "module": "data", "symbol": "BTCUSDT",
                "source": "binance", "limit": 200, "inputs": {}
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{concept_id}: {body}");
        assert_eq!(body["status"], "computed");
        if concept_id == "book_pitfall_open_candle" {
            assert_eq!(
                body["bar_origin"],
                "server_fetched_binance_provisional_snapshot"
            );
            assert_eq!(body["bars"].as_array().unwrap().len(), 1);
            assert_eq!(body["provisional_snapshot"]["is_closed"], false);
            assert_eq!(
                body["provisional_snapshot"]["candle"]["close"],
                body["values"]["provisional_close"]
            );
            assert_eq!(body["values"]["is_current_candle_closed"], 0.0);
            assert!(body["values"].get("final_close").is_none());
            assert!(body["completion_evidence"]
                .as_str()
                .unwrap()
                .contains("timestamp-derived"));
        } else {
            assert_eq!(body["bar_origin"], "server_fetched_completed_source_bars");
            assert_eq!(body["bars"].as_array().unwrap().len(), 200);
        }
        assert_eq!(body["context"], "selected_dataset");
    }
    server.abort();
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn trade_volume_practice_uses_binance_quote_notional_and_rejects_client_data() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new()
                .route("/api/v3/klines", get(mock_trade_volume_klines))
                .route("/api/v3/time", get(mock_binance_time)),
        )
        .await
        .unwrap()
    });
    let dir = PathBuf::from(format!(
        "target/practice-trade-volume-{}",
        uuid::Uuid::new_v4()
    ));
    let mut state = AppState::new(default_config(), dir.clone());
    let mut feed = HttpFeed::new(&dir);
    feed.base_url = format!("http://127.0.0.1:{port}");
    state.feed = Arc::new(feed);
    let app = api::router(Arc::new(state));
    let (status, body) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_trade_volume","module":"data","symbol":"BTCUSDT","source":"binance","limit":2,"inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["values"]["quote_volume"], 1_017.0);
    assert_ne!(body["values"]["quote_volume"], 1_100.0);
    assert_eq!(body["values"]["vwap"], 101.7);
    assert_eq!(
        body["asset_units"],
        json!({"base_asset":"BTC","quote_asset":"USDT"})
    );
    assert_eq!(
        body["bar_origin"],
        "server_fetched_completed_binance_usdt_spot_bars"
    );
    assert_eq!(body["bars"].as_array().unwrap().len(), 24);
    assert_eq!(body["series"][1]["name"], "quote_volume_series");
    for invalid in [
        json!({"source":"synthetic","symbol":"BTCUSDT","bars":null,"inputs":{}}),
        json!({"source":"binance","symbol":"BTCFDUSD","bars":null,"inputs":{}}),
        json!({"source":"binance","symbol":"BTCUSDT","bars":[],"inputs":{}}),
        json!({"source":"binance","symbol":"BTCUSDT","bars":null,"inputs":{"price":1}}),
    ] {
        let mut request_body = json!({"concept_id":"book_trade_volume","module":"data","limit":2});
        request_body
            .as_object_mut()
            .unwrap()
            .extend(invalid.as_object().unwrap().clone());
        let (status, _) = request(&app, "/api/practice", request_body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    server.abort();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn websocket_pushes_again_without_needing_a_client_message() {
    use futures::StreamExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move { axum::serve(listener, app()).await.unwrap() });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/api/paper/ws"))
            .await
            .unwrap();
    for _ in 0..2 {
        let message = tokio::time::timeout(std::time::Duration::from_secs(4), socket.next())
            .await
            .expect("server must send periodically")
            .unwrap()
            .unwrap();
        let snapshot: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        assert!(snapshot["equity"].as_f64().unwrap().is_finite());
        assert!(snapshot["is_running"].is_boolean());
    }
    socket.close(None).await.unwrap();
    server.abort();
}

#[tokio::test]
async fn every_catalog_entry_publishes_a_non_forced_real_practice_plan() {
    let response = app()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 10_000_000).await.unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let concepts = document["concepts"].as_array().unwrap();
    assert_eq!(concepts.len(), practice::catalog().len());
    for concept in concepts {
        let plan = &concept["plan"];
        assert!(
            plan["markets"].as_array().is_some_and(|v| !v.is_empty()),
            "{} has no applicable market",
            concept["id"]
        );
        assert!(
            plan["modules"].as_array().is_some_and(|v| !v.is_empty()),
            "{} has no meaningful destination",
            concept["id"]
        );
        assert!(
            plan["required_datasets"]
                .as_array()
                .is_some_and(|v| !v.is_empty()),
            "{} has no evidence requirement",
            concept["id"]
        );
        assert!(matches!(
            plan["source_policy"].as_str(),
            Some("real_required" | "result_required" | "evidence_required")
        ));
        assert!(!plan["goal"].as_str().unwrap_or("").is_empty());
        if concept["category"] == "风险-绩效" {
            assert_eq!(plan["source_policy"], "result_required");
            assert_eq!(plan["modules"], json!(["backtest", "paper", "compare"]));
            let required = plan["required_datasets"].as_array().unwrap();
            let benchmark_dependent = matches!(
                concept["id"].as_str(),
                Some(
                    "information_ratio"
                        | "treynor"
                        | "tracking_error"
                        | "capture_ratio"
                        | "beta"
                        | "alpha"
                )
            );
            assert_eq!(
                required
                    .iter()
                    .any(|item| item == "same_period_benchmark_returns"),
                benchmark_dependent,
                "{} benchmark requirement",
                concept["id"]
            );
        }
    }
}

#[tokio::test]
async fn book_financial_practices_require_equity_evidence_and_reject_crypto() {
    let app = app();
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let document: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    for id in [
        "book_current_ratio",
        "book_bank_nim",
        "book_dcf",
        "book_share_counts",
        "book_adjustment",
    ] {
        let concept = document["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|concept| concept["id"] == id)
            .unwrap();
        assert_eq!(
            concept["plan"]["markets"],
            json!(["cn_equity", "us_equity"]),
            "{id}"
        );
        assert_eq!(concept["plan"]["modules"], json!(["data"]), "{id}");
        assert_eq!(
            concept["plan"]["source_policy"], "evidence_required",
            "{id}"
        );
        let (status, _) = request(
            &app,
            "/api/practice",
            json!({"concept_id": id, "module": "data", "symbol": "BTCUSDT", "source": "binance", "inputs": {}}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id} should reject crypto");
    }
    let cross_market = document["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|concept| concept["id"] == "book_log_return")
        .unwrap();
    assert_eq!(
        cross_market["plan"]["markets"],
        json!(["crypto", "cn_equity", "us_equity"])
    );
    let action = document["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|concept| concept["id"] == "book_adjustment")
        .unwrap();
    assert_eq!(
        action["plan"]["required_datasets"],
        json!(["dated_corporate_actions", "raw_market_price"])
    );
    let share_count = document["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|concept| concept["id"] == "book_share_counts")
        .unwrap();
    assert_eq!(
        share_count["plan"]["required_datasets"],
        json!(["dated_share_register", "market_price"])
    );
    let cape = document["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|concept| concept["id"] == "book_cape")
        .unwrap();
    assert_eq!(cape["plan"]["markets"], json!(["cn_equity", "us_equity"]));
    assert_eq!(
        cape["plan"]["required_datasets"],
        json!([
            "ten_annual_point_in_time_eps",
            "ten_annual_cpi",
            "market_price"
        ])
    );
    for id in ["book_etf_flows", "book_etf_balances"] {
        let etf = document["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|concept| concept["id"] == id)
            .unwrap();
        assert_eq!(etf["plan"]["markets"], json!(["us_equity"]), "{id}");
        assert_eq!(
            etf["plan"]["required_datasets"],
            json!(["dated_crypto_etf_holdings_or_flows", "etf_symbol"]),
            "{id}"
        );
    }
    let funding = document["concepts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|concept| concept["id"] == "book_funding")
        .unwrap();
    assert_eq!(funding["plan"]["markets"], json!(["crypto"]));
    assert_eq!(
        funding["plan"]["required_datasets"],
        json!(["timestamped_derivatives_or_blockchain_observations"])
    );
    for id in [
        "book_option_dte",
        "book_iv_smile",
        "book_second_order_greeks",
    ] {
        let option = document["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|concept| concept["id"] == id)
            .unwrap();
        assert_eq!(option["plan"]["markets"], json!(["us_equity"]), "{id}");
        assert_eq!(
            option["plan"]["required_datasets"],
            json!(["timestamped_option_chain", "underlying_price"]),
            "{id}"
        );
    }
}

#[tokio::test]
async fn market_practice_rejects_the_current_unfinished_binance_hour() {
    let app = app();
    let hour_start = Utc::now()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let bars = SyntheticFeed::new(8)
        .fetch_historical("BTCUSDT", hour_start - chrono::Duration::hours(1), 2)
        .unwrap();
    let (status, _) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_chart_renko","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    for (source, symbol, hours_before_now) in [("a_share", "600519", 7), ("us_stock", "AAPL", 21)] {
        let mut daily_bars = SyntheticFeed::new(8)
            .fetch_historical(
                "BTCUSDT",
                Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                2,
            )
            .unwrap();
        daily_bars[0].timestamp = Utc::now() - chrono::Duration::hours(hours_before_now + 24);
        daily_bars[1].timestamp = Utc::now() - chrono::Duration::hours(hours_before_now);
        let (status, _) = request(
            &app,
            "/api/practice",
            json!({"concept_id":"book_chart_renko","module":"data","symbol":symbol,"source":source,"bars":daily_bars,"inputs":{}}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{source} unfinished session"
        );
    }
}

#[tokio::test]
async fn logarithmic_return_uses_the_last_two_observed_closes_only() {
    let app = app();
    let mut bars = SyntheticFeed::new(91)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            3,
        )
        .unwrap();
    bars[0].close = bars[0].open;
    let expected = (bars[2].close / bars[1].close).ln();
    let (status, output) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_log_return","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{output}");
    assert_eq!(output["provenance"], "provided_market_bars");
    assert!((output["values"]["book_log_return"].as_f64().unwrap() - expected).abs() < 1e-12);
    let (status, _) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_log_return","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{"start_price":100.0,"end_price":110.0}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cdp_uses_the_previous_completed_a_share_daily_bar_only() {
    let app = app();
    let mut bars = SyntheticFeed::new(91)
        .fetch_historical(
            "600519",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            2,
        )
        .unwrap();
    bars[0].open = 100.0;
    bars[0].high = 110.0;
    bars[0].low = 90.0;
    bars[0].close = 100.0;
    bars[1].open = 122.0;
    bars[1].high = 130.0;
    bars[1].low = 120.0;
    bars[1].close = 125.0;
    bars[1].timestamp = bars[0].timestamp + chrono::Duration::days(1);
    let body = |source: &str, input: Value| json!({"concept_id":"book_cdp","module":"data","symbol":"600519","source":source,"bars":bars,"inputs":input});
    let (status, output) = request(&app, "/api/practice", body("a_share", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{output}");
    assert_eq!(output["provenance"], "provided_market_bars");
    assert_eq!(output["context"], "module_snapshot");
    assert_eq!(output["values"]["cdp"], 100.0);
    assert_eq!(output["values"]["ah"], 120.0);
    for body in [
        body("binance", json!({})),
        body("a_share", json!({"previous_high":110.0})),
    ] {
        let (status, _) = request(&app, "/api/practice", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let mut intraday = bars;
    intraday[1].timestamp = intraday[0].timestamp + chrono::Duration::hours(1);
    let (status, _) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_cdp","module":"data","symbol":"600519","source":"a_share","bars":intraday,"inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn annualized_volatility_uses_source_bound_bar_frequency_and_returns() {
    let app = app();
    let mut bars = SyntheticFeed::new(91)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            3,
        )
        .unwrap();
    for (bar, close) in bars.iter_mut().zip([100.0, 102.0, 100.0]) {
        bar.open = close;
        bar.high = close;
        bar.low = close;
        bar.close = close;
    }
    let body = |source: &str, symbol: &str, supplied: Vec<axiom::types::Bar>| json!({"concept_id":"book_annualized_volatility","module":"data","symbol":symbol,"source":source,"bars":supplied,"inputs":{}});
    let (status, crypto) = request(
        &app,
        "/api/practice",
        body("binance", "BTCUSDT", bars.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{crypto}");
    assert_eq!(crypto["input_kind"], "market_bars");
    assert_eq!(crypto["provenance"], "provided_market_bars");
    assert_eq!(crypto["values"]["periods_per_year"], 8760.0);
    assert!(
        (crypto["values"]["annualized_volatility"].as_f64().unwrap() - 2.621_309_180_996_4).abs()
            < 1e-9
    );
    assert!(crypto["notes"].as_array().unwrap().iter().any(|note| note
        .as_str()
        .is_some_and(|note| note.contains("简单收益率"))));
    let (status, _) = request(
        &app,
        "/api/practice",
        body("synthetic", "BTCUSDT", bars.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let mut daily = bars.clone();
    daily[1].timestamp = daily[0].timestamp + chrono::Duration::days(1);
    daily[2].timestamp = daily[1].timestamp + chrono::Duration::days(3);
    let (status, equity) = request(
        &app,
        "/api/practice",
        body("a_share", "600519", daily.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{equity}");
    assert_eq!(equity["values"]["periods_per_year"], 252.0);
    assert!(
        (equity["values"]["annualized_volatility"].as_f64().unwrap() - 0.444_596_936_546_081).abs()
            < 1e-9
    );
    let (status, us_equity) = request(&app, "/api/practice", body("us_stock", "AAPL", daily)).await;
    assert_eq!(status, StatusCode::OK, "{us_equity}");
    assert_eq!(us_equity["values"]["periods_per_year"], 252.0);

    let (status, short) = request(
        &app,
        "/api/practice",
        body("binance", "BTCUSDT", bars[..2].to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{short}");
    assert_eq!(short["status"], "undefined");
    assert!(short["values"]["annualized_volatility"].is_null());

    let mut gapped_hourly = bars;
    gapped_hourly[2].timestamp += chrono::Duration::hours(1);
    let intraday_equity = SyntheticFeed::new(91)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            3,
        )
        .unwrap();
    for body in [
        body("binance", "BTCUSDT", gapped_hourly),
        body("a_share", "600519", intraday_equity),
    ] {
        let (status, _) = request(&app, "/api/practice", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn rolling_24h_volume_requires_contiguous_hourly_crypto_bars() {
    let app = app();
    let bars = SyntheticFeed::new(73)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            30,
        )
        .unwrap();
    let expected: f64 = bars.iter().skip(6).map(|bar| bar.volume).sum();
    let body = |source: &str, symbol: &str, bars: Vec<axiom::types::Bar>, inputs: Value| json!({"concept_id":"book_volume_24h","module":"data","symbol":symbol,"source":source,"bars":bars,"inputs":inputs});
    let (status, out) = request(
        &app,
        "/api/practice",
        body("binance", "BTCUSDT", bars.clone(), json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["provenance"], "provided_market_bars");
    assert_eq!(out["values"]["rolling_24h_volume"], expected);
    for request_body in [
        body("a_share", "600519", bars.clone(), json!({})),
        body(
            "binance",
            "BTCUSDT",
            bars.clone(),
            json!({"hourly_volumes":[1,2]}),
        ),
        body("binance", "BTCUSDT", bars[..23].to_vec(), json!({})),
    ] {
        let (status, _) = request(&app, "/api/practice", request_body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let mut gap = bars;
    gap[29].timestamp += chrono::Duration::hours(1);
    let (status, _) = request(
        &app,
        "/api/practice",
        body("binance", "BTCUSDT", gap, json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn five_nonstandard_charts_use_the_selected_real_bar_context() {
    let app = app();
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            20,
        )
        .unwrap();
    for id in [
        "book_chart_heikin_ashi",
        "book_chart_renko",
        "book_chart_point_figure",
        "book_chart_kagi",
        "book_chart_three_line_break",
    ] {
        let (status, out) = request(
            &app,
            "/api/practice",
            json!({
                "concept_id": id, "module": "data", "symbol": "BTCUSDT",
                "source": "binance", "bars": bars, "inputs": {}
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{id}: {out}");
        assert_eq!(out["context"], "module_snapshot", "{id}");
        assert_eq!(out["provenance"], "provided_market_bars", "{id}");
        assert_eq!(out["chart"]["input"], "provided_ohlcv_bars", "{id}");
        assert_eq!(out["chart"]["source_bar_count"], bars.len(), "{id}");
    }
    for body in [
        json!({"concept_id":"book_chart_renko","module":"data","source":"binance","bars":bars,"inputs":{"prices":[10,11]}}),
        json!({"concept_id":"book_chart_heikin_ashi","module":"data","source":"binance","bars":[],"inputs":{}}),
    ] {
        let (status, _) = request(&app, "/api/practice", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn result_practice_rejects_invalid_evidence_without_falling_back_to_teaching_defaults() {
    let app = app();
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            3,
        )
        .unwrap();
    let with_result = |concept_id: &str, inputs: Value| {
        json!({
            "concept_id": concept_id, "module": "backtest", "symbol": "BTCUSDT",
            "source": "synthetic", "bars": bars, "inputs": inputs
        })
    };
    let benchmark = json!({
        "strategy_returns": [0.01, 0.03], "benchmark_returns": [0.02, -0.01],
        "periods_per_year": 365.0, "risk_free_annual": 0.0
    });
    let (status, body) = request(
        &app,
        "/api/practice",
        with_result("information_ratio", benchmark.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["provenance"], "provided_result_context");
    for inputs in [
        json!({"strategy_returns":[0.01,0.03],"benchmark_returns":[0.02],"periods_per_year":365.0,"risk_free_annual":0.0}),
        json!({"strategy_returns":[0.01],"benchmark_returns":[0.02],"periods_per_year":365.0,"risk_free_annual":0.0}),
        json!({"strategy_returns":[],"benchmark_returns":[0.02],"periods_per_year":365.0,"risk_free_annual":0.0}),
        json!({"strategy_returns":["bad",0.03],"benchmark_returns":[0.02,-0.01],"periods_per_year":365.0,"risk_free_annual":0.0}),
        json!({"strategy_returns":[0.01,0.03],"periods_per_year":365.0,"risk_free_annual":0.0}),
    ] {
        let (status, _) = request(
            &app,
            "/api/practice",
            with_result("information_ratio", inputs),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    for inputs in [
        json!({"equity":[100.0,0.0,102.0,103.0],"elapsed_days":3.0}),
        json!({"equity":[100.0,101.0,102.0,103.0,104.0],"elapsed_days":3.0}),
    ] {
        let (status, _) = request(&app, "/api/practice", with_result("total_return", inputs)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    for inputs in [
        json!({"confidence":0.95,"periods_per_year":365.0,"returns":[0.01,-1.1],"risk_free_annual":0.0}),
        json!({"confidence":0.95,"periods_per_year":365.0,"returns":["bad",0.01],"risk_free_annual":0.0}),
    ] {
        let (status, _) = request(&app, "/api/practice", with_result("sharpe", inputs)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, _) = request(&app, "/api/practice", with_result("sharpe", json!([]))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    for (source, symbol) in [("a_share", "600519"), ("us_stock", "AAPL")] {
        let (status, body) = request(
            &app,
            "/api/practice",
            json!({
                "concept_id": "eps", "module": "data", "source": source, "symbol": symbol, "inputs": {}
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["context"], "editable_teaching_inputs");
    }
}

#[tokio::test]
async fn book_r_squared_uses_verified_same_period_equity_and_market_returns() {
    let app = app();
    let bars = json!([
        {"timestamp":"2024-01-01T00:00:00Z","open":100.0,"high":100.0,"low":100.0,"close":100.0,"volume":1.0},
        {"timestamp":"2024-01-01T01:00:00Z","open":110.0,"high":110.0,"low":110.0,"close":110.0,"volume":1.0},
        {"timestamp":"2024-01-01T02:00:00Z","open":99.0,"high":99.0,"low":99.0,"close":99.0,"volume":1.0},
        {"timestamp":"2024-01-01T03:00:00Z","open":108.9,"high":108.9,"low":108.9,"close":108.9,"volume":1.0}
    ]);
    let inputs = json!({
        "initial_capital": 100.0,
        "equity": [100.0, 98.0, 105.0, 103.0, 106.09],
        "equity_points": [
            {"timestamp":"2024-01-01T00:00:00Z","equity":98.0},
            {"timestamp":"2024-01-01T01:00:00Z","equity":105.0},
            {"timestamp":"2024-01-01T02:00:00Z","equity":103.0},
            {"timestamp":"2024-01-01T03:00:00Z","equity":106.09}
        ],
        "strategy_returns": [105.0 / 98.0 - 1.0, 103.0 / 105.0 - 1.0, 0.03],
        "benchmark_returns": [0.1, -0.1, 0.1]
    });
    let body = |inputs: Value| {
        json!({
            "concept_id":"book_r_squared", "module":"backtest", "symbol":"BTCUSDT",
            "source":"binance", "bars":bars, "inputs":inputs
        })
    };
    let (status, out) = request(&app, "/api/practice", body(inputs.clone())).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["provenance"], "provided_result_context");
    assert!((out["values"]["r_squared"].as_f64().unwrap() - 0.790_826_854_342_459_4).abs() < 1e-12);

    for changed in [
        json!({"initial_capital":100.0,"equity":[100.0,98.0,105.0,103.0,106.09],"strategy_returns":[105.0/98.0-1.0,0.01,0.03],"benchmark_returns":[0.1,-0.1,0.1]}),
        json!({"initial_capital":100.0,"equity":[100.0,98.0,105.0,103.0,106.09],"strategy_returns":[105.0/98.0-1.0,103.0/105.0-1.0,0.03],"benchmark_returns":[0.1,0.0,0.1]}),
        json!({"initial_capital":100.0,"equity":[100.0,98.0,105.0,103.0],"strategy_returns":[105.0/98.0-1.0,103.0/105.0-1.0,0.03],"benchmark_returns":[0.1,-0.1,0.1]}),
    ] {
        let (status, _) = request(&app, "/api/practice", body(changed)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    for points in [
        json!([
            {"timestamp":"2024-01-01T01:00:00Z","equity":98.0},
            {"timestamp":"2024-01-01T00:00:00Z","equity":105.0},
            {"timestamp":"2024-01-01T02:00:00Z","equity":103.0},
            {"timestamp":"2024-01-01T03:00:00Z","equity":106.09}
        ]),
        json!([
            {"timestamp":"2024-01-01T00:00:00Z","equity":98.0},
            {"timestamp":"2024-01-01T01:00:00Z","equity":105.0},
            {"timestamp":"2024-01-01T02:00:00Z","equity":103.0}
        ]),
    ] {
        let mut changed = inputs.clone();
        changed["equity_points"] = points;
        let (status, _) = request(&app, "/api/practice", body(changed)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn relative_volume_at_time_uses_only_matching_utc_hour_prefixes() {
    use axiom::types::Bar;
    let app = app();
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let bars: Vec<Bar> = (0..8 * 24)
        .map(|index| Bar {
            timestamp: start + chrono::Duration::hours(index),
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: (index / 24 + 1) as f64,
        })
        .collect();
    let body = |bars: Vec<Bar>| json!({"concept_id":"book_relative_volume_at_time","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":{}});
    let (status, out) = request(&app, "/api/practice", body(bars.clone())).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["provenance"], "provided_market_bars");
    assert_eq!(out["values"]["historical_sample_count"], 7.0);
    assert_eq!(out["values"]["relative_volume_at_time"], 2.0);
    assert!(out["notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |note| note.as_str().is_some_and(|text| text.contains("2024-01-01")
                && text.contains("2024-01-07")
                && text.contains("2024-01-09"))
        ));
    let mut older_gap = vec![
        Bar {
            timestamp: start - chrono::Duration::hours(10),
            ..bars[0]
        },
        Bar {
            timestamp: start - chrono::Duration::hours(8),
            ..bars[0]
        },
    ];
    older_gap.extend(bars.clone());
    let (status, out) = request(&app, "/api/practice", body(older_gap)).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["values"]["relative_volume_at_time"], 2.0);
    let mut no_history_volume = bars.clone();
    for bar in &mut no_history_volume[..7 * 24] {
        bar.volume = 0.0;
    }
    let (status, undefined) = request(&app, "/api/practice", body(no_history_volume)).await;
    assert_eq!(status, StatusCode::OK, "{undefined}");
    assert_eq!(undefined["status"], "undefined");
    assert!(undefined["values"]["relative_volume_at_time"].is_null());
    assert!(undefined["reason"].as_str().unwrap().contains("均值为零"));
    let mut manual = body(bars.clone());
    manual["inputs"] = json!({"current_cumulative_volume": 999.0});
    let (status, _) = request(&app, "/api/practice", manual).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut wrong_market = body(bars.clone());
    wrong_market["source"] = json!("a_share");
    wrong_market["symbol"] = json!("600519");
    let (status, _) = request(&app, "/api/practice", wrong_market).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut missing = bars;
    missing.remove(24 * 3 + 5);
    let (status, _) = request(&app, "/api/practice", body(missing)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn nonstandard_bar_compares_observed_ohlc4_with_actual_close_without_manual_prices() {
    use axiom::types::Bar;
    let app = app();
    let bars = vec![Bar {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        open: 100.0,
        high: 109.0,
        low: 99.0,
        close: 106.0,
        volume: 10.0,
    }];
    let body = |inputs: Value| json!({"concept_id":"book_nonstandard_bar","module":"data","symbol":"BTCUSDT","source":"binance","bars":bars,"inputs":inputs});
    let (status, output) = request(&app, "/api/practice", body(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{output}");
    assert_eq!(output["provenance"], "provided_market_bars");
    assert_eq!(output["values"]["book_nonstandard_bar"], 103.5);
    assert_eq!(output["units"]["book_nonstandard_bar"], "price");
    assert_eq!(output["values"]["actual_close"], 106.0);
    assert_eq!(output["values"]["synthetic_minus_close"], -2.5);
    assert!(output["notes"].as_array().unwrap().iter().any(|note| note
        .as_str()
        .is_some_and(|text| text.contains("不可作为成交价"))));
    let (status, _) = request(&app, "/api/practice", body(json!({"open":100.0}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn book_period_requires_real_source_bound_bars_and_preserves_stock_calendar_gaps() {
    let app = app();
    let crypto = json!([
        {"timestamp":"2024-01-01T00:00:00Z","open":100.0,"high":101.0,"low":99.0,"close":100.0,"volume":1.0},
        {"timestamp":"2024-01-01T01:00:00Z","open":100.0,"high":102.0,"low":99.0,"close":101.0,"volume":1.0}
    ]);
    let stock = json!([
        {"timestamp":"2024-01-05T00:00:00Z","open":100.0,"high":101.0,"low":99.0,"close":100.0,"volume":1.0},
        {"timestamp":"2024-01-08T00:00:00Z","open":100.0,"high":102.0,"low":99.0,"close":101.0,"volume":1.0}
    ]);
    for (source, symbol, bars, expected) in [
        ("binance", "BTCUSDT", crypto.clone(), 3_600),
        ("a_share", "600519", stock.clone(), 86_400),
        ("us_stock", "AAPL", stock.clone(), 86_400),
    ] {
        let (status, output) = request(&app, "/api/practice", json!({
            "concept_id":"book_period","module":"data","source":source,"symbol":symbol,"bars":bars,"inputs":{}
        })).await;
        assert_eq!(status, StatusCode::OK, "{source}: {output}");
        assert_eq!(output["provenance"], "provided_market_bars");
        assert_eq!(output["values"]["book_period"].as_i64(), Some(expected));
        if source != "binance" {
            assert_eq!(
                output["values"]["last_observed_interval_seconds"].as_i64(),
                Some(259_200)
            );
            assert!(output["notes"].to_string().contains("不表示多日K线"));
        }
    }
    for request_body in [
        json!({"concept_id":"book_period","module":"data","source":"synthetic","symbol":"BTCUSDT","bars":crypto,"inputs":{}}),
        json!({"concept_id":"book_period","module":"data","source":"binance","symbol":"BTCUSDT","bars":stock,"inputs":{"period_seconds":300}}),
        json!({"concept_id":"book_period","module":"backtest","source":"binance","symbol":"BTCUSDT","bars":stock,"inputs":{}}),
        json!({"concept_id":"book_period","module":"data","source":"binance","symbol":"BTCUSDT","bars":[{"timestamp":"2024-01-01T00:30:00Z","open":100.0,"high":101.0,"low":99.0,"close":100.0,"volume":1.0}],"inputs":{}}),
    ] {
        let (status, _) = request(&app, "/api/practice", request_body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn rolling_correlation_uses_only_verified_current_result_returns_across_markets() {
    let app = app();
    let strategy = vec![105.0 / 98.0 - 1.0, 103.0 / 105.0 - 1.0, 0.03];
    let benchmark = vec![0.1, -0.1, 0.1];

    for (source, symbol, timestamps) in [
        (
            "binance",
            "BTCUSDT",
            vec![
                "2024-01-01T00:00:00Z",
                "2024-01-01T01:00:00Z",
                "2024-01-01T02:00:00Z",
                "2024-01-01T03:00:00Z",
            ],
        ),
        (
            "a_share",
            "600519",
            vec![
                "2024-01-01T00:00:00Z",
                "2024-01-02T00:00:00Z",
                "2024-01-03T00:00:00Z",
                "2024-01-04T00:00:00Z",
            ],
        ),
        (
            "us_stock",
            "AAPL",
            vec![
                "2024-01-01T00:00:00Z",
                "2024-01-02T00:00:00Z",
                "2024-01-03T00:00:00Z",
                "2024-01-04T00:00:00Z",
            ],
        ),
    ] {
        let closes = [100.0, 110.0, 99.0, 108.9];
        let bars = Value::Array(
            timestamps
                .iter()
                .zip(closes)
                .map(|(timestamp, close)| {
                    json!({"timestamp":timestamp,"open":close,"high":close,"low":close,"close":close,"volume":1.0})
                })
                .collect(),
        );
        let inputs = json!({
            "period":2,
            "initial_capital":100.0,
            "equity":[100.0,98.0,105.0,103.0,106.09],
            "equity_points":timestamps.iter().zip([98.0,105.0,103.0,106.09]).map(|(timestamp,equity)| json!({"timestamp":timestamp,"equity":equity})).collect::<Vec<_>>(),
            "strategy_returns":strategy,
            "benchmark_returns":benchmark,
            "series_x":strategy,
            "series_y":benchmark
        });
        let body = |inputs: Value| {
            json!({
                "concept_id":"rolling_correlation","module":"backtest","symbol":symbol,
                "source":source,"bars":bars,"inputs":inputs
            })
        };
        let (status, out) = request(&app, "/api/practice", body(inputs.clone())).await;
        assert_eq!(status, StatusCode::OK, "{source}: {out}");
        assert_eq!(out["provenance"], "provided_result_context");
        let values = out["series"][0]["values"].as_array().unwrap();
        assert!(values[0].is_null(), "{out}");
        assert_eq!(values.len(), strategy.len());
        assert!((values[1].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((values[2].as_f64().unwrap() - 1.0).abs() < 1e-12);

        for (key, value) in [
            ("series_x", json!([0.0, 0.0, 0.0])),
            ("series_y", json!([0.0, 0.0, 0.0])),
            ("period", json!(1)),
            ("period", json!(4)),
        ] {
            let mut changed = inputs.clone();
            changed[key] = value;
            let (status, _) = request(&app, "/api/practice", body(changed)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{source} {key}");
        }
        let mut missing = inputs.clone();
        missing.as_object_mut().unwrap().remove("series_x");
        let (status, _) = request(&app, "/api/practice", body(missing)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{source}");
    }
}
