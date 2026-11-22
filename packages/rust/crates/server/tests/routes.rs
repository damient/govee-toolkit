//! The routes, called in process.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Web, router};
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
