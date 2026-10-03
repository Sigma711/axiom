use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    disclosure_valuation::{self, ValuationSourceRegistry},
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

async fn serve_once(body: &'static [u8]) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request).await.unwrap();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
    });
    format!("http://{address}/nuveen.pdf")
}

#[tokio::test]
async fn nav_discount_api_discloses_the_verified_archived_original_fallback() {
    let root = std::env::temp_dir().join(format!(
        "axiom-valuation-archive-api-{}",
        uuid::Uuid::new_v4()
    ));
    let mut registry = ValuationSourceRegistry::default();
    registry.sources.get_mut("nuveen_proxy_2025").unwrap().url =
        serve_once(b"<html>TIAA security page</html>").await;
    let app = api::router(Arc::new(
        AppState::new(default_config(), root.clone()).with_valuation_sources(registry),
    ));
    let (status, result) = post(&app, json!({"concept_id":"book_nav_discount","module":"data","source":"issuer_disclosure","symbol":"DIAX","inputs":{}})).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    let verification = &result["industry_case"]["verification"];
    assert_eq!(verification["status"], "verified_archived_original");
    assert_eq!(verification["matched_bytes"], 1_438_050);
    assert!(verification["retrieval_note"]
        .as_str()
        .unwrap()
        .contains("原站当前未返回已核验 PDF"));
    assert_eq!(
        result["values"]["nav_premium_discount"],
        (14.91 - 16.30) / 16.30
    );
    let _ = tokio::fs::remove_dir_all(root).await;
}

fn nasdaq(symbol: &str, rows: &[(&str, &str)]) -> Vec<u8> {
    serde_json::to_vec(&json!({"data":{"symbol":symbol,"tradesTable":{"rows":rows.iter().map(|(date,close)|json!({"date":date,"close":format!("${close}")})).collect::<Vec<_>>()}}})).unwrap()
}

fn seeded(root: &std::path::Path) -> ValuationSourceRegistry {
    let mut registry = ValuationSourceRegistry::default();
    let directory = root.join("valuation-sources");
    std::fs::create_dir_all(&directory).unwrap();
    for (id, source) in &mut registry.sources {
        let bytes = match id.as_str() {
            "apple_price" => nasdaq("AAPL", &[("11/01/2024","222.91"),("11/03/2025","269.05"),("11/21/2025","271.49"),("11/26/2025","277.55")]),
            "ebay_price" => nasdaq("EBAY", &[("10/01/2025","87.58")]),
            "hpq_price" => nasdaq("HPQ", &[("11/26/2025","23.98")]),
            "fred_cpi" => b"observation_date,CPIAUCSL\n2016-09-01,241.176\n2017-09-01,246.435\n2018-09-01,252.182\n2019-09-01,256.430\n2020-09-01,259.997\n2021-09-01,273.910\n2022-09-01,296.349\n2023-09-01,307.276\n2024-09-01,314.732\n2025-09-01,324.245\n".to_vec(),
            _ => {
                let fixture = format!("transport-verified fixture for {id}").into_bytes();
                source.bytes = fixture.len();
                source.sha256 = format!("{:x}", Sha256::digest(&fixture));
                fixture
            }
        };
        std::fs::write(directory.join(format!("{id}.source")), bytes).unwrap();
    }
    registry
}

#[tokio::test]
async fn valuation_catalog_is_fixed_and_rejects_caller_substitutions() {
    let root = std::env::temp_dir().join(format!("axiom-valuation-api-{}", uuid::Uuid::new_v4()));
    let app = api::router(Arc::new(AppState::new(default_config(), root.clone())));
    let response = app
        .clone()
        .oneshot(Request::get("/api/practice").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let catalog: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10_000_000).await.unwrap()).unwrap();
    for id in disclosure_valuation::SUPPORTED_IDS {
        let symbol = disclosure_valuation::fixed_symbol(id);
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
            json!({"symbol":"NOT_THE_FIXED_CASE"}),
            json!({"source":"binance"}),
            json!({"inputs":{"price":1}}),
            json!({"bars":[]}),
            json!({"limit":1}),
            json!({"second_symbol":"MSFT"}),
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

fn expected_result(id: &str) -> (&'static str, f64, &'static str) {
    match id {
        "altman_z" => ("altman_z", 10.61901749051463, "score"),
        "beneish_m" => ("beneish_m", -2.294943021712222, "score"),
        "book_dcf" => ("dcf_value_per_share", 125.12238523309892, "USD/share"),
        "book_dividend_yield" => ("dividend_yield", 1.02 / 269.05, "fraction"),
        "book_earnings_yield" => ("earnings_yield", 7.46 / 269.05, "fraction"),
        "book_ebitda_margin" => ("ebitda_margin", 144748.0 / 416161.0, "fraction"),
        "book_ev" => ("enterprise_value", 4037468.603, "USD millions"),
        "book_ev_ebit" => ("ev_ebit", 4037468.603 / 133050.0, "multiple"),
        "book_ev_sales" => ("ev_sales", 4037468.603 / 416161.0, "multiple"),
        "book_fcf_yield" => ("fcf_yield", 98767.0 / 3974745.603, "fraction"),
        "book_intangibles_ratio" => ("intangibles_ratio", 4388.0 / 5158.0, "fraction"),
        "book_interest_coverage" => ("interest_coverage", 2318.0 / 259.0, "multiple"),
        "book_nav_discount" => ("nav_premium_discount", (14.91 - 16.30) / 16.30, "fraction"),
        "book_net_debt_ebitda" => ("net_debt_ebitda", 62723.0 / 144748.0, "multiple"),
        "book_payout_ratio" => ("payout_ratio", 15421.0 / 112010.0, "fraction"),
        "book_price_cashflow" => (
            "price_operating_cash_flow",
            3974745.603 / 111482.0,
            "multiple",
        ),
        "book_ptbv" => ("price_tangible_book", 87.58 * 471.0 / 770.0, "multiple"),
        "book_qoq" => ("qoq_growth", 102466.0 / 94036.0 - 1.0, "fraction"),
        "ev_ebitda" => ("ev_ebitda", 4037468.603 / 144748.0, "multiple"),
        "goodwill_ratio" => (
            "goodwill_intangibles_to_equity",
            4388.0 / 5158.0,
            "fraction",
        ),
        "industry_pe_compare" => ("apple_pe", 277.55 / 7.46, "multiple"),
        "pb" => ("pb", 3974745.603 / 73733.0, "multiple"),
        "pe" => ("pe", 269.05 / 7.46, "multiple"),
        "peg" => (
            "peg",
            (269.05 / 7.46) / ((7.46 / 6.08 - 1.0) * 100.0),
            "multiple per percentage point",
        ),
        "piotroski" => ("piotroski_f_score", 7.0, "0-9 score"),
        "ps" => ("ps", 3974745.603 / 416161.0, "multiple"),
        "roic" => ("roic", 0.8314270092598283, "fraction"),
        "book_cape" => ("cape", 53.214146004905224, "multiple"),
        _ => panic!("missing expected valuation result: {id}"),
    }
}

#[tokio::test]
async fn every_valuation_concept_succeeds_through_the_http_boundary() {
    let root = std::env::temp_dir().join(format!("axiom-valuation-http-{}", uuid::Uuid::new_v4()));
    let registry = seeded(&root);
    let state = AppState::new(default_config(), root.clone()).with_valuation_sources(registry);
    let app = api::router(Arc::new(state));
    for id in disclosure_valuation::SUPPORTED_IDS {
        let symbol = disclosure_valuation::fixed_symbol(id);
        let (status, result) = post(&app, json!({"concept_id":id,"module":"data","source":"issuer_disclosure","symbol":symbol,"inputs":{}})).await;
        assert_eq!(status, StatusCode::OK, "{id}: {result}");
        let (key, want, unit) = expected_result(id);
        assert_eq!(
            result["industry_facts"]["calculation"]["result_key"], key,
            "{id}"
        );
        let got = result["values"][key].as_f64().unwrap();
        assert!((got - want).abs() < 1e-10, "{id}: {got} != {want}");
        assert_eq!(result["units"][key], unit, "{id}: {key}");
        if *id == "pe" {
            assert!(result["industry_case"]["period"]["label"]
                .as_str()
                .unwrap()
                .contains("2025-11-03"));
            assert_eq!(result["industry_case"]["period"]["end"], "2025-11-03");
        }
        if *id == "book_cape" {
            assert!(result["industry_case"]["period"]["label"]
                .as_str()
                .unwrap()
                .contains("2016–2025"));
            assert_eq!(result["industry_case"]["period"]["end"], "2025-11-21");
        }
        if *id == "piotroski" {
            for (signal, expected) in [
                ("positive_roa", 1),
                ("positive_cfo", 1),
                ("improving_roa", 1),
                ("cfo_exceeds_net_income", 0),
                ("declining_leverage", 1),
                ("improving_liquidity", 1),
                ("no_common_share_issuance", 0),
                ("improving_gross_margin", 1),
                ("improving_asset_turnover", 1),
            ] {
                assert_eq!(result["values"][signal], expected, "{signal}");
                assert_eq!(result["units"][signal], "binary", "{signal}");
            }
            assert!(result["values"]["signals"].is_null());
        }
        let numeric_keys = result["values"]
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(key, value)| value.is_number().then_some(key.as_str()))
            .collect::<std::collections::BTreeSet<_>>();
        let derived_metrics = result["industry_facts"]["derived_metrics"]
            .as_array()
            .unwrap();
        let described_keys = derived_metrics
            .iter()
            .map(|metric| {
                assert!(!metric["label"].as_str().unwrap().is_empty(), "{id}");
                assert!(!metric["description"].as_str().unwrap().is_empty(), "{id}");
                metric["key"].as_str().unwrap()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(described_keys, numeric_keys, "{id}");
        assert_eq!(derived_metrics.len(), numeric_keys.len(), "{id}");
        assert_eq!(result["industry_case"]["issuer"]["ticker"], symbol, "{id}");
        assert!(
            result["industry_case"]["sources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|source| source["title"].as_str().is_some()
                    && source["verification"]["fingerprint_scope"]
                        .as_str()
                        .is_some()),
            "{id}"
        );
        assert!(
            result["industry_facts"]["reported_facts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|fact| matches!(
                    fact["kind"].as_str(),
                    Some("reported" | "derived" | "assumption")
                )),
            "{id}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
