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
use chrono::{TimeZone, Timelike, Utc};
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
        json!({"concept_id":"book_log_return","module":"data","symbol":"BTCUSDT","source":"synthetic","bars":bars,"inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{output}");
    assert_eq!(output["provenance"], "provided_market_bars");
    assert!((output["values"]["book_log_return"].as_f64().unwrap() - expected).abs() < 1e-12);
    let (status, _) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"book_log_return","module":"data","symbol":"BTCUSDT","source":"synthetic","bars":bars,"inputs":{"start_price":100.0,"end_price":110.0}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
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
        body("synthetic", "BTCUSDT", bars.clone(), json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["provenance"], "provided_market_bars");
    assert_eq!(out["values"]["rolling_24h_volume"], expected);
    for request_body in [
        body("a_share", "600519", bars.clone(), json!({})),
        body(
            "synthetic",
            "BTCUSDT",
            bars.clone(),
            json!({"hourly_volumes":[1,2]}),
        ),
        body("synthetic", "BTCUSDT", bars[..23].to_vec(), json!({})),
    ] {
        let (status, _) = request(&app, "/api/practice", request_body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let mut gap = bars;
    gap[29].timestamp += chrono::Duration::hours(1);
    let (status, _) = request(
        &app,
        "/api/practice",
        body("synthetic", "BTCUSDT", gap, json!({})),
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
                "source": "synthetic", "bars": bars, "inputs": {}
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
        json!({"concept_id":"book_chart_renko","module":"data","source":"synthetic","bars":bars,"inputs":{"prices":[10,11]}}),
        json!({"concept_id":"book_chart_heikin_ashi","module":"data","source":"synthetic","bars":[],"inputs":{}}),
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
