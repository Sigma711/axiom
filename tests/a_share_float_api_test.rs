use axiom::{
    a_share_float::{AShareFloatSourceRegistry, VerifiedDocumentSource},
    api,
    app_state::AppState,
    config::default_config,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Json, Router,
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
    let bytes = to_bytes(response.into_body(), 2_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes))),
    )
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 2_000_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn fixture_app() -> (Router, tokio::task::JoinHandle<()>, PathBuf) {
    static PDF: &[u8] = b"%PDF-1.4\nverified A-share float fixture\n%%EOF\n";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, Router::new()
            .route("/annual.pdf", get(|| async { PDF }))
            .route("/concert.pdf", get(|| async { PDF }))
            .route("/method.pdf", get(|| async { PDF }))
            .route("/price", get(|| async { Json(json!({"code":0,"msg":"","data":{"sh600519":{"day":[["2025-12-31","1390.000","1377.180","1394.000","1377.170","34766.000"]]}}})) })))
            .await.unwrap()
    });
    let document = |path: &str| VerifiedDocumentSource {
        url: format!("http://{address}/{path}"),
        sha256: format!("{:x}", Sha256::digest(PDF)),
        bytes: PDF.len(),
    };
    let sources = AShareFloatSourceRegistry {
        annual_report: document("annual.pdf"),
        concert_party_announcement: document("concert.pdf"),
        index_methodology: document("method.pdf"),
        price_endpoint: format!("http://{address}/price"),
    };
    let cache = std::env::temp_dir().join(format!("axiom-a-share-float-{}", uuid::Uuid::new_v4()));
    let state = AppState::new(default_config(), cache.clone()).with_a_share_float_sources(sources);
    (api::router(Arc::new(state)), server, cache)
}

#[tokio::test]
async fn public_practice_api_proves_free_float_is_not_unrestricted_shares() {
    let (app, server, cache) = fixture_app().await;
    let (status, result) = post(&app, json!({"concept_id":"book_free_float","module":"data","source":"issuer_disclosure","symbol":"600519","inputs":{}})).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["values"]["unrestricted_shares"], 1_252_270_215.0);
    assert_eq!(result["values"]["non_free_float_shares"], 709_132_623.0);
    assert_eq!(result["values"]["free_float_shares"], 543_137_592.0);
    assert_eq!(
        result["values"]["book_free_float"],
        543_137_592.0 / 1_252_270_215.0
    );
    assert_eq!(
        result["values"]["book_float_market_cap"],
        1_724_601_494_693.70
    );
    assert_eq!(result["float_case"]["as_of"], "2025-12-31");
    assert_eq!(
        result["float_case"]["definition"]["threshold"],
        "5% including concert parties"
    );
    assert_eq!(
        result["float_case"]["non_free_float_holders"][0]["shares"],
        681_282_935.0
    );
    assert_eq!(
        result["float_case"]["non_free_float_holders"][1]["shares"],
        27_849_688.0
    );
    assert_eq!(
        result["float_case"]["non_free_float_holders"][1]["relationship"],
        "wholly_owned_subsidiary_and_concert_party"
    );
    assert_eq!(result["float_case"]["sources"].as_array().unwrap().len(), 4);
    assert!(result["float_case"]["sources"][0]["official_url"]
        .as_str()
        .unwrap()
        .starts_with("http://127.0.0.1:"));
    assert_eq!(
        result["float_case"]["sources"][0]["verification"]["requested_url"],
        result["float_case"]["sources"][0]["official_url"]
    );
    assert!(result["notes"].as_array().unwrap().iter().any(|note| note
        .as_str()
        .unwrap_or_default()
        .contains("无限售条件流通股份不等于自由流通股")));
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}

#[tokio::test]
async fn public_catalog_pins_both_float_concepts_to_the_evidence_case() {
    let (app, server, cache) = fixture_app().await;
    let (status, catalog) = get_json(&app, "/api/practice").await;
    assert_eq!(status, StatusCode::OK);
    for id in ["book_free_float", "book_float_market_cap"] {
        let concept = catalog["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|concept| concept["id"] == id)
            .unwrap();
        assert_eq!(concept["input_kind"], "industry_case");
        assert_eq!(concept["inputs"], json!([]));
        assert_eq!(concept["plan"]["fixed_source"], "issuer_disclosure");
        assert_eq!(concept["plan"]["fixed_symbol"], "600519");
        assert!(concept["plan"]["required_datasets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "verified_official_index_methodology"));
    }
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}

#[tokio::test]
async fn public_practice_api_aligns_circulating_cap_price_and_share_date() {
    let (app, server, cache) = fixture_app().await;
    let (status, result) = post(&app, json!({"concept_id":"book_float_market_cap","module":"data","source":"issuer_disclosure","symbol":"600519","inputs":{}})).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["values"]["close_price"], 1377.18);
    assert_eq!(
        result["values"]["book_free_float"],
        543_137_592.0 / 1_252_270_215.0
    );
    assert_eq!(result["values"]["unrestricted_shares"], 1_252_270_215.0);
    assert_eq!(
        result["values"]["book_float_market_cap"],
        1_724_601_494_693.70
    );
    assert_eq!(result["units"]["book_float_market_cap"], "CNY");
    assert_eq!(result["float_case"]["price"]["trading_date"], "2025-12-31");
    assert_eq!(result["float_case"]["share_register_date"], "2025-12-31");
    assert_eq!(
        result["float_case"]["price"]["basis"],
        "unadjusted_daily_close"
    );
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}

#[tokio::test]
async fn float_case_rejects_caller_controlled_evidence() {
    let (app, server, cache) = fixture_app().await;
    let base = json!({"concept_id":"book_free_float","module":"data","source":"issuer_disclosure","symbol":"600519","inputs":{}});
    for patch in [
        json!({"symbol":"000001"}),
        json!({"source":"a_share"}),
        json!({"module":"backtest"}),
        json!({"inputs":{"free_float_shares":1252270215}}),
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
    server.abort();
    let _ = tokio::fs::remove_dir_all(cache).await;
}
