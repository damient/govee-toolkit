//! Discovery, the socket, the cache and the breaker, tied together.
//!
//! The send path:
//!
//! 1. resolve the device from what is already known — cache or a past scan,
//!    **never** a scan issued for this command;
//! 2. ask the breaker, which answers from recorded state and touches no socket;
//! 3. write the datagram and return.
//!
//! Verification runs after the fact, on its own task: a `devStatus` request
//! whose answer — or absence — feeds the breaker. This is the fire-and-verify
//! of `docs/protocol/lan.md` §1, and it keeps a round-trip off the send path.
//!
//! Replies carry no request id. The source address is the only correlation
//! there is, so each device owns a [`tokio::sync::watch`] channel holding its
//! last known status: a caller waiting for one waits for that value to change,
//! and every waiter is woken by the same reply.

mod impl_transport;
mod inbound;
mod options;
mod shared;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

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

/// Background tasks, stopped when the last [`Transport`] handle goes away.
struct Tasks(Vec<JoinHandle<()>>);

impl Drop for Tasks {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

/// The `lan` transport.
///
/// Cheap to clone; every clone shares one socket, one cache and one set of
/// breakers. The background tasks stop when the last clone is dropped.
#[derive(Clone)]
pub struct Transport {
    shared: Arc<Shared>,
    _tasks: Arc<Tasks>,
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
    /// here, and none is needed before the first command.
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
            status_timeout: options.status_timeout,
            verify_interval: options.verify_interval,
            devices: Devices::new(),
            cache: Mutex::new(options.cache),
            events,
            replies,
        });

        shared.adopt_cached_devices();

        let mut tasks = vec![tokio::spawn(receive_loop(Arc::clone(&shared)))];
        if let Some(interval) = options.refresh_interval {
            tasks.push(tokio::spawn(refresh_loop(
                Arc::clone(&shared),
                interval,
                options.scan_window,
                options.forget_after,
            )));
        }

        Ok(Self {
            shared,
            _tasks: Arc::new(Tasks(tasks)),
        })
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
    /// [`Error::Io`] if the socket cannot report it.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.shared.socket.local_addr()
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
        self.shared.scan(window).await
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
