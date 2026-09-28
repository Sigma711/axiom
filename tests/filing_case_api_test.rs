use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    filing_case::{FilingSourceConfig, SUPPORTED_IDS},
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

async fn fixture_app(bytes: &'static [u8]) -> (Router, tokio::task::JoinHandle<()>, PathBuf) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/filing.pdf", get(move || async move { bytes })),
        )
        .await
        .unwrap()
    });
    let cache = std::env::temp_dir().join(format!("axiom-filing-api-{}", uuid::Uuid::new_v4()));
    let source = FilingSourceConfig {
        url: format!("http://{address}/filing.pdf"),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        bytes: bytes.len(),
    };
    let state = AppState::new(default_config(), cache.clone()).with_filing_source(source);
    (api::router(Arc::new(state)), server, cache)
}

#[tokio::test]
async fn catalog_and_all_filing_concepts_use_one_verified_issuer_case() {
    static PDF: &[u8] = b"%PDF-1.4\nhermetic checked filing fixture\n%%EOF\n";
    let (app, server, cache) = fixture_app(PDF).await;
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    let response = app
        .clone()
        .oneshot(Request::get("/api/knowledge").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let knowledge: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    assert_eq!(SUPPORTED_IDS.len(), 28);
    for id in SUPPORTED_IDS {
        let concept = catalog["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|concept| concept["id"] == *id)
            .unwrap();
        assert_eq!(concept["input_kind"], "filing_case", "{id}");
        assert_eq!(concept["inputs"], json!([]), "{id}");
        assert_eq!(concept["plan"]["markets"], json!(["us_equity"]), "{id}");
        assert_eq!(concept["plan"]["modules"], json!(["data"]), "{id}");
        assert_eq!(concept["plan"]["source_policy"], "real_required", "{id}");
        let entry = knowledge["categories"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|entries| entries.as_array().unwrap())
            .find(|entry| entry["id"] == *id)
            .unwrap();
        assert!(
            entry["signals"]
                .as_str()
                .unwrap()
                .contains("Apple FY2025/FY2024"),
            "{id}"
        );
        assert!(!entry["formula"].as_str().unwrap().is_empty(), "{id}");
        assert!(!entry["summary"].as_str().unwrap().is_empty(), "{id}");

        let (status, result) = post(
            &app,
            json!({"concept_id":id,"module":"data","symbol":"AAPL","source":"us_stock","inputs":{}}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{id}: {result}");
        assert_eq!(result["input_kind"], "filing_case", "{id}");
        assert_eq!(result["provenance"], "verified_issuer_filing_case", "{id}");
        assert_eq!(result["context"], "historical_filing_case", "{id}");
        assert_eq!(result["filing_case"]["issuer"], "Apple Inc.", "{id}");
        assert_eq!(result["filing_case"]["audited"], false, "{id}");
        assert_eq!(result["filing_case"]["published"], "2025-10-30", "{id}");
        assert_eq!(
            result["facts"]["annual_income_statement"]["2025"]["sales"], 416161,
            "{id}"
        );
        assert!(
            result["values"]
                .as_object()
                .unwrap()
                .values()
                .all(|value| value.as_f64().is_some_and(f64::is_finite)),
            "{id}"
        );
    }
    assert_eq!(
        post(&app, json!({"concept_id":"book_dpo","module":"data","symbol":"AAPL","source":"us_stock","inputs":{}})).await.1["values"]["days_payable_outstanding"],
        json!(((68960.0 + 69860.0) / 2.0) / 220960.0 * 364.0)
    );
    let diluted = post(&app, json!({"concept_id":"book_diluted_shares","module":"data","symbol":"AAPL","source":"us_stock","inputs":{}})).await.1;
    assert_eq!(
        diluted["values"]["weighted_diluted_shares"].as_f64(),
        Some(15004697.0)
    );
    assert_eq!(diluted["values"]["reported_diluted_eps"], 7.46);
    for (id, key, expected) in [
        ("accrual_ratio", "accrual_ratio", 0.00145811844726955),
        ("book_yoy", "revenue_year_over_year", 0.06425511782832749),
        ("book_ccc", "cash_conversion_cycle", -71.62500817717944),
    ] {
        let result = post(&app, json!({"concept_id":id,"module":"data","symbol":"AAPL","source":"us_stock","inputs":{}})).await.1;
        let actual = result["values"][key].as_f64().unwrap();
        assert!((actual - expected).abs() < 1e-12, "{id}: {actual}");
    }
    let hash = format!("{:x}", Sha256::digest(PDF));
    let cached = cache.join("filing-sources").join(format!("{hash}.pdf"));
    tokio::fs::write(&cached, b"corrupt partial cache")
        .await
        .unwrap();
    let (status, recovered) = post(
        &app,
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{recovered}");
    assert_eq!(
        recovered["filing_case"]["verification"]["cache_status"],
        "verified_then_cached"
    );
    let recovered_bytes = tokio::fs::read(&cached).await.unwrap();
    assert_eq!(recovered_bytes.as_slice(), PDF);
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}

#[tokio::test]
async fn filing_case_rejects_caller_data_and_source_hash_mismatch() {
    static PDF: &[u8] = b"%PDF-1.4\nwrong source\n%%EOF\n";
    let (app, server, cache) = fixture_app(PDF).await;
    for body in [
        json!({"concept_id":"eps","module":"data","symbol":"MSFT","source":"us_stock","inputs":{}}),
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"binance","inputs":{}}),
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{"net_income":1}}),
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{},"limit":1}),
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{},"bars":[]}),
    ] {
        assert_eq!(post(&app, body).await.0, StatusCode::BAD_REQUEST);
    }
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let bad_server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/filing.pdf", get(|| async { PDF }))
                .route(
                    "/stream.pdf",
                    get(|| async {
                        axum::response::Response::new(Body::from_stream(futures::stream::iter([
                            Ok::<_, std::io::Error>(&PDF[..10]),
                            Ok::<_, std::io::Error>(&PDF[10..]),
                        ])))
                    }),
                ),
        )
        .await
        .unwrap()
    });
    let bad_cache = std::env::temp_dir().join(format!("axiom-filing-bad-{}", uuid::Uuid::new_v4()));
    let state =
        AppState::new(default_config(), bad_cache.clone()).with_filing_source(FilingSourceConfig {
            url: format!("http://{address}/filing.pdf"),
            sha256: "0".repeat(64),
            bytes: PDF.len() + 1,
        });
    let bad_app = api::router(Arc::new(state));
    let (status, body) = post(
        &bad_app,
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(body.to_string().contains("Content-Length differs"));

    let hash_cache =
        std::env::temp_dir().join(format!("axiom-filing-hash-{}", uuid::Uuid::new_v4()));
    let hash_app = api::router(Arc::new(
        AppState::new(default_config(), hash_cache.clone()).with_filing_source(
            FilingSourceConfig {
                url: format!("http://{address}/filing.pdf"),
                sha256: "0".repeat(64),
                bytes: PDF.len(),
            },
        ),
    ));
    let (status, body) = post(
        &hash_app,
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(body.to_string().contains("different SHA-256"));

    let size_cache =
        std::env::temp_dir().join(format!("axiom-filing-size-{}", uuid::Uuid::new_v4()));
    let size_app = api::router(Arc::new(
        AppState::new(default_config(), size_cache.clone()).with_filing_source(
            FilingSourceConfig {
                url: format!("http://{address}/stream.pdf"),
                sha256: "0".repeat(64),
                bytes: PDF.len() - 1,
            },
        ),
    ));
    let (status, body) = post(
        &size_app,
        json!({"concept_id":"eps","module":"data","symbol":"AAPL","source":"us_stock","inputs":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(body
        .to_string()
        .contains("exceeds the checked document size"));
    bad_server.abort();
    let _ = tokio::fs::remove_dir_all(bad_cache).await;
    let _ = tokio::fs::remove_dir_all(hash_cache).await;
    let _ = tokio::fs::remove_dir_all(size_cache).await;
}
