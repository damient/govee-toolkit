//! The local HTTP server of Studio. The routes are under `/api`, and any
//! other path is a file of the web UI that [`Options::web`] selects. A path
//! with no dot in its last segment is a page of Studio and gets `index.html`.

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

mod backup;
mod compat;
mod devices;
mod entries;
mod reset;
mod settings;
mod share;
mod storage;
mod studio;
mod web;

use axum::http::Uri;
use axum::routing::{get, post};
use axum::{Router, middleware};
use tower_http::services::ServeDir;

pub use crate::storage::Storage;

/// Loopback only: the API carries no authentication.
pub const DEFAULT_LISTEN: &str = "127.0.0.1:4080";

/// The directory that this build embeds. [`Web::Dir`] can serve it.
pub const WEB_DIR: &str = env!("GOVEE_SERVER_WEB_DIR");

/// What the server serves beside the API.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Where the web UI comes from.
    pub web: Web,
    /// The files that keep the settings. `None` keeps them in memory, and
    /// `/api/backup` answers 404.
    pub storage: Option<Storage>,
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

/// The routes. `/api/share` cannot open the network here: only [`serve`]
/// knows the port.
///
/// # Errors
///
/// `InvalidData` when `studio.json` does not parse, and the error of a read
/// of `studio.json`.
pub fn router(options: &Options) -> io::Result<Router> {
    Ok(routes(options, None)?.0)
}

fn routes(
    options: &Options,
    listen: Option<SocketAddr>,
) -> io::Result<(Router, Arc<share::Share>)> {
    let storage = options.storage.clone();
    let store = Arc::new(studio::Store::open(storage.as_ref().map(Storage::studio))?);
    let sdk = Arc::new(devices::Sdk::new(storage.clone()));
    let share = Arc::new(share::Share::new(listen, Arc::clone(&store)));
    let backup = Arc::new(backup::Backup {
        storage,
        store: Arc::clone(&store),
        share: Arc::clone(&share),
        sdk: Arc::clone(&sdk),
    });
    let api = Router::new()
        .route("/devices", get(devices::list).put(entries::put))
        .route("/devices/{id}/identify", post(devices::identify))
        .with_state(sdk)
        .merge(
            Router::new()
                .route("/share", get(share::get).post(share::set))
                .with_state(Arc::clone(&share)),
        )
        .merge(
            Router::new()
                .route("/settings", get(settings::get).put(settings::put))
                .with_state(store),
        )
        .merge(
            Router::new()
                .route("/backup", get(backup::export).post(backup::import))
                .route("/backup/restore", post(backup::restore))
                .route("/reset", post(reset::reset))
                .with_state(backup),
        );
    let mut app = Router::new().nest("/api", api);
    match &options.web {
        Web::Embedded => {
            app = app
                .fallback(|uri: Uri| async move { web::respond(web::ASSETS, &uri) })
                .layer(middleware::map_request(web::to_index));
        }
        Web::Dir(dir) => {
            app = app
                .fallback_service(ServeDir::new(dir))
                .layer(middleware::map_request(web::to_index));
        }
        Web::None => {}
    }
    let app = app.layer(middleware::from_fn_with_state(
        Arc::clone(&share),
        share::guard,
    ));
    share.set_app(app.clone());
    Ok((app, share))
}

/// Serve on `listener` until the future is dropped, inside a Tokio runtime.
///
/// # Errors
///
/// As for [`router`], or the listener error.
pub async fn serve(listener: std::net::TcpListener, options: &Options) -> io::Result<()> {
    let (app, share) = routes(options, Some(listener.local_addr()?))?;
    share.restore();
    listener.set_nonblocking(true)?;
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(tokio::net::TcpListener::from_std(listener)?, service).await
}
