//! `POST /api/reset`.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Storage, Web, router};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const CONFIG: &str =
    "lan:\n  status_timeout_ms: 500\ndevices:\n  \"AA:BB:CC:DD:EE:FF\":\n    name: desk\n";

fn folder(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("govee-server-reset-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.yaml"), CONFIG).unwrap();
    dir
}

fn options(dir: &Path) -> Options {
    Options {
        web: Web::None,
        storage: Some(Storage::in_dir(dir)),
    }
}

async fn call(dir: &Path, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = router(&options(dir))
        .unwrap()
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    // A rejection of the JSON extractor is plain text.
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn light_theme(dir: &Path) {
    let change = json!({ "theme": "light" }).to_string();
    let (status, _) = call(dir, "PUT", "/api/settings", &change).await;
    assert_eq!(status, StatusCode::OK);
}

fn config(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("config.yaml")).unwrap()
}

#[tokio::test]
async fn a_reset_of_the_prefs_keeps_the_devices() {
    let dir = folder("prefs");
    light_theme(&dir).await;
    let (status, _) = call(&dir, "POST", "/api/reset", r#"{"prefs":true}"#).await;
    assert_eq!(status, StatusCode::OK);
    let (_, settings) = call(&dir, "GET", "/api/settings", "").await;
    assert_eq!(settings["theme"], "dark");
    assert_eq!(config(&dir), CONFIG);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_reset_of_the_devices_keeps_the_prefs_and_the_other_keys() {
    let dir = folder("devices");
    light_theme(&dir).await;
    let rooms = r#"{"rooms":[{"name":"Office","devices":["AA:BB:CC:DD:EE:FF"]}]}"#;
    let (status, _) = call(&dir, "PUT", "/api/rooms", rooms).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&dir, "POST", "/api/reset", r#"{"devices":true}"#).await;
    assert_eq!(status, StatusCode::OK);
    let (_, settings) = call(&dir, "GET", "/api/settings", "").await;
    assert_eq!(settings["theme"], "light");
    let text = config(&dir);
    assert!(!text.contains("desk"), "{text}");
    assert!(text.contains("status_timeout_ms: 500"), "{text}");
    let (_, rooms) = call(&dir, "GET", "/api/rooms", "").await;
    assert_eq!(rooms, json!({ "rooms": [] }));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_reset_of_nothing_is_refused() {
    let dir = folder("nothing");
    for body in ["{}", r#"{"all":true}"#] {
        let (status, _) = call(&dir, "POST", "/api/reset", body).await;
        assert!(status.is_client_error(), "{body}");
    }
    assert_eq!(config(&dir), CONFIG);
    std::fs::remove_dir_all(&dir).unwrap();
}
