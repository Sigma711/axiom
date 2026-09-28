use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    industry_case::{IndustrySourceConfig, IndustrySourceRegistry},
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

async fn post(app: &Router, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/practice")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 10_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes))),
    )
}

async fn fixture_app() -> (Router, tokio::task::JoinHandle<()>, PathBuf) {
    static PDF: &[u8] = b"%PDF-1.4\nverified industry disclosure fixture\n%%EOF\n";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/ping-an.pdf", get(|| async { PDF }))
                .route("/shopify.pdf", get(|| async { PDF }))
                .route("/ebay.pdf", get(|| async { PDF }))
                .route("/realty.pdf", get(|| async { PDF })),
        )
        .await
        .unwrap()
    });
    let source = |path: &str| IndustrySourceConfig {
        url: format!("http://{address}/{path}"),
        sha256: format!("{:x}", Sha256::digest(PDF)),
        bytes: PDF.len(),
    };
    let registry = IndustrySourceRegistry {
        ping_an: source("ping-an.pdf"),
        shopify: source("shopify.pdf"),
        ebay: source("ebay.pdf"),
        realty_income: source("realty.pdf"),
    };
    let cache = std::env::temp_dir().join(format!("axiom-industry-api-{}", uuid::Uuid::new_v4()));
    let state = AppState::new(default_config(), cache.clone()).with_industry_sources(registry);
    (api::router(Arc::new(state)), server, cache)
}

#[tokio::test]
async fn industry_api_verifies_each_fixed_issuer_disclosure() {
    let (app, server, cache) = fixture_app().await;
    for (concept, symbol) in [
        ("book_bank_nim", "2318.HK"),
        ("book_saas_arr", "SHOP"),
        ("book_platform_take_rate", "EBAY"),
        ("book_reit_occupancy", "O"),
    ] {
        let (status, result) = post(
            &app,
            json!({"concept_id":concept,"module":"data","source":"issuer_disclosure","symbol":symbol,"inputs":{}}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{concept}: {result}");
        assert_eq!(result["input_kind"], "industry_case", "{concept}");
        assert_eq!(
            result["provenance"], "verified_original_issuer_disclosure",
            "{concept}"
        );
        assert_eq!(result["context"], "historical_industry_disclosure");
        assert_eq!(result["bar_origin"], "server_verified_issuer_filing_pdf");
        assert_eq!(result["source"], "issuer_disclosure");
        assert_eq!(result["symbol"], symbol);
        assert!(result["industry_case"]["verification"]["status"]
            .as_str()
            .is_some_and(|status| status.starts_with("verified")));
        assert!(result["industry_facts"]
            .as_object()
            .is_some_and(|v| !v.is_empty()));
    }
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}

#[tokio::test]
async fn industry_api_rejects_every_caller_controlled_evidence_seam() {
    let (app, server, cache) = fixture_app().await;
    let base = json!({"concept_id":"book_bank_nim","module":"data","source":"issuer_disclosure","symbol":"2318.HK","inputs":{}});
    for patch in [
        json!({"symbol":"SHOP"}),
        json!({"source":"us_stock"}),
        json!({"inputs":{"nim":0.1}}),
        json!({"bars":[]}),
        json!({"limit":1}),
        json!({"second_symbol":"AAPL"}),
    ] {
        let mut body = base.clone();
        body.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert_eq!(post(&app, body).await.0, StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        post(
            &app,
            json!({"concept_id":"book_platform_take_rate","module":"data","source":"issuer_disclosure","symbol":"SHOP","inputs":{}}),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST,
        "Shopify GMV must not be reused as the fixed eBay take-rate case"
    );
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}
