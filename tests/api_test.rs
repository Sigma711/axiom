//! Exercise the actual axum router without external market dependencies.
use axiom::{api, app_state::AppState, config::default_config};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

fn app() -> axum::Router {
    api::router(Arc::new(AppState::new(
        default_config(),
        PathBuf::from("target/test-data"),
    )))
}

async fn request(method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 20_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes))),
    )
}

#[tokio::test]
async fn malformed_market_requests_are_rejected_before_fetching() {
    for path in [
        "/api/data?source=unknown",
        "/api/data?source=synthetic&limit=0",
        "/api/data?source=synthetic&limit=999999",
        "/api/data?source=synthetic&symbol=..%2Fsecret",
        "/api/indicators?source=synthetic&indicators=rsi_0",
        "/api/indicators?source=synthetic&indicators=unknown",
        "/api/indicators?source=synthetic&indicators=ema_-1",
        "/api/patterns?source=synthetic&limit=0",
    ] {
        let (status, body) = request("GET", path, Value::Null).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
    }
}

#[tokio::test]
async fn an_empty_indicator_selection_returns_a_price_only_chart() {
    let (status, body) = request(
        "GET",
        "/api/indicators?source=synthetic&limit=60&indicators=",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["bars"].as_array().unwrap().len(), 60);
    assert_eq!(body["indicators"].as_object().unwrap().len(), 0);
}

#[tokio::test]
async fn indicator_identifiers_with_underscores_preserve_their_meaning() {
    let (status, body) = request("GET", "/api/indicators?source=synthetic&limit=60&indicators=williams_r_14,atr_14,atr_percent_14,z_score_20,alligator", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let indicators = &body["indicators"];
    for id in [
        "williams_r_14",
        "atr_14",
        "atr_percent_14",
        "z_score_20",
        "alligator_jaw",
    ] {
        assert_eq!(indicators[id].as_array().unwrap().len(), 60, "{id}");
    }
    let i = 59;
    let wr = indicators["williams_r_14"][i]["y"].as_f64().unwrap();
    assert!((-100.0..=0.0).contains(&wr));
    let atr = indicators["atr_14"][i]["y"].as_f64().unwrap();
    let pct = indicators["atr_percent_14"][i]["y"].as_f64().unwrap();
    let close = body["bars"][i]["close"].as_f64().unwrap();
    assert!((pct - atr / close * 100.0).abs() < 1e-10);
    assert!(
        indicators["alligator_jaw"][19].is_null(),
        "13-period SMMA shifted 8 bars needs 20 warm-up slots"
    );
}

#[tokio::test]
async fn every_published_strategy_runs_through_the_public_backtest_api() {
    let (status, catalog) = request("GET", "/api/strategies", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{catalog}");
    let strategies = catalog["strategies"]
        .as_array()
        .expect("strategy catalog array");
    assert!(!strategies.is_empty());

    for strategy in strategies {
        let name = strategy["name"].as_str().expect("strategy name");
        let params = strategy["params"]
            .as_array()
            .expect("strategy parameter array")
            .iter()
            .map(|parameter| {
                (
                    parameter["key"].as_str().expect("parameter key").to_owned(),
                    parameter["default"].clone(),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let (status, result) = request(
            "POST",
            "/api/backtest",
            json!({
                "strategy": name,
                "source": "synthetic",
                "limit": 200,
                "initial_capital": 10_000.0,
                "params": params,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{name}: {result}");
        assert_eq!(result["config"]["strategy"], name);
        assert_eq!(result["equity_curve"].as_array().unwrap().len(), 200);
        assert!(!result["metrics"].as_object().unwrap().is_empty());
    }
}

#[tokio::test]
async fn backtest_rejects_invalid_configuration_instead_of_panicking_or_clamping() {
    let cases = [
        json!({"initial_capital":0}),
        json!({"commission_rate":-0.1}),
        json!({"slippage_rate":1.0}),
        json!({"max_position_pct":1.1}),
        json!({"stop_loss_pct":-0.1}),
        json!({"limit":0}),
        json!({"params":{"fast":0,"slow":20}}),
        json!({"params":{"fast":2.5,"slow":20}}),
        json!({"params":{"fast":20,"slow":5}}),
        json!({"params":{"typo":12}}),
    ];
    for patch in cases {
        let mut req = json!({"strategy":"sma_cross","source":"synthetic","limit":50});
        req.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let (status, response) = request("POST", "/api/backtest", req.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{req}: {response}");
    }
}

#[tokio::test]
async fn compare_can_reuse_exactly_the_same_bars_and_initial_capital() {
    let (_, data) = request("GET", "/api/data?source=synthetic&limit=100", Value::Null).await;
    for strategy in [
        "buy_and_hold",
        "sma_cross",
        "rsi",
        "random",
        "macd",
        "bollinger",
        "supertrend",
        "donchian_breakout",
        "vwap_reversion",
        "kdj",
        "ichimoku",
        "ppo",
        "vortex",
        "elder_ray",
    ] {
        let (status, result) = request(
            "POST",
            "/api/backtest",
            json!({
                "strategy":strategy,"source":"synthetic","bars":data["bars"],"initial_capital":10000
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{strategy}: {result}");
        assert_eq!(
            result["bars"], data["bars"],
            "{strategy} changed the comparison dataset"
        );
        assert_eq!(result["metrics"]["初始资金"], 10000.0);
        assert_eq!(result["equity_curve"].as_array().unwrap().len(), 100);
    }
}

#[tokio::test]
async fn supplied_bars_must_be_finite_valid_and_strictly_chronological() {
    let good = json!({"timestamp":"2024-01-01T00:00:00Z","open":100,"high":110,"low":90,"close":105,"volume":1});
    let mut invalid = good.clone();
    invalid["high"] = json!(99);
    for bars in [
        json!([]),
        json!([good.clone(), good.clone()]),
        json!([invalid]),
    ] {
        let (status, _) = request(
            "POST",
            "/api/backtest",
            json!({"strategy":"buy_and_hold","source":"synthetic","bars":bars}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn paper_controls_persist_and_reject_unknown_strategy() {
    let router = app();
    for (path, running) in [("/api/paper/start", true), ("/api/paper/stop", false)] {
        let response = router
            .clone()
            .oneshot(Request::post(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let snapshot = router
            .clone()
            .oneshot(
                Request::get("/api/paper/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let snapshot: Value =
            serde_json::from_slice(&to_bytes(snapshot.into_body(), 1_000_000).await.unwrap())
                .unwrap();
        assert_eq!(snapshot["is_running"], running);
    }
    let (status, _) = request("POST", "/api/paper/strategy", json!({"strategy":"typo"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn all_supported_indicator_overlays_have_full_length_public_series() {
    let ids = [
        "sma_20",
        "ema_20",
        "rsi_14",
        "vwma_20",
        "bbands_20",
        "macd",
        "vwap",
        "atr_14",
        "atr_percent_14",
        "obv",
        "zscore_20",
        "ichimoku",
        "kdj",
        "stoch_14",
        "williams_r_14",
        "cci_20",
        "adx_14",
        "bbi",
        "alligator",
        "ppo",
        "vortex_14",
    ];
    let path = format!(
        "/api/indicators?source=synthetic&limit=120&indicators={}",
        ids.join(",")
    );
    let (status, body) = request("GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["bars"].as_array().unwrap().len(), 120);

    let overlays = body["indicators"].as_object().expect("indicator object");
    for key in [
        "sma_20",
        "ema_20",
        "rsi_14",
        "vwma_20",
        "bbands_20_upper",
        "bbands_20_middle",
        "bbands_20_lower",
        "macd_dif",
        "macd_dea",
        "macd_hist",
        "vwap",
        "atr_14",
        "atr_percent_14",
        "obv",
        "zscore_20",
        "ichimoku_tenkan",
        "ichimoku_kijun",
        "ichimoku_senkou_a",
        "ichimoku_senkou_b",
        "ichimoku_chikou",
        "kdj_k",
        "kdj_d",
        "kdj_j",
        "stoch_k",
        "stoch_d",
        "williams_r_14",
        "cci_20",
        "adx_plus_di",
        "adx_minus_di",
        "adx_adx",
        "bbi",
        "alligator_jaw",
        "alligator_teeth",
        "alligator_lips",
        "ppo",
        "vortex_plus",
        "vortex_minus",
    ] {
        let points = overlays[key].as_array().expect(key);
        assert_eq!(points.len(), 120, "{key}");
        assert!(
            points
                .iter()
                .filter(|point| !point.is_null())
                .all(|point| point["x"].is_string() && point["y"].is_number()),
            "{key} contains a malformed finite point"
        );
    }
}

#[tokio::test]
async fn public_learning_and_exploration_reads_return_complete_safe_documents() {
    let (status, config) = request("GET", "/api/config", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(config["trading"]["symbol"], "BTCUSDT");

    let (status, data) = request("GET", "/api/data?source=synthetic&limit=25", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{data}");
    assert_eq!(data["bars"].as_array().unwrap().len(), 25);

    let (status, patterns) = request(
        "GET",
        "/api/patterns?source=synthetic&limit=25",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{patterns}");
    assert_eq!(patterns["patterns"].as_array().unwrap().len(), 20);
    assert!(patterns["patterns"][0]["pattern_code"].is_string());

    let (status, heikin_ashi) = request(
        "GET",
        "/api/heikin_ashi?source=synthetic&limit=25",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{heikin_ashi}");
    assert_eq!(heikin_ashi["chart"], "heikin_ashi");
    assert_eq!(heikin_ashi["bars"].as_array().unwrap().len(), 25);

    let (status, knowledge) = request("GET", "/api/knowledge", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{knowledge}");
    assert!(knowledge["total"].as_u64().unwrap() > 100);
    assert!(!knowledge["categories"].as_object().unwrap().is_empty());

    let (status, location) = request(
        "GET",
        "/api/code_loc?ref=src%2Findicators%2Fma.rs%3A%3Asma",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(location["ok"], true);
    assert!(location["line"].as_u64().unwrap() > 0);
    assert!(!location["url"].as_str().unwrap().is_empty());

    let (status, missing) = request("GET", "/api/code_loc?ref=not-real", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(missing["ok"], false);

    let router = app();
    let source = router
        .clone()
        .oneshot(
            Request::get("/api/code/source?path=src%2Findicators%2Fma.rs&line=1&end_line=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source.status(), StatusCode::OK);
    let source_html = String::from_utf8(
        to_bytes(source.into_body(), 2_000_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(source_html.contains("id=\"L1\""));
    assert!(source_html.contains("固定版本源码快照") || source_html.contains("含本地改动"));

    let denied = router
        .oneshot(
            Request::get("/api/code/source?path=..%2FCargo.toml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}
