//! Each request here fails before the SDK starts, so no test opens a socket.

#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Web, router};
use tower::ServiceExt;

async fn apply(body: &str) -> StatusCode {
    let options = Options {
        web: Web::None,
        storage: None,
    };
    let request = Request::builder()
        .method("POST")
        .uri("/api/apply")
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    router(&options)
        .unwrap()
        .oneshot(request)
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn a_request_that_changes_nothing_is_refused() {
    assert_eq!(
        apply(r#"{"ids":[],"power":true}"#).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(apply(r#"{"ids":["AA:BB"]}"#).await, StatusCode::BAD_REQUEST);
    let empty = r##"{"ids":["AA:BB"],"segment":{"zones":[],"color":"#ff0000"}}"##;
    assert_eq!(apply(empty).await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_value_that_is_not_valid_is_refused() {
    let color = r#"{"ids":["AA:BB"],"color":"red"}"#;
    assert_eq!(apply(color).await, StatusCode::UNPROCESSABLE_ENTITY);
    let key = r#"{"ids":["AA:BB"],"hue":10}"#;
    assert_eq!(apply(key).await, StatusCode::UNPROCESSABLE_ENTITY);
}
