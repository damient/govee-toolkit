//! Discovery, the socket, the cache and the breaker, tied together.
//!
//! The shape of the send path is the whole point of this module:
//!
//! 1. resolve the device from what is already known — cache or a past scan,
//!    **never** a scan issued for this command;
//! 2. ask the breaker, which answers from recorded state and touches no socket;
//! 3. write the datagram and return.
//!
//! Verification happens after the fact, on its own task: a `devStatus` request
//! whose answer — or absence — is what feeds the breaker. That is the
//! fire-and-verify of `docs/protocol/lan.md` §1, and it is why a command does
//! not pay for a round-trip it does not need.
//!
//! Replies carry no request id. The source address is the only correlation
//! there is, so each device owns a [`tokio::sync::watch`] channel holding its
//! last known status: a caller waiting for one waits for that value to change,
//! and every waiter is woken by the same reply.

mod events;
mod inbound;
mod options;
mod shared;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use self::events::health_of;
pub use self::events::{Event, Health, KnownDevice, Sent};
use self::inbound::{receive_loop, refresh_loop};
pub use self::options::{Options, Verify};
use self::shared::{Shared, datagram};
use crate::codec::{Encoded, Mode};
use crate::lan::DeviceId;
use crate::lan::discovery::DiscoveredDevice;
// Referenced from the doc comments below, nowhere else.
#[cfg(doc)]
use crate::lan::error::Error;
use crate::lan::error::Result;
use crate::lan::socket::Socket;
use crate::lan::status::DeviceStatus;

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
    // Nothing here awaits, but the background tasks are spawned onto the
    // current runtime: `async` is what states that there has to be one.
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
            devices: Mutex::new(HashMap::new()),
            by_address: Mutex::new(HashMap::new()),
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
        let now = Instant::now();
        let Ok(devices) = self.shared.devices.lock() else {
            return Vec::new();
        };
        let mut out: Vec<KnownDevice> = devices
            .iter()
            .map(|(id, tracked)| KnownDevice {
                id: id.clone(),
                ip: tracked.ip,
                sku: tracked.sku.clone(),
                health: health_of(&tracked.breaker, now),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The SKU a device reports, if it is known.
    #[must_use]
    pub fn sku(&self, id: &DeviceId) -> Option<String> {
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).map(|d| d.sku.clone())
    }

    /// A device's health in this mode, if it is known.
    #[must_use]
    pub fn health(&self, id: &DeviceId) -> Option<Health> {
        let now = Instant::now();
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).map(|d| health_of(&d.breaker, now))
    }

    /// Watch a device's status as replies arrive.
    ///
    /// The value starts at whatever was last heard — `None` if nothing has
    /// been. Nothing is requested by subscribing; use [`Transport::status`] for
    /// that.
    #[must_use]
    pub fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).map(|d| d.status.subscribe())
    }

    /// The last status heard from a device, without asking for a new one.
    #[must_use]
    pub fn last_status(&self, id: &DeviceId) -> Option<DeviceStatus> {
        let devices = self.shared.devices.lock().ok()?;
        devices.get(id).and_then(|d| d.status.borrow().clone())
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
    pub async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify<'_>) -> Result<Sent> {
        let now = Instant::now();
        let addr = self.shared.route(id, now)?;
        let bytes = datagram(command)?;

        self.shared.socket.send_to(&bytes, addr).await?;

        let sent = Sent {
            id: id.clone(),
            mode: Mode::Lan,
            cmd: command.cmd.clone(),
            addr,
        };
        let _ = self.shared.events.send(Event::Sent(sent.clone()));

        if let Verify::With(request) = verify
            && self.shared.claim_verification(id, now)
        {
            let shared = Arc::clone(&self.shared);
            let id = id.clone();
            let request = request.clone();
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
