use axiom::{
    api,
    app_state::AppState,
    config::default_config,
    data::{DataFeed, SyntheticFeed},
    practice,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

fn app() -> axum::Router {
    api::router(Arc::new(AppState::new(
        default_config(),
        PathBuf::from("target/practice-api-data"),
    )))
}
async fn request(app: &axum::Router, path: &str, value: Value) -> (StatusCode, Value) {
    let r = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let b = to_bytes(r.into_body(), 10_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&b).unwrap_or_else(|_| json!(String::from_utf8_lossy(&b))),
    )
}

#[tokio::test]
async fn every_knowledge_concept_has_one_canonical_data_exploration_practice() {
    let app = app();
    let bars = SyntheticFeed::new(31)
        .fetch_historical(
            "BTCUSDT",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            300,
        )
        .unwrap();
    for concept in practice::catalog() {
        let mut first: Option<Value> = None;
        for module in ["data"] {
            let(status,out)=request(&app,"/api/practice",json!({"concept_id":concept.id,"module":module,"symbol":"BTCUSDT","source":"synthetic","bars":bars,"inputs":{}})).await;
            assert_eq!(status, StatusCode::OK, "{} {module}: {out}", concept.id);
            assert_eq!(out["concept_id"], concept.id);
            assert_eq!(out["module"], module);
            assert!(
                ["computed", "undefined"].contains(&out["status"].as_str().unwrap()),
                "{}: {out}",
                concept.id
            );
            if out["status"] == "undefined" {
                assert!(
                    !out["reason"].as_str().unwrap_or("").is_empty(),
                    "{} needs an explanation",
                    concept.id
                );
            }
            assert!(
                !out["values"].as_object().unwrap().is_empty(),
                "{} has no output",
                concept.id
            );
            for (key, value) in out["values"].as_object().unwrap() {
                assert!(
                    out["units"][key].is_string(),
                    "{} {key} missing unit",
                    concept.id
                );
                assert!(
                    value.is_null() || value.as_f64().is_some_and(f64::is_finite),
                    "{} {key} invalid",
                    concept.id
                );
            }
            if let Some(first) = &first {
                assert_eq!(
                    first, &out["values"],
                    "{} differs across identical module inputs",
                    concept.id
                );
            } else {
                first = Some(out["values"].clone());
            }
            if concept.input_kind == "market_bars" {
                assert_eq!(out["context"], "module_snapshot");
                assert_eq!(out["provenance"], "provided_market_bars");
            }
        }
    }
}

#[tokio::test]
async fn external_exercises_do_not_fetch_market_bars_and_invalid_requests_are_explicit() {
    let app = app();
    let (status,out)=request(&app,"/api/practice",json!({"concept_id":"book_funding","module":"data","source":"real","inputs":{"is_long":false}})).await;
    assert_eq!(status, StatusCode::OK, "{out}");
    assert_eq!(out["values"]["payment"], -1.0);
    assert_eq!(out["bars"], json!([]));
    for body in [
        json!({"concept_id":"sma","module":"unknown"}),
        json!({"concept_id":"not_registered","module":"data"}),
        json!({"concept_id":"rsi","module":"data","inputs":{"period":0}}),
        json!({"concept_id":"book_funding","module":"paper","inputs":{"rate":0.1}}),
        json!({"concept_id":"eps","module":"compare","inputs":[]}),
    ] {
        let (status, _) = request(&app, "/api/practice", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, out) = request(
        &app,
        "/api/practice",
        json!({"concept_id":"rsi","module":"data","bars":[]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(out["status"], "undefined");
}

#[tokio::test]
async fn websocket_pushes_again_without_needing_a_client_message() {
    use futures::StreamExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move { axum::serve(listener, app()).await.unwrap() });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/api/paper/ws"))
            .await
            .unwrap();
    for _ in 0..2 {
        let message = tokio::time::timeout(std::time::Duration::from_secs(4), socket.next())
            .await
            .expect("server must send periodically")
            .unwrap()
            .unwrap();
        let snapshot: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        assert!(snapshot["equity"].as_f64().unwrap().is_finite());
        assert!(snapshot["is_running"].is_boolean());
    }
    socket.close(None).await.unwrap();
    server.abort();
}
