//! `/api/rooms`.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Storage, Web, router};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn folder(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("govee-server-rooms-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn call(dir: &Path, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
    let options = Options {
        web: Web::None,
        storage: Some(Storage::in_dir(dir)),
    };
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = router(&options).unwrap().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn rooms_survive_a_restart_and_keep_their_ids() {
    let dir = folder("restart");
    let (_, empty) = call(&dir, "GET", "/api/rooms", "").await;
    assert_eq!(empty, json!({ "rooms": [] }));

    let rooms = json!({ "rooms": [
        { "name": "Office", "devices": ["AA:BB"] },
        { "name": "Kitchen" },
    ] });
    let (status, saved) = call(&dir, "PUT", "/api/rooms", &rooms.to_string()).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["rooms"][0]["id"], "room-1");
    assert_eq!(saved["rooms"][1]["devices"], json!([]));

    let renamed = json!({ "rooms": [{ "id": "room-2", "name": "Den", "devices": ["AA:BB"] }] });
    call(&dir, "PUT", "/api/rooms", &renamed.to_string()).await;
    let (_, read) = call(&dir, "GET", "/api/rooms", "").await;
    assert_eq!(read, renamed);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_room_that_is_not_valid_changes_nothing() {
    let dir = folder("invalid");
    let rooms = json!({ "rooms": [{ "name": "A" }, { "name": "a" }] });
    let (status, _) = call(&dir, "PUT", "/api/rooms", &rooms.to_string()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!dir.join("rooms.json").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_device_that_leaves_the_configuration_leaves_its_rooms() {
    let dir = folder("forget");
    let rooms = json!({ "rooms": [
        { "name": "A", "devices": ["AA:BB", "CC:DD"] },
        { "name": "B", "devices": ["aa:bb"] },
    ] });
    call(&dir, "PUT", "/api/rooms", &rooms.to_string()).await;
    let (status, _) = call(&dir, "PUT", "/api/devices", r#"{"remove":["AA:BB"]}"#).await;
    assert_eq!(status, StatusCode::OK);
    let (_, read) = call(&dir, "GET", "/api/rooms", "").await;
    assert_eq!(read["rooms"][0]["devices"], json!(["CC:DD"]));
    assert_eq!(read["rooms"][1]["devices"], json!([]));
    std::fs::remove_dir_all(&dir).unwrap();
}
