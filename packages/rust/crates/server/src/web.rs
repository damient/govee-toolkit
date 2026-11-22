//! The web UI that `build.rs` compiles in.

use axum::extract::Request;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

include!(concat!(env!("OUT_DIR"), "/web.rs"));

/// `(path, body)` pairs. A path is relative to the root and uses `/`.
pub(crate) type Assets = &'static [(&'static str, &'static [u8])];

/// A page of Studio, which the UI routes on the path: outside `/api`, and no
/// dot in the last segment.
fn is_page(path: &str) -> bool {
    let api = path == "/api" || path.starts_with("/api/");
    !api && !path.rsplit('/').next().unwrap_or_default().contains('.')
}

/// Send a page of Studio to `/`, so that a reload or a link to `/lights`
/// gets `index.html`.
pub(crate) async fn to_index(mut request: Request) -> Request {
    if is_page(request.uri().path()) {
        *request.uri_mut() = Uri::from_static("/");
    }
    request
}

pub(crate) fn respond(assets: Assets, uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() || path.ends_with('/') {
        format!("{path}index.html")
    } else {
        path.to_owned()
    };
    match assets.iter().find(|(name, _)| *name == path) {
        Some((name, body)) => {
            let mime = mime_guess::from_path(name).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], *body).into_response()
        }
        None if assets.is_empty() => (
            StatusCode::NOT_FOUND,
            "This build of govee-server carries no web UI. Build apps/studio and build \
             again, or start with --web <directory>.",
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TWO: Assets = &[("index.html", b"<p>home</p>"), ("fonts/a.woff2", b"font")];

    fn get(assets: Assets, path: &str) -> Response {
        respond(assets, &path.parse().unwrap_or_default())
    }

    fn content_type(response: &Response) -> Option<&str> {
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
    }

    #[test]
    fn the_root_is_the_index() {
        let response = get(TWO, "/");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(content_type(&response), Some("text/html"));
    }

    #[test]
    fn a_nested_file_gets_its_type() {
        let response = get(TWO, "/fonts/a.woff2");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(content_type(&response), Some("font/woff2"));
    }

    #[test]
    fn an_unknown_path_is_not_found() {
        assert_eq!(get(TWO, "/missing.js").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_path_without_a_dot_is_a_page() {
        assert!(is_page("/"));
        assert!(is_page("/lights"));
        assert!(is_page("/setup/"));
        assert!(!is_page("/app.js"));
        assert!(!is_page("/fonts/a.woff2"));
        assert!(!is_page("/api"));
        assert!(!is_page("/api/devices"));
    }

    #[test]
    fn a_build_without_ui_says_so() {
        assert_eq!(get(&[], "/").status(), StatusCode::NOT_FOUND);
    }
}
