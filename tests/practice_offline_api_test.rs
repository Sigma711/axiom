//! Offline mode must fail closed rather than fetch providers or fabricate facts.
//! Kept in a separate integration-test process to isolate its environment setting.
use axiom::{api, app_state::AppState, config::default_config};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn offline_practices_never_replace_missing_original_sources_with_teaching_data() {
    std::env::set_var("AXIOM_OFFLINE", "1");
    let root = std::env::temp_dir().join(format!("axiom-offline-api-{}", uuid::Uuid::new_v4()));
    let app = api::router(Arc::new(AppState::new(default_config(), root.clone())));
    for (id, source, symbol) in [
        ("rsi", "binance", "BTCUSDT"),
        ("book_adjustment", "us_stock", "AAPL"),
        ("eps", "us_stock", "AAPL"),
        ("altman_z", "issuer_disclosure", "AAPL"),
        ("book_transaction_fees", "binance", "BTCUSDT"),
        ("book_utxo_counts", "binance", "BTCUSDT"),
        ("book_sending_receiving", "binance", "BTCUSDT"),
        ("book_block_height", "binance", "BTCUSDT"),
        ("book_net_volume", "binance", "BTCUSDT"),
    ] {
        let response = app.clone().oneshot(Request::post("/api/practice")
            .header("content-type", "application/json")
            .body(Body::from(json!({"concept_id":id,"module":"data","source":source,"symbol":symbol,"inputs":{}}).to_string())).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{id}: {text}");
        assert!(text.contains("offline mode"), "{id}: {text}");
        assert!(!text.contains("\"values\""), "{id}: {text}");
    }
    let response = app
        .oneshot(
            Request::get("/api/symbols?source=binance")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(result["status"], "offline");
    std::env::remove_var("AXIOM_OFFLINE");
    let _ = tokio::fs::remove_dir_all(root).await;
}
