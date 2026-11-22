//! `PUT /api/devices`, against a temporary folder.

#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use govee_toolkit_server::{Options, Storage, Web, router};
use tower::ServiceExt;

async fn put(options: &Options, body: &str) -> StatusCode {
    let request = Request::put("/api/devices")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    router(options)
        .unwrap()
        .oneshot(request)
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn an_entry_reaches_the_configuration_file_and_goes_again() {
    let dir =
        std::env::temp_dir().join(format!("govee-server-test-entries-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let options = Options {
        web: Web::None,
        storage: Some(Storage::in_dir(&dir)),
    };
    let config = dir.join("config.yaml");
    let id = "AA:BB:CC:DD:EE:FF".into();

    let keep = r#"{"keep":[{"id":"AA:BB:CC:DD:EE:FF","sku":"H6022","name":" desk "}]}"#;
    assert_eq!(put(&options, keep).await, StatusCode::OK);
    let saved = govee_toolkit::Config::load_from(&config).unwrap();
    assert_eq!(saved.name_for(&id), Some("desk"));
    assert_eq!(saved.sku_for(&id), Some("H6022"));

    assert_eq!(
        put(&options, r#"{"remove":["aa:bb:cc:dd:ee:ff"]}"#).await,
        StatusCode::OK
    );
    let saved = govee_toolkit::Config::load_from(&config).unwrap();
    assert!(saved.devices.is_empty());

    let long = format!(
        r#"{{"keep":[{{"id":"AA:BB","sku":null,"name":"{}"}}]}}"#,
        "x".repeat(65)
    );
    assert_eq!(put(&options, &long).await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_server_with_no_storage_cannot_write_entries() {
    let options = Options {
        web: Web::None,
        storage: None,
    };
    assert_eq!(
        put(&options, r#"{"remove":[]}"#).await,
        StatusCode::NOT_FOUND
    );
}
