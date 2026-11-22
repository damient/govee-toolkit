//! The routes, called in process.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Web, router};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get(options: &Options, uri: &str) -> axum::response::Response {
    let request = Request::get(uri);
    router(options)
        .unwrap()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn hello_names_the_crate_and_counts_the_skus() {
    let response = get(&Options::default(), "/api/hello").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["name"], "govee-toolkit-server");
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["sdk"], govee_toolkit::VERSION);
    assert!(value["skus"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn without_a_web_ui_other_paths_are_not_found() {
    let response = get(
        &Options {
            web: Web::None,
            ..Options::default()
        },
        "/index.html",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
