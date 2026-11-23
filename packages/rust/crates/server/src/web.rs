//! The web UI that `build.rs` compiles in.

use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

include!(concat!(env!("OUT_DIR"), "/web.rs"));

/// `(path, body, etag)`. A path is relative to the root and uses `/`.
pub(crate) type Assets = &'static [(&'static str, &'static [u8], &'static str)];

fn is_page(path: &str) -> bool {
    let api = path == "/api" || path.starts_with("/api/");
    !api && !path.rsplit('/').next().unwrap_or_default().contains('.')
}

/// The UI routes on the path, so a reload of `/lights` must get `index.html`.
pub(crate) async fn to_index(mut request: Request) -> Request {
    if is_page(request.uri().path()) {
        *request.uri_mut() = Uri::from_static("/");
    }
    request
}

/// `no-cache` with an `ETag`, because the file names carry no hash.
pub(crate) fn respond(assets: Assets, uri: &Uri, headers: &HeaderMap) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() || path.ends_with('/') {
        format!("{path}index.html")
    } else {
        path.to_owned()
    };
    match assets.iter().find(|(name, ..)| *name == path) {
        Some((name, body, tag)) => {
            let cache = [(header::ETAG, *tag), (header::CACHE_CONTROL, "no-cache")];
            let fresh = headers
                .get(header::IF_NONE_MATCH)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.split(',').any(|item| item.trim() == *tag));
            if fresh {
                return (StatusCode::NOT_MODIFIED, cache).into_response();
            }
            let mime = mime_guess::from_path(name).first_or_octet_stream();
            (cache, [(header::CONTENT_TYPE, mime.as_ref())], *body).into_response()
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

    static TWO: Assets = &[
        ("index.html", b"<p>home</p>", "\"1\""),
        ("fonts/a.woff2", b"font", "\"2\""),
    ];

    fn get(assets: Assets, path: &str) -> Response {
        respond(assets, &path.parse().unwrap_or_default(), &HeaderMap::new())
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
    fn a_file_that_did_not_change_gets_304() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            axum::http::HeaderValue::from_static("\"0\", \"2\""),
        );
        let uri = "/fonts/a.woff2".parse().unwrap_or_default();
        assert_eq!(
            respond(TWO, &uri, &headers).status(),
            StatusCode::NOT_MODIFIED
        );
        let response = get(TWO, "/fonts/a.woff2");
        assert_eq!(
            response
                .headers()
                .get(header::ETAG)
                .map(axum::http::HeaderValue::as_bytes),
            Some(&b"\"2\""[..])
        );
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
