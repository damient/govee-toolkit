//! `PUT /api/devices/{id}/name`, against a temporary folder.

#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use govee_toolkit_server::{Options, Storage, Web, router};
use tower::ServiceExt;

async fn rename(options: &Options, body: &str) -> StatusCode {
    let request = Request::put("/api/devices/AA:BB:CC:DD:EE:FF/name")
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
async fn a_name_reaches_the_configuration_file_and_goes_again() {
    let dir = std::env::temp_dir().join(format!("govee-server-test-names-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let options = Options {
        web: Web::None,
        storage: Some(Storage::in_dir(&dir)),
    };
    let config = dir.join("config.yaml");

    assert_eq!(
        rename(&options, r#"{"name":" desk "}"#).await,
        StatusCode::OK
    );
    let saved = govee_toolkit::Config::load_from(&config).unwrap();
    assert_eq!(saved.name_for(&"AA:BB:CC:DD:EE:FF".into()), Some("desk"));

    assert_eq!(rename(&options, r#"{"name":null}"#).await, StatusCode::OK);
    let saved = govee_toolkit::Config::load_from(&config).unwrap();
    assert_eq!(saved.name_for(&"AA:BB:CC:DD:EE:FF".into()), None);

    let long = format!(r#"{{"name":"{}"}}"#, "x".repeat(65));
    assert_eq!(rename(&options, &long).await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_server_with_no_storage_cannot_rename() {
    let options = Options {
        web: Web::None,
        storage: None,
    };
    assert_eq!(
        rename(&options, r#"{"name":"desk"}"#).await,
        StatusCode::NOT_FOUND
    );
}
