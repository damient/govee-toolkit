//! The web UI from a directory: a page of Studio gets `index.html`.

#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Web, router};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get(options: &Options, uri: &str) -> (StatusCode, String) {
    let request = Request::get(uri).body(Body::empty()).unwrap();
    let response = router(options).unwrap().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// One folder per test: the tests run at the same time.
fn options(name: &str) -> Options {
    let dir = std::env::temp_dir().join(format!(
        "govee-server-test-web-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<p>studio</p>").unwrap();
    std::fs::write(dir.join("app.js"), "export {};").unwrap();
    Options {
        web: Web::Dir(dir),
        ..Options::default()
    }
}

#[tokio::test]
async fn a_page_gets_the_index() {
    let options = options("page");
    for path in ["/", "/lights", "/setup/", "/stage?scene=1"] {
        assert_eq!(
            get(&options, path).await,
            (StatusCode::OK, "<p>studio</p>".to_owned()),
            "{path}"
        );
    }
}

#[tokio::test]
async fn a_file_is_itself_or_not_found() {
    let options = options("file");
    assert_eq!(
        get(&options, "/app.js").await,
        (StatusCode::OK, "export {};".to_owned())
    );
    assert_eq!(get(&options, "/missing.js").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unknown_route_of_the_api_is_not_found() {
    assert_eq!(
        get(&options("api"), "/api/nothing").await.0,
        StatusCode::NOT_FOUND
    );
}
