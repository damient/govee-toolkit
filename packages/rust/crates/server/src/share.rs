//! `/api/share`. The API has no authentication, so sharing is off by default
//! and only a client on this machine can switch it.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, Router};
use serde_json::{Value, json};
use tokio::task::AbortHandle;

use crate::studio::Store;
use crate::{refuse, storage};

const RETRY: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Debug, Default)]
pub(crate) struct Share {
    /// `None` when [`crate::serve`] does not run the router.
    listen: Option<SocketAddr>,
    open: Mutex<Option<Open>>,
    app: OnceLock<Router>,
    store: Arc<Store>,
}

#[derive(Debug)]
struct Open {
    addr: SocketAddr,
    task: AbortHandle,
}

/// Whether the client of a request is on this machine.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Local(pub(crate) bool);

impl Share {
    pub(crate) fn new(listen: Option<SocketAddr>, store: Arc<Store>) -> Self {
        Self {
            listen,
            store,
            ..Self::default()
        }
    }

    /// At boot the network can come up after the server: a failed start tries
    /// again every [`RETRY`] while the setting stays on.
    pub(crate) fn restore(self: &Arc<Self>) {
        let wanted = |share: &Self| share.store.get().server.settings.share && !share.fixed();
        if !wanted(self) || self.start().is_ok() {
            return;
        }
        let share = Arc::clone(self);
        tokio::spawn(async move {
            while wanted(&share) && share.opened().is_none() {
                tokio::time::sleep(RETRY).await;
                if wanted(&share) && share.start().is_ok() {
                    break;
                }
            }
        });
    }

    /// Writes nothing to `studio.json`.
    pub(crate) fn apply(&self, on: bool) -> Result<(), String> {
        if self.fixed() || self.listen.is_none() {
            return Ok(());
        }
        if on {
            self.start()
        } else {
            self.stop();
            Ok(())
        }
    }

    pub(crate) fn set_app(&self, app: Router) {
        // Set once, by `crate::routes`.
        let _ = self.app.set(app);
    }

    fn lock(&self) -> MutexGuard<'_, Option<Open>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `--listen` on a non-loopback address: the switch does nothing.
    fn fixed(&self) -> bool {
        self.listen.is_some_and(|addr| !addr.ip().is_loopback())
    }

    fn opened(&self) -> Option<SocketAddr> {
        let mut open = self.lock();
        if open.as_ref().is_some_and(|o| o.task.is_finished()) {
            *open = None;
        }
        open.as_ref().map(|o| o.addr)
    }

    fn state(&self, local: Local) -> Value {
        let fixed = self.fixed();
        let address = match self.listen {
            Some(listen) if fixed && listen.ip().is_unspecified() => {
                lan_ip().ok().map(|ip| SocketAddr::new(ip, listen.port()))
            }
            Some(listen) if fixed => Some(listen),
            _ => self.opened(),
        };
        json!({
            "fixed": fixed,
            "shared": fixed || address.is_some(),
            "url": address.map(|addr| format!("http://{addr}/")),
            "control": local.0 && !fixed && self.listen.is_some(),
            "local": local.0,
        })
    }

    fn start(&self) -> Result<(), String> {
        let mut open = self.lock();
        if open.as_ref().is_some_and(|o| !o.task.is_finished()) {
            return Ok(());
        }
        let (Some(listen), Some(app)) = (self.listen, self.app.get()) else {
            return Err("this server cannot share".to_owned());
        };
        let bind = || -> io::Result<(SocketAddr, tokio::net::TcpListener)> {
            let ip = lan_ip()?;
            let listener =
                TcpListener::bind((ip, listen.port())).or_else(|_| TcpListener::bind((ip, 0)))?;
            listener.set_nonblocking(true)?;
            Ok((
                listener.local_addr()?,
                tokio::net::TcpListener::from_std(listener)?,
            ))
        };
        let (addr, listener) = bind().map_err(|e| e.to_string())?;
        let service = app
            .clone()
            .into_make_service_with_connect_info::<SocketAddr>();
        // An error of `serve` ends the task: `opened` reads it as sharing off.
        let task = tokio::spawn(async move { axum::serve(listener, service).await }).abort_handle();
        *open = Some(Open { addr, task });
        Ok(())
    }

    fn stop(&self) {
        if let Some(open) = self.lock().take() {
            open.task.abort();
        }
    }
}

/// A UDP `connect` sends no packet: it only picks the route.
fn lan_ip() -> io::Result<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    // TEST-NET-1 takes the default route. The mDNS group also has a route on
    // a network with no gateway.
    for target in [Ipv4Addr::new(192, 0, 2, 1), Ipv4Addr::new(224, 0, 0, 251)] {
        if socket.connect((target, 9)).is_ok() {
            let ip = socket.local_addr()?.ip();
            if !ip.is_loopback() && !ip.is_unspecified() {
                return Ok(ip);
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no network address",
    ))
}

/// A connection that opened while sharing was on stays open after `stop`:
/// this check closes it to the API and the UI.
pub(crate) async fn guard(
    State(share): State<Arc<Share>>,
    mut request: Request,
    next: Next,
) -> Response {
    // A request in process, as in the tests, carries no peer: it is local.
    let local = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_none_or(|ConnectInfo(peer)| peer.ip().to_canonical().is_loopback());
    if !local && !share.fixed() && share.opened().is_none() {
        return StatusCode::FORBIDDEN.into_response();
    }
    request.extensions_mut().insert(Local(local));
    next.run(request).await
}

pub(crate) async fn get(
    State(share): State<Arc<Share>>,
    Extension(local): Extension<Local>,
) -> Json<Value> {
    Json(share.state(local))
}

pub(crate) async fn set(
    State(share): State<Arc<Share>>,
    Extension(local): Extension<Local>,
    Json(body): Json<Value>,
) -> Response {
    if !local.0 {
        return refuse(
            StatusCode::FORBIDDEN,
            "only this computer can switch sharing",
        );
    }
    if share.fixed() {
        return refuse(
            StatusCode::CONFLICT,
            "--listen opens the server to the network already",
        );
    }
    let Some(on) = body.get("shared").and_then(Value::as_bool) else {
        return refuse(
            StatusCode::BAD_REQUEST,
            "expected { \"shared\": true | false }",
        );
    };
    if on {
        if let Err(error) = share.start() {
            return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error);
        }
    } else {
        share.stop();
    }
    let store = Arc::clone(&share.store);
    let saved = storage::blocking(move || store.update(|studio| studio.server.settings.share = on));
    if let Err(error) = saved.await {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    Json(share.state(local)).into_response()
}
