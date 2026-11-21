//! Discovery, the socket, the cache and the breaker, tied together.
//!
//! The send path resolves the device from what is already known, asks the
//! breaker, and writes. Neither step touches the socket, and **no scan is ever
//! issued for a command**. Verification follows on its own task and feeds the
//! breaker — the fire-and-verify of `docs/protocol/lan.md` 1.
//!
//! Replies carry no request id, and the source address is the only
//! correlation. Each device owns a [`tokio::sync::watch`] channel holding its
//! last status, so one reply wakes every waiter.

mod impl_transport;
mod inbound;
mod options;
mod shared;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};
use tokio::task::JoinSet;

use self::inbound::{receive_loop, refresh_loop};
pub use self::options::Options;
use self::shared::{Shared, datagram};
use crate::codec::{Encoded, Mode};
use crate::lan::discovery::{DiscoveredDevice, Endpoints};
use crate::lan::socket::Socket;
// Referenced from the doc comments below, nowhere else.
#[cfg(doc)]
use crate::transport::error::Error;
use crate::transport::error::Result;
use crate::transport::registry::{Devices, publish_sent};
use crate::transport::status::DeviceStatus;
use crate::transport::{DeviceId, Event, Health, KnownDevice, Sent, Verify};

/// How long one command waits behind the one before it, to one device.
///
/// A device can drop a command that arrives directly behind two others, and
/// nothing acknowledges a command — see `docs/protocol/lan.md` 1,
/// "Consecutive commands".
pub const COMMAND_GAP: Duration = Duration::from_millis(5);

/// What the refresh loop needs to start again after a close.
#[derive(Clone, Copy)]
struct Refresh {
    interval: Duration,
    window: Duration,
    forget_after: Duration,
}

/// The `lan` transport.
///
/// Cheap to clone; every clone shares one socket, one cache and one set of
/// breakers. The background tasks stop at [`Transport::close`] or with the
/// last clone: dropping the set aborts them.
#[derive(Clone)]
pub struct Transport {
    shared: Arc<Shared>,
    /// Also serializes a close against a reopen.
    tasks: Arc<tokio::sync::Mutex<JoinSet<()>>>,
    refresh: Option<Refresh>,
}

impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("endpoints", &self.shared.endpoints)
            .finish_non_exhaustive()
    }
}

impl Transport {
    /// Bind the socket and start the receive loop.
    ///
    /// Devices already in the cache are usable immediately: no scan is issued
    /// here, and none is needed before the first command. A cached device
    /// that has not answered a scan for [`Options::forget_after`] is dropped
    /// first.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the socket cannot be bound.
    // Nothing here awaits. `async` states that a runtime must exist: the
    // background tasks are spawned onto the current one.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    pub async fn start(options: Options) -> Result<Self> {
        let socket = Socket::bind(&options.endpoints)?;
        let (events, _) = broadcast::channel(256);
        let (replies, _) = broadcast::channel(256);

        let shared = Arc::new(Shared {
            socket,
            endpoints: options.endpoints,
            policy: options.policy,
            scan_window: options.scan_window,
            status_timeout: options.status_timeout,
            verify_interval: options.verify_interval,
            devices: Devices::new(),
            cache: Mutex::new(options.cache),
            events,
            replies,
        });

        // A short process selects before the first scan of the refresh loop.
        shared.forget_stale(options.forget_after);
        shared.adopt_cached_devices();

        let refresh = options.refresh_interval.map(|interval| Refresh {
            interval,
            window: options.scan_window,
            forget_after: options.forget_after,
        });
        let mut tasks = JoinSet::new();
        spawn_tasks(&shared, refresh, &mut tasks, true);

        Ok(Self {
            shared,
            tasks: Arc::new(tokio::sync::Mutex::new(tasks)),
            refresh,
        })
    }

    /// Bind again and restart the receive loop after [`Transport::close`].
    async fn reopen(&self) -> Result<()> {
        if self.shared.socket.is_open() {
            return Ok(());
        }
        let mut tasks = self.tasks.lock().await;
        if self.shared.socket.reopen(&self.shared.endpoints)? {
            spawn_tasks(&self.shared, self.refresh, &mut tasks, false);
        }
        Ok(())
    }

    /// Subscribe to transport events.
    #[must_use]
    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.shared.events.subscribe()
    }

    /// Where this transport sends and listens.
    #[must_use]
    pub fn endpoints(&self) -> Endpoints {
        self.shared.endpoints
    }

    /// The address the socket is bound to.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the socket cannot report it, or after
    /// [`Transport::close`] until the next scan, command or status request.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.shared.socket.local_addr()
    }

    /// What keeps this transport from hearing replies, one sentence each.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        if !self.shared.socket.held_elsewhere() {
            return Vec::new();
        }
        let port = self.shared.endpoints.reply_bind.port();
        vec![format!(
            "another socket listens on UDP port {port}: the replies can reach it instead, \
             so a scan or a status request can hear nothing. Stop the process that holds it, \
             such as a running MCP server or `govee-dmx`, or close the other SDK of this \
             process, and try again"
        )]
    }

    /// Stop the background tasks and free the reply port before this call
    /// returns.
    ///
    /// The next scan, command or status request binds the port again. The
    /// known devices and the cache stay.
    pub async fn close(&self) {
        let mut tasks = self.tasks.lock().await;
        tasks.shutdown().await;
        self.shared.socket.close();
    }

    /// How long a scan collects replies when the caller names no window.
    #[must_use]
    pub fn scan_window(&self) -> Duration {
        self.shared.scan_window
    }

    /// [`COMMAND_GAP`]: how long a command waits behind the one before it.
    #[must_use]
    pub fn command_gap(&self) -> Duration {
        COMMAND_GAP
    }

    /// Send a discovery request and collect replies for `window`.
    ///
    /// Devices found are recorded in the cache, which is written out if
    /// anything changed. Nothing on the send path calls this.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the request cannot be sent.
    pub async fn scan(&self, window: Duration) -> Result<Vec<DiscoveredDevice>> {
        self.reopen().await?;
        self.shared.scan(window).await
    }

    /// Send a discovery request and stop at the reply from one device.
    ///
    /// `window` is the longest this waits. Devices that answer before the one
    /// asked for are recorded too. Nothing on the send path calls this.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the request cannot be sent.
    pub async fn scan_for(
        &self,
        id: &DeviceId,
        window: Duration,
    ) -> Result<Option<DiscoveredDevice>> {
        self.reopen().await?;
        self.shared.scan_for(id, window).await
    }

    /// Every device the transport knows, from a scan or from the cache.
    #[must_use]
    pub fn devices(&self) -> Vec<KnownDevice> {
        let port = self.shared.endpoints.control_port;
        self.shared
            .devices
            .known(|tracked| SocketAddr::new(tracked.ip, port).to_string())
    }

    /// The SKU a device reports, if it is known.
    #[must_use]
    pub fn sku(&self, id: &DeviceId) -> Option<String> {
        self.shared.devices.sku(id)
    }

    /// A device's health in this mode, if it is known.
    #[must_use]
    pub fn health(&self, id: &DeviceId) -> Option<Health> {
        self.shared.devices.health(id)
    }

    /// Watch a device's status as replies arrive.
    ///
    /// The value starts at whatever was last heard — `None` if nothing has
    /// been. Nothing is requested by subscribing; use [`Transport::status`] for
    /// that.
    #[must_use]
    pub fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        self.shared.devices.watch_status(id)
    }

    /// The last status heard from a device, without asking for a new one.
    #[must_use]
    pub fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus> {
        self.shared.devices.last_status(id)
    }

    /// Write a command to the socket.
    ///
    /// Returns as soon as the datagram is out. The protocol acknowledges
    /// nothing, so a successful return means the command was sent, never that
    /// it was applied — which is exactly what [`Verify::With`] is for.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`] if nothing has been discovered under this
    /// identity, [`Error::Unavailable`] if the breaker refuses this mode right
    /// now — decided from recorded state, without touching the network — or
    /// [`Error::Io`] if the write fails.
    pub async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify) -> Result<Sent> {
        let now = Instant::now();
        let (addr, verifying) =
            self.shared
                .route_and_claim(id, now, matches!(&verify, Verify::With(_)))?;
        let bytes = datagram(command)?;

        self.reopen().await?;
        self.shared.socket.send_to(&bytes, addr).await?;

        let sent = Sent {
            id: id.clone(),
            mode: Mode::Lan,
            cmd: command.cmd.clone(),
            endpoint: addr.to_string(),
        };
        publish_sent(&self.shared.events, &sent);

        if let Verify::With(request) = verify
            && verifying
        {
            let shared = Arc::clone(&self.shared);
            let id = id.clone();
            let timeout = self.shared.status_timeout;
            tokio::spawn(async move {
                // The result is already recorded against the breaker and
                // published as an event; nothing here needs it.
                let _ = shared.request_status(&id, &request, timeout).await;
            });
        }

        Ok(sent)
    }

    /// Ask a device for its status and wait for the answer.
    ///
    /// The answer, or the silence, is recorded against the breaker. Concurrent
    /// callers share one request: the reply wakes all of them.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownDevice`], [`Error::Unavailable`] or [`Error::Io`] as for
    /// [`Transport::send`], and [`Error::Unreachable`] if nothing answers in
    /// time.
    pub async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        self.reopen().await?;
        self.shared
            .request_status(id, request, self.shared.status_timeout)
            .await
    }

    /// Write the device cache out.
    ///
    /// # Errors
    ///
    /// [`Error::Cache`] if the file cannot be written.
    pub fn save_cache(&self) -> Result<()> {
        let cache = {
            let Ok(cache) = self.shared.cache.lock() else {
                return Ok(());
            };
            cache.clone()
        };
        cache.save()
    }
}

fn spawn_tasks(
    shared: &Arc<Shared>,
    refresh: Option<Refresh>,
    tasks: &mut JoinSet<()>,
    scan_now: bool,
) {
    tasks.spawn(receive_loop(Arc::clone(shared)));
    if let Some(refresh) = refresh {
        tasks.spawn(refresh_loop(
            Arc::clone(shared),
            refresh.interval,
            refresh.window,
            refresh.forget_after,
            scan_now,
        ));
    }
}
