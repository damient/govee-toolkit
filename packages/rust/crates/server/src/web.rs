//! The web UI that `build.rs` compiles in.

use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

include!(concat!(env!("OUT_DIR"), "/web.rs"));

/// `(path, body)` pairs. A path is relative to the root and uses `/`.
pub(crate) type Assets = &'static [(&'static str, &'static [u8])];

pub(crate) fn embedded() -> Assets {
    ASSETS
}

pub(crate) fn respond(assets: Assets, uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() || path.ends_with('/') {
        format!("{path}index.html")
    } else {
        path.to_owned()
    };
    match assets.iter().find(|(name, _)| *name == path) {
        Some((name, body)) => ([(header::CONTENT_TYPE, content_type(name))], *body).into_response(),
        None if assets.is_empty() => (
            StatusCode::NOT_FOUND,
            "This build of govee-server carries no web UI. Build apps/studio and build \
             again, or start with --web <directory>.",
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        Some("json" | "map") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TWO: Assets = &[("index.html", b"<p>home</p>"), ("fonts/a.woff2", b"font")];

    fn get(assets: Assets, path: &str) -> Response {
        respond(assets, &path.parse().unwrap_or_default())
    }

    #[test]
    fn the_root_is_the_index() {
        let response = get(TWO, "/");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
    }

    #[test]
    fn a_nested_file_gets_its_type() {
        let response = get(TWO, "/fonts/a.woff2");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("font/woff2")
        );
    }

    #[test]
    fn an_unknown_path_is_not_found() {
        assert_eq!(get(TWO, "/missing.js").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_build_without_ui_says_so() {
        assert_eq!(get(&[], "/").status(), StatusCode::NOT_FOUND);
    }
}
