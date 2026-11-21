//! The local HTTP server of Studio. The routes are under `/api`, and any
//! other path is a file of the web UI that [`Options::web`] selects.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

mod devices;
mod web;

use axum::http::Uri;
use axum::routing::get;
use axum::{Json, Router};
use govee_toolkit::Catalog;
use serde_json::json;
use tower_http::services::ServeDir;

/// Loopback only: the API carries no authentication.
pub const DEFAULT_LISTEN: &str = "127.0.0.1:4080";

/// The directory that this build embeds. [`Web::Dir`] can serve it.
pub const WEB_DIR: &str = env!("GOVEE_SERVER_WEB_DIR");

/// What the server serves beside the API.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Where the web UI comes from.
    pub web: Web,
}

/// The source of the web UI.
#[derive(Debug, Clone, Default)]
pub enum Web {
    /// The binary's copy. Without one, a path outside `/api` answers 404.
    #[default]
    Embedded,
    /// The files of a directory, read on each request.
    Dir(PathBuf),
    /// No web UI: the API alone.
    None,
}

/// The routes.
///
/// # Errors
///
/// The error of [`Catalog::embedded`].
pub fn router(options: &Options) -> govee_toolkit::codec::Result<Router> {
    let hello = Json(json!({
        "name": "govee-toolkit",
        "version": govee_toolkit::VERSION,
        "skus": Catalog::embedded()?.skus().count(),
    }));
    let api = Router::new()
        .route("/hello", get(move || async move { hello }))
        .route("/devices", get(devices::list))
        .with_state(Arc::new(devices::Sdk::default()));
    let mut app = Router::new().nest("/api", api);
    match &options.web {
        Web::Embedded => {
            app = app.fallback(|uri: Uri| async move { web::respond(web::ASSETS, &uri) });
        }
        Web::Dir(dir) => app = app.fallback_service(ServeDir::new(dir)),
        Web::None => {}
    }
    Ok(app)
}

/// Serve on `listener` until the future is dropped, inside a Tokio runtime.
///
/// # Errors
///
/// `InvalidData` when the device files do not load, or the listener error.
pub async fn serve(listener: std::net::TcpListener, options: &Options) -> io::Result<()> {
    let app = router(options).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    listener.set_nonblocking(true)?;
    axum::serve(tokio::net::TcpListener::from_std(listener)?, app).await
}
