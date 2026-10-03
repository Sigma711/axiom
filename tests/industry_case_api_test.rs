use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    industry_case::{fixed_symbol, IndustrySourceConfig, IndustrySourceRegistry, SUPPORTED_IDS},
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
                .route("/moutai.pdf", get(|| async { PDF }))
                .route("/realty.pdf", get(|| async { PDF }))
                .route("/delta.pdf", get(|| async { PDF }))
                .route("/costco.pdf", get(|| async { PDF })),
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
        moutai: source("moutai.pdf"),
        realty_income: source("realty.pdf"),
        delta: source("delta.pdf"),
        costco: source("costco.pdf"),
        moderna: source("costco.pdf"),
        petrobras: source("costco.pdf"),
        barrick: source("costco.pdf"),
        siemens: source("costco.pdf"),
        meta: source("costco.pdf"),
        spotify: source("costco.pdf"),
        snowflake: source("costco.pdf"),
        similarweb: source("costco.pdf"),
        zoom: source("costco.pdf"),
        smic: source("costco.pdf"),
        verizon: source("costco.pdf"),
        frontline: source("costco.pdf"),
    };
    let cache = std::env::temp_dir().join(format!("axiom-industry-api-{}", uuid::Uuid::new_v4()));
    let state = AppState::new(default_config(), cache.clone()).with_industry_sources(registry);
    (api::router(Arc::new(state)), server, cache)
}

#[tokio::test]
async fn industry_api_verifies_each_fixed_issuer_disclosure() {
    let (app, server, cache) = fixture_app().await;
    for concept in SUPPORTED_IDS {
        let symbol = fixed_symbol(concept).unwrap();
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
#[allow(clippy::approx_constant)] // 3.14 is Meta's reported DAP literal, not an approximation of PI.
async fn new_industry_cases_return_independently_recomputed_values() {
    let (app, server, cache) = fixture_app().await;
    let expectations = [
        (
            "book_biopharma_cash_runway",
            "cash_runway_months",
            9_519.0 / 3_004.0 * 12.0,
        ),
        ("book_energy_lifting_cost", "lifting_cost", 6.05),
        ("book_energy_reserve_life", "reserve_life", 13.2),
        ("book_gold_aisc", "gold_aisc", 1_350.0),
        ("book_industrial_backlog", "order_backlog", 113.0),
        (
            "book_industrial_book_to_bill",
            "book_to_bill",
            84_056.0 / 75_930.0,
        ),
        ("book_internet_arpu", "premium_arpu", 4.19),
        (
            "book_internet_dau_mau",
            "daily_monthly_active_ratio",
            3.14 / 3.96,
        ),
        ("book_saas_cac_payback", "cac_payback_lower_bound", 21.0),
        ("book_saas_churn", "monthly_customer_churn", 0.032),
        ("book_saas_nrr", "net_revenue_retention", 1.31),
        (
            "book_semiconductor_asp",
            "implied_revenue_per_equivalent_wafer",
            2_207_281.0 * 0.925 * 1_000.0 / 1_991_761.0,
        ),
        (
            "book_semiconductor_utilization",
            "capacity_utilization",
            0.855,
        ),
        ("book_shipping_tce", "vlcc_spot_tce", 49_600.0),
        ("book_telecom_arpu", "prepaid_arpu", 31.17),
        ("book_telecom_churn", "prepaid_monthly_churn", 0.0426),
    ];
    for (concept, metric, expected) in expectations {
        let symbol = fixed_symbol(concept).unwrap();
        let (status, result) = post(
            &app,
            json!({"concept_id":concept,"module":"data","source":"issuer_disclosure","symbol":symbol,"inputs":{}}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{concept}: {result}");
        let actual = result["values"][metric].as_f64().unwrap();
        assert!(
            (actual - expected).abs() <= expected.abs().max(1.0) * 1e-12,
            "{concept}/{metric}: expected {expected}, got {actual}"
        );
        assert!(result["industry_facts"]["calculation"]["formula"]
            .as_str()
            .is_some_and(|formula| !formula.trim().is_empty()));
        assert!(result["industry_facts"]["calculation"]["symbol_mapping"]
            .as_object()
            .is_some_and(|mapping| !mapping.is_empty()));
        assert!(result["notes"].as_array().is_some_and(|notes| notes
            .iter()
            .all(|note| note.as_str().is_some_and(|text| !text.trim().is_empty()))));
        if concept == "book_saas_cac_payback" {
            assert_eq!(result["values"]["cac_payback_lower_bound"], 21.0);
            assert_eq!(result["values"]["cac_payback_upper_bound"], 22.0);
            assert_eq!(
                result["units"]["cac_payback_lower_bound"],
                "months lower bound"
            );
            assert_eq!(
                result["units"]["cac_payback_upper_bound"],
                "months upper bound"
            );
        }
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

#[tokio::test]
async fn moutai_share_structure_is_dated_and_never_free_float() {
    let (app, server, cache) = fixture_app().await;
    let base = json!({"concept_id":"book_share_counts","module":"data","source":"issuer_disclosure","symbol":"600519","inputs":{}});
    let (status, result) = post(&app, base.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["values"], json!({"restricted_shares_residual":0.0}));
    assert_eq!(
        result["units"],
        json!({"restricted_shares_residual":"shares"})
    );
    assert_eq!(result["industry_case"]["published"], "2026-04-17");
    assert_eq!(result["industry_case"]["period"]["end"], "2025-12-31");
    assert_eq!(result["industry_case"]["pdf_pages"], json!([47]));
    let facts = result["industry_facts"]["reported_facts"]
        .as_array()
        .unwrap();
    for (key, literal) in [
        ("opening_total_shares", 1256197800.0),
        ("cancelled_shares", 3927585.0),
        ("closing_total_shares", 1252270215.0),
        ("unrestricted_shares", 1252270215.0),
    ] {
        let fact = facts.iter().find(|f| f["key"] == key).unwrap();
        assert_eq!(fact["value"], literal);
        assert_eq!(fact["unit"], "shares");
        assert_eq!(fact["pdf_page"], 47);
    }
    assert!(result["values"].get("free_float_shares").is_none());
    for patch in [
        json!({"symbol":"AAPL"}),
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
