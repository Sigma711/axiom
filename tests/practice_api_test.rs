use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    data::{DataFeed, SyntheticFeed},
    practice,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
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
    for concept in practice::catalog()
        .into_iter()
        .filter(|concept| concept.input_kind == "market_bars")
    {
        let mut first: Option<Value> = None;
        for module in ["data"] {
            let(status,out)=request(&app,"/api/practice",json!({"concept_id":concept.id,"module":module,"symbol":"BTCUSDT","source":"synthetic","bars":bars,"inputs":{}})).await;
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
