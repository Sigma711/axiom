use axiom::{api, app_state::AppState, config::default_config};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

fn app() -> Router {
    api::router(Arc::new(AppState::new(
        default_config(),
        PathBuf::from("target/alpha-api-data"),
    )))
}

async fn body(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes))),
    )
}

#[tokio::test]
async fn direct_alpha_learning_url_serves_the_application_shell() {
    let response = app()
        .oneshot(Request::get("/learn/alphas").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("id=\"root\""));
}

#[tokio::test]
async fn catalog_keeps_worldquant_and_exact_qlib_default_feature_set_separate() {
    let response = app()
        .oneshot(
            Request::get("/api/alphas/catalog")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, result) = body(response).await;
    assert_eq!(status, StatusCode::OK);
    let collections = result["collections"].as_array().unwrap();
    assert_eq!(collections.len(), 2);
    assert_eq!(collections[0]["id"], "worldquant101");
    assert_eq!(collections[1]["id"], "qlib_alpha158");
    assert_eq!(collections[0]["factors"].as_array().unwrap().len(), 101);
    assert_eq!(collections[1]["factors"].as_array().unwrap().len(), 158);
    assert_eq!(collections[0]["factors"][100]["status"], "available");
    assert_eq!(
        collections[0]["factors"][100]["formula"],
        "((close - open) / ((high - low) + .001))"
    );
    let qlib = collections[1]["factors"].as_array().unwrap();
    let names = qlib
        .iter()
        .map(|factor| factor["id"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(names.len(), 158);
    for (id, status) in [("kmid", "available"), ("vwap0", "unavailable")] {
        let factor = qlib.iter().find(|factor| factor["id"] == id).unwrap();
        assert_eq!(factor["status"], status);
        assert!(factor["formula"].as_str().is_some());
        if status == "available" {
            assert!(factor["code_url"]
                .as_str()
                .unwrap()
                .contains("alpha_research.rs"));
        }
    }
    assert_eq!(collections[0]["factors"][0]["status"], "unavailable");
    assert!(collections[0]["factors"][0]["formula"].as_str().is_some());
    assert!(collections[0]["factors"][100]["code_url"]
        .as_str()
        .unwrap()
        .contains("alpha_research.rs"));
}

#[tokio::test]
async fn unsupported_formula_and_market_are_rejected_before_provider_io() {
    let app = app();
    for (request, expected) in [
        (
            json!({"collection":"worldquant101","id":"alpha1","source":"us_stock","symbol":"AAPL","limit":120}),
            "该因子尚未",
        ),
        (
            json!({"collection":"qlib_alpha158","id":"kmid","source":"us_stock","symbol":"AAPL","limit":120}),
            "对应证券市场",
        ),
        (
            json!({"collection":"unknown","id":"alpha101","source":"us_stock","symbol":"AAPL","limit":120}),
            "未知的因子系列",
        ),
        (
            json!({"collection":"gtja191","id":"alpha191","source":"a_share","symbol":"600519","limit":120}),
            "未知的因子系列",
        ),
        (
            json!({"collection":"worldquant101","id":"alpha101","source":"us_stock","symbol":"AAPL","limit":501}),
            "30–500",
        ),
        (
            json!({"collection":"qlib_alpha158","id":"vwap0","source":"a_share","symbol":"600519","limit":120}),
            "该因子尚未",
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/alphas/research")
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, result) = body(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
        assert!(result.to_string().contains(expected), "{result}");
    }
}
