use axiom::{api, app_state::AppState, config::default_config};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

async fn post_backtest(body: Value) -> (StatusCode, String) {
    let app = api::router(Arc::new(AppState::new(
        default_config(),
        PathBuf::from("target/stock-split-coverage-test"),
    )));
    let response = app
        .oneshot(
            Request::post("/api/backtest")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 2_000_000).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn bars() -> Value {
    json!([
        {"timestamp":"2026-09-01T01:00:00Z","open":100,"high":101,"low":99,"close":100,"volume":1000},
        {"timestamp":"2026-09-02T01:00:00Z","open":100,"high":101,"low":99,"close":100,"volume":1000}
    ])
}

#[tokio::test]
async fn caller_provided_stock_bars_remain_available_for_exact_dataset_comparison() {
    for (source, symbol) in [("a_share", "600000"), ("us_stock", "AAPL")] {
        let (status, body) = post_backtest(json!({
            "strategy":"buy_and_hold",
            "source":source,
            "symbol":symbol,
            "bars":bars(),
            "initial_capital":50_000
        }))
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let result: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(result["equity_curve"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["market_provenance"]["provider"],
            "caller_provided_unverified"
        );
        assert_eq!(
            result["market_provenance"]["stock_split_coverage"]["status"],
            "unverified"
        );
    }
}

#[tokio::test]
async fn split_guard_does_not_block_non_stock_backtests() {
    let (status, body) = post_backtest(json!({
        "strategy":"buy_and_hold",
        "source":"synthetic",
        "bars":bars(),
        "initial_capital":50_000
    }))
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let result: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(result["equity_curve"].as_array().unwrap().len(), 2);
}
