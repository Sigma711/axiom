use axiom::{
    api, app_state::AppState, config::default_config, disclosure_ownership::OwnershipSourceRegistry,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
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

#[tokio::test]
async fn ownership_catalog_has_fixed_disclosure_contract_and_rejects_caller_substitutions() {
    let root = std::env::temp_dir().join(format!("axiom-ownership-api-{}", uuid::Uuid::new_v4()));
    let state = AppState::new(default_config(), root.clone())
        .with_ownership_sources(OwnershipSourceRegistry::default());
    let app = api::router(Arc::new(state));
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    for id in axiom::disclosure_ownership::SUPPORTED_IDS {
        let symbol = axiom::disclosure_ownership::fixed_symbol(id);
        let concept = catalog["concepts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == *id)
            .unwrap();
        assert_eq!(concept["input_kind"], "industry_case", "{id}");
        assert_eq!(concept["inputs"], json!([]), "{id}");
        assert_eq!(concept["plan"]["fixed_symbol"], symbol, "{id}");
        assert_eq!(concept["plan"]["fixed_source"], "issuer_disclosure", "{id}");
        assert_eq!(concept["plan"]["modules"], json!(["data"]), "{id}");
        let base = json!({"concept_id":id,"module":"data","source":"issuer_disclosure","symbol":symbol,"inputs":{}});
        for patch in [
            json!({"symbol":"NOT_THE_ISSUER"}),
            json!({"source":"binance"}),
            json!({"inputs":{"fact":1}}),
            json!({"bars":[]}),
            json!({"limit":1}),
            json!({"second_symbol":"BTCUSDT"}),
            json!({"module":"backtest"}),
        ] {
            let mut body = base.clone();
            body.as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert_eq!(
                post(&app, body).await.0,
                StatusCode::BAD_REQUEST,
                "{id} {patch}"
            );
        }
    }
    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn verified_ownership_http_returns_reconciled_share_counts_and_source_errors_fail_closed() {
    // A transport fixture exercises the HTTP trust boundary, not the original-source audit.
    static PDF: &[u8] = b"%PDF-1.4\nverified disclosure transport fixture\n%%EOF\n";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/report.pdf", get(|| async { PDF })),
        )
        .await
        .unwrap();
    });
    let root = std::env::temp_dir().join(format!("axiom-ownership-http-{}", uuid::Uuid::new_v4()));
    let mut sources = OwnershipSourceRegistry::default();
    sources.moutai.url = format!("http://{address}/report.pdf");
    sources.moutai.sha256 = format!("{:x}", Sha256::digest(PDF));
    sources.moutai.bytes = PDF.len();
    for source in [
        &mut sources.cboe_short_interest,
        &mut sources.airbus_consensus,
        &mut sources.airbus_fy_results,
        &mut sources.mwb_report,
        &mut sources.signify_consensus,
    ] {
        source.url = format!("http://{address}/report.pdf");
        source.sha256 = format!("{:x}", Sha256::digest(PDF));
        source.bytes = PDF.len();
    }
    let state =
        AppState::new(default_config(), root.clone()).with_ownership_sources(sources.clone());
    let app = api::router(Arc::new(state));
    let base = json!({"concept_id":"restricted_shares","module":"data","source":"issuer_disclosure","symbol":"600519","inputs":{}});
    let (status, result) = post(&app, base.clone()).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["input_kind"], "industry_case");
    assert_eq!(result["source"], "issuer_disclosure");
    assert_eq!(
        result["bar_origin"],
        "server_verified_disclosures_and_dated_observations"
    );
    assert_eq!(result["values"]["restricted_shares_balance"], 0.0);
    assert_eq!(result["values"]["restricted_share_fraction"], 0.0);
    assert_eq!(result["industry_case"]["published"], "2026-04-17");
    assert_eq!(
        result["industry_case"]["verification"]["matched_sha256"],
        sources.moutai.sha256
    );
    assert_eq!(
        result["industry_case"]["verification"]["matched_bytes"],
        PDF.len()
    );
    assert_eq!(result["inputs"], json!({}));
    assert_eq!(result["bars"], json!([]));
    // Independently worked examples from the page-cited disclosure audit.
    for (id, key, expected) in [
        (
            "buyback_rate",
            "buyback_rate",
            4_014_644.0 / 1_256_197_800.0,
        ),
        (
            "holder_concentration",
            "top_holder_fraction",
            874_519_547.0 / 1_252_270_215.0,
        ),
        (
            "institution_holding",
            "institution_holding",
            34_904_752.0 / 1_252_270_215.0,
        ),
        ("restricted_shares", "restricted_share_fraction", 0.0),
        ("share_pledge", "share_pledge", 0.0),
        (
            "insider_trading",
            "insider_trading_rate",
            2_071_359.0 / 679_211_576.0,
        ),
        ("short_interest", "days_to_cover", 378_713.0 / 56_692.0),
        ("consensus", "mean_eps", 2.82),
        ("earnings_surprise", "earnings_surprise", 0.45 / 2.82),
        ("target_upside", "target_upside", -14.90 / 159.90),
        ("forecast_dispersion", "forecast_dispersion", 83.0 / 1533.0),
        ("revision", "earnings_revision", 0.16 / 5.85),
    ] {
        let (status, payload) = post(&app, json!({"concept_id":id,"module":"data","source":"issuer_disclosure","symbol":axiom::disclosure_ownership::fixed_symbol(id),"inputs":{}})).await;
        assert_eq!(status, StatusCode::OK, "{id}: {payload}");
        assert!(
            (payload["values"][key].as_f64().unwrap() - expected).abs() < 1e-12,
            "{id}"
        );
        assert_eq!(payload["industry_facts"]["calculation"]["result_key"], key);
        let (expected_label, expected_reporting_entity, expected_metric_entity) = match id {
            "buyback_rate"
            | "holder_concentration"
            | "institution_holding"
            | "restricted_shares"
            | "share_pledge"
            | "insider_trading" => (
                "公司股东披露",
                "贵州茅台酒股份有限公司",
                "贵州茅台酒股份有限公司",
            ),
            "short_interest" => (
                "市场空仓报告",
                "Cboe BZX 合并空仓报告",
                "ARK Next Generation Technology (exact Cboe CSV security name)",
            ),
            "consensus" => ("预测汇总", "Airbus SE（外部分析师预测汇总）", "Airbus SE"),
            "earnings_surprise" => (
                "预测汇总与公司业绩原文",
                "Airbus SE（外部分析师预测汇总与法规披露）",
                "Airbus SE",
            ),
            "forecast_dispersion" => (
                "预测汇总",
                "Signify N.V.（外部分析师预测汇总）",
                "Signify N.V.",
            ),
            "revision" | "target_upside" => ("分析师报告", "mwb research", "Airbus SE"),
            _ => unreachable!(),
        };
        assert_eq!(payload["industry_case"]["label"], expected_label, "{id}");
        assert_eq!(
            payload["industry_case"]["issuer"]["reporting_entity"], expected_reporting_entity,
            "{id}"
        );
        assert_eq!(
            payload["industry_case"]["issuer"]["metric_entity"], expected_metric_entity,
            "{id}"
        );
        assert!(
            payload["industry_case"]["pdf_pages"].as_array().is_some(),
            "{id}"
        );
        assert!(
            payload["industry_facts"]["definitions"]["metric"]
                .as_str()
                .is_some(),
            "{id}"
        );
        assert!(
            payload["industry_facts"]["field_provenance"]
                .as_object()
                .is_some(),
            "{id}"
        );
        assert!(
            !payload["industry_facts"]["reported_facts"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{id}"
        );
    }
    sources.moutai.url = "http://127.0.0.1:1/unreachable".into();
    sources.moutai.sha256 = "0".repeat(64);
    let broken = api::router(Arc::new(
        AppState::new(default_config(), root.clone()).with_ownership_sources(sources),
    ));
    assert_eq!(post(&broken, base).await.0, StatusCode::BAD_GATEWAY);
    server.abort();
    let _ = tokio::fs::remove_dir_all(root).await;
}
