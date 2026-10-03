use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    industry_case::{
        fixed_symbol, IndustrySourceConfig, IndustrySourceRegistry, SUPPORTED_IDS, ZOOM_BYTES,
        ZOOM_SHA256,
    },
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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

const ZOOM_PDF: &[u8] =
    include_bytes!("../data/verified-sources/zoom-q1-fy2025-prepared-remarks.pdf");

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

async fn serve_once(status: &'static str, content_type: &'static str, body: Vec<u8>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request).await.unwrap();
        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
    });
    format!("http://{address}/zoom")
}

async fn zoom_app(url: String, root: &std::path::Path) -> Router {
    let mut registry = IndustrySourceRegistry::default();
    registry.zoom.url = url;
    api::router(Arc::new(
        AppState::new(default_config(), root.to_path_buf()).with_industry_sources(registry),
    ))
}

fn zoom_request() -> Value {
    json!({"concept_id":"book_saas_churn","module":"data","source":"issuer_disclosure","symbol":"ZM","inputs":{}})
}

#[tokio::test]
async fn zoom_http_failure_uses_only_the_exact_archived_original_and_preserves_warm_provenance() {
    let root =
        std::env::temp_dir().join(format!("axiom-zoom-archive-api-{}", uuid::Uuid::new_v4()));
    let app = zoom_app(
        serve_once("403 Forbidden", "text/html", b"access denied".to_vec()).await,
        &root,
    )
    .await;
    let (status, result) = post(&app, zoom_request()).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["values"]["monthly_customer_churn"], 0.032);
    assert_eq!(
        result["industry_case"]["verification"]["status"],
        "verified_archived_original"
    );
    assert!(result["industry_case"]["verification"]["retrieval_note"]
        .as_str()
        .unwrap()
        .contains("Zoom 原站请求未返回已核验 PDF"));
    let (warm_status, warm) = post(&app, zoom_request()).await;
    assert_eq!(warm_status, StatusCode::OK, "{warm}");
    assert_eq!(
        warm["industry_case"]["verification"]["status"],
        "verified_archived_original"
    );
    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn zoom_non_pdf_uses_archive_but_changed_and_oversized_pdfs_fail_closed() {
    let html_root =
        std::env::temp_dir().join(format!("axiom-zoom-html-api-{}", uuid::Uuid::new_v4()));
    let html_app = zoom_app(
        serve_once(
            "200 OK",
            "text/html",
            b"<html>security page</html>".to_vec(),
        )
        .await,
        &html_root,
    )
    .await;
    assert_eq!(post(&html_app, zoom_request()).await.0, StatusCode::OK);
    let _ = tokio::fs::remove_dir_all(html_root).await;

    let changed_root =
        std::env::temp_dir().join(format!("axiom-zoom-changed-api-{}", uuid::Uuid::new_v4()));
    let changed_app = zoom_app(
        serve_once(
            "200 OK",
            "application/pdf",
            b"%PDF-1.7\nchanged Zoom document".to_vec(),
        )
        .await,
        &changed_root,
    )
    .await;
    assert_eq!(
        post(&changed_app, zoom_request()).await.0,
        StatusCode::BAD_GATEWAY
    );
    assert!(!changed_root
        .join(format!("industry-sources/{ZOOM_SHA256}.pdf"))
        .exists());
    let _ = tokio::fs::remove_dir_all(changed_root).await;

    let oversized_root =
        std::env::temp_dir().join(format!("axiom-zoom-oversized-api-{}", uuid::Uuid::new_v4()));
    let mut oversized = b"%PDF-1.7\n".to_vec();
    oversized.resize(20_000_001, b'x');
    let oversized_app = zoom_app(
        serve_once("200 OK", "application/pdf", oversized).await,
        &oversized_root,
    )
    .await;
    assert_eq!(
        post(&oversized_app, zoom_request()).await.0,
        StatusCode::BAD_GATEWAY
    );
    assert!(!oversized_root
        .join(format!("industry-sources/{ZOOM_SHA256}.pdf"))
        .exists());
    let _ = tokio::fs::remove_dir_all(oversized_root).await;

    let mismatch_root = std::env::temp_dir().join(format!(
        "axiom-zoom-archive-mismatch-{}",
        uuid::Uuid::new_v4()
    ));
    let mut mismatch_registry = IndustrySourceRegistry::default();
    mismatch_registry.zoom.url = serve_once("403 Forbidden", "text/html", b"denied".to_vec()).await;
    mismatch_registry.zoom.sha256 = "0".repeat(64);
    let mismatch_app = api::router(Arc::new(
        AppState::new(default_config(), mismatch_root.clone())
            .with_industry_sources(mismatch_registry),
    ));
    assert_eq!(
        post(&mismatch_app, zoom_request()).await.0,
        StatusCode::BAD_GATEWAY
    );
    assert!(!mismatch_root.join("industry-sources").exists());
}

#[tokio::test]
async fn zoom_live_pdf_and_marker_integrity_keep_cache_provenance_honest() {
    assert_eq!(ZOOM_PDF.len(), ZOOM_BYTES);
    assert_eq!(format!("{:x}", Sha256::digest(ZOOM_PDF)), ZOOM_SHA256);
    let live_root =
        std::env::temp_dir().join(format!("axiom-zoom-live-api-{}", uuid::Uuid::new_v4()));
    let live_app = zoom_app(
        serve_once("200 OK", "application/pdf", ZOOM_PDF.to_vec()).await,
        &live_root,
    )
    .await;
    let (_, live) = post(&live_app, zoom_request()).await;
    assert_eq!(
        live["industry_case"]["verification"]["status"],
        "verified_then_cached"
    );
    let (_, warm) = post(&live_app, zoom_request()).await;
    assert_eq!(
        warm["industry_case"]["verification"]["status"],
        "verified_immutable_cache"
    );
    let _ = tokio::fs::remove_dir_all(live_root).await;

    let marker_root =
        std::env::temp_dir().join(format!("axiom-zoom-marker-api-{}", uuid::Uuid::new_v4()));
    let archive_app = zoom_app(
        serve_once("403 Forbidden", "text/html", b"denied".to_vec()).await,
        &marker_root,
    )
    .await;
    assert_eq!(post(&archive_app, zoom_request()).await.0, StatusCode::OK);
    let directory = marker_root.join("industry-sources");
    std::fs::write(
        directory.join(format!("{ZOOM_SHA256}.archived-original")),
        b"wrong",
    )
    .unwrap();
    let changed_app = zoom_app(
        serve_once(
            "200 OK",
            "application/pdf",
            b"%PDF-1.7\nchanged after marker corruption".to_vec(),
        )
        .await,
        &marker_root,
    )
    .await;
    assert_eq!(
        post(&changed_app, zoom_request()).await.0,
        StatusCode::BAD_GATEWAY
    );
    assert!(!directory.join(format!("{ZOOM_SHA256}.pdf")).exists());

    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join(format!("{ZOOM_SHA256}.pdf")), b"corrupt").unwrap();
    std::fs::write(
        directory.join(format!("{ZOOM_SHA256}.archived-original")),
        ZOOM_SHA256,
    )
    .unwrap();
    let repaired_app = zoom_app(
        serve_once("403 Forbidden", "text/html", b"denied".to_vec()).await,
        &marker_root,
    )
    .await;
    let (_, repaired) = post(&repaired_app, zoom_request()).await;
    assert_eq!(
        repaired["industry_case"]["verification"]["status"],
        "verified_archived_original"
    );
    assert_eq!(
        std::fs::read(directory.join(format!("{ZOOM_SHA256}.pdf"))).unwrap(),
        ZOOM_PDF
    );
    let _ = tokio::fs::remove_dir_all(marker_root).await;
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
