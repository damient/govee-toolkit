//! The local HTTP server of Studio. The `govee-server` binary runs it
//! on its own, and the desktop app runs it in its process.
//!
//! The server holds no protocol logic: every answer comes from
//! `govee-toolkit`. The routes are under `/api`; any other path is a file of
//! the web UI that [`Options::web`] selects.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

mod web;

use axum::extract::State;
use axum::http::Uri;
use axum::routing::get;
use axum::{Json, Router};
use govee_toolkit::Catalog;
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

/// The address that `govee-server` listens on when `--listen` is absent.
///
/// Loopback only: the API sends commands to the lights and carries no
/// authentication.
pub const DEFAULT_LISTEN: &str = "127.0.0.1:4080";

/// What the server serves beside the API.
///
/// The API allows no other origin: a page that calls it comes from this server.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Where the web UI comes from.
    pub web: Web,
}

/// The source of the web UI.
#[derive(Debug, Clone, Default)]
pub enum Web {
    /// The build of `apps/studio` compiled into the binary. A build made
    /// without it answers every path outside `/api` with 404 and a message.
    #[default]
    Embedded,
    /// The files of a directory, read on each request: a rebuild of the UI
    /// shows on a reload.
    Dir(PathBuf),
    /// No web UI: the API alone.
    None,
}

#[derive(Debug)]
struct Shared {
    skus: usize,
}

/// The answer of `GET /api/hello`.
#[derive(Debug, Serialize)]
struct Hello {
    name: &'static str,
    version: &'static str,
    skus: usize,
}

/// Build the routes.
///
/// # Errors
///
/// The error of [`Catalog::embedded`] when the compiled-in device files do
/// not load.
pub fn router(options: &Options) -> govee_toolkit::codec::Result<Router> {
    let catalog = Catalog::embedded()?;
    let shared = Arc::new(Shared {
        skus: catalog.skus().count(),
    });

    let api = Router::new().route("/hello", get(hello)).with_state(shared);
    let mut app = Router::new().nest("/api", api);
    match &options.web {
        Web::Embedded => {
            let assets = web::embedded();
            app = app.fallback(move |uri: Uri| async move { web::respond(assets, &uri) });
        }
        Web::Dir(dir) => app = app.fallback_service(ServeDir::new(dir)),
        Web::None => {}
    }
    Ok(app)
}

/// Serve on `listener` until the future is dropped.
///
/// # Errors
///
/// [`io::ErrorKind::InvalidData`] when the device files do not load, or the
/// error of the listener.
pub async fn serve(listener: TcpListener, options: &Options) -> io::Result<()> {
    let app = router(options).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    axum::serve(listener, app).await
}

async fn hello(State(shared): State<Arc<Shared>>) -> Json<Hello> {
    Json(Hello {
        name: "govee-toolkit",
        version: govee_toolkit::VERSION,
        skus: shared.skus,
    })
}
