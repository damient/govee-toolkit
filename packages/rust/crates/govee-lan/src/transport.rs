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

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use govee_core::{Encoded, Mode};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use crate::DeviceId;
use crate::breaker::{Breaker, Policy, State, Transition};
use crate::cache::{Cache, Change};
use crate::discovery::{DiscoveredDevice, Endpoints, scan_request};
use crate::error::{Error, Result};
use crate::socket::{MAX_DATAGRAM, Socket, parse_reply};
use crate::status::DeviceStatus;

/// How the transport is set up.
#[derive(Debug, Clone)]
pub struct Options {
    /// Where to send and listen. The defaults are the protocol's own ports.
    pub endpoints: Endpoints,
    /// Circuit breaker thresholds.
    pub policy: Policy,
    /// The device cache. [`Cache::in_memory`] by default — a caller that wants
    /// discovery to survive a restart passes [`Cache::load`].
    pub cache: Cache,
    /// How long a scan collects replies before returning.
    pub scan_window: Duration,
    /// How often to rescan in the background. `None` scans only when asked.
    pub refresh_interval: Option<Duration>,
    /// How long a status request waits for its answer.
    pub status_timeout: Duration,
    /// The shortest interval between two verifications of the same device.
    /// `None` disables verification: the breaker then learns nothing, which is
    /// what a caller streaming frames wants.
    pub verify_interval: Option<Duration>,
    /// Drop a cached device that has not answered a scan for this long.
    pub forget_after: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            endpoints: Endpoints::default(),
            policy: Policy::default(),
            cache: Cache::in_memory(),
            scan_window: Duration::from_secs(2),
            refresh_interval: Some(Duration::from_secs(60)),
            // The idle round-trip measured on real hardware is tens of
            // milliseconds (`devices/H61A0.yaml`); half a second is silence,
            // not slowness.
            status_timeout: Duration::from_millis(500),
            verify_interval: Some(Duration::from_secs(1)),
            forget_after: Duration::from_secs(7 * 24 * 3600),
        }
    }
}

/// What to do about a command once it has been written to the socket.
#[derive(Debug, Clone, Copy)]
pub enum Verify<'a> {
    /// Nothing. The breaker learns nothing from this command — right for a
    /// stream of frames, where the verification traffic would compete with the
    /// frames themselves.
    None,
    /// Ask the device for its status afterwards, and record the answer, or its
    /// absence, against the breaker. The request is supplied by the caller
    /// because building it means reading the device file, which is the codec's
    /// job and not this crate's.
    With(&'a Encoded),
}

/// A command that was written to the socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    /// Which device it went to.
    pub id: DeviceId,
    /// Which mode served it. Always `lan` here; carried so that a caller
    /// reporting "which mode served this command" reads the same whatever the
    /// transport was.
    pub mode: Mode,
    /// The `msg.cmd` that went out.
    pub cmd: String,
    /// Where it went.
    pub addr: SocketAddr,
}

/// A device's health in this mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// Breaker state.
    pub state: State,
    /// Consecutive unanswered verifications.
    pub failures: u32,
    /// Whether a command would be sent right now.
    pub available: bool,
}

/// A device the transport can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownDevice {
    /// Its identity.
    pub id: DeviceId,
    /// Its last known address.
    pub ip: IpAddr,
    /// The SKU it reports.
    pub sku: String,
    /// Its health.
    pub health: Health,
}

/// Something worth telling the application about.
///
/// `docs/modes.md` requires every mode transition to be subscribable; the rest
/// is here because an application that shows devices needs it and polling for
/// it would be worse.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// A device answered a scan.
    Discovered {
        /// What it reported.
        device: DiscoveredDevice,
        /// Whether it is new, has moved, or has been updated.
        change: Change,
    },
    /// A cached device has not answered a scan for [`Options::forget_after`].
    Forgotten {
        /// The device that was dropped.
        id: DeviceId,
    },
    /// A command was written to the socket.
    Sent(Sent),
    /// A device reported its state.
    Status(DeviceStatus),
    /// A device's health in this mode changed.
    HealthChanged {
        /// The device.
        id: DeviceId,
        /// The mode, always `lan` here.
        mode: Mode,
        /// What it moved from and to.
        transition: Transition,
    },
}

/// One device, as the transport tracks it.
struct Tracked {
    ip: IpAddr,
    sku: String,
    breaker: Breaker,
    status: watch::Sender<Option<DeviceStatus>>,
    /// A status request is out; another waiter should listen rather than send.
    probing: bool,
    /// When verification last ran, so a burst of commands does not turn into a
    /// burst of probes.
    verified_at: Option<Instant>,
}

struct Shared {
    socket: Socket,
    endpoints: Endpoints,
    policy: Policy,
    status_timeout: Duration,
    verify_interval: Option<Duration>,
    devices: Mutex<HashMap<DeviceId, Tracked>>,
    by_address: Mutex<HashMap<IpAddr, DeviceId>>,
    cache: Mutex<Cache>,
    events: broadcast::Sender<Event>,
    replies: broadcast::Sender<DiscoveredDevice>,
}

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

impl Shared {
    /// Make the cached devices usable before any scan has run.
    fn adopt_cached_devices(&self) {
        let Ok(cache) = self.cache.lock() else {
            return;
        };
        let (Ok(mut devices), Ok(mut by_address)) = (self.devices.lock(), self.by_address.lock())
        else {
            return;
        };
        for cached in cache.devices() {
            by_address.insert(cached.ip, cached.id.clone());
            devices.insert(
                cached.id.clone(),
                Tracked {
                    ip: cached.ip,
                    sku: cached.sku.clone(),
                    breaker: Breaker::new(self.policy),
                    status: watch::Sender::new(None),
                    probing: false,
                    verified_at: None,
                },
            );
        }
    }

    /// Where to send, if the breaker allows it.
    ///
    /// Both answers come from memory. Nothing here waits on the network — that
    /// is the rule this whole design exists for.
    fn route(&self, id: &DeviceId, now: Instant) -> Result<SocketAddr> {
        let devices = self.devices.lock().map_err(|_| Error::ShutDown)?;
        let tracked = devices
            .get(id)
            .ok_or_else(|| Error::UnknownDevice { id: id.clone() })?;
        if !tracked.breaker.allows(now) {
            return Err(Error::Unavailable {
                id: id.clone(),
                state: tracked.breaker.state(),
            });
        }
        Ok(SocketAddr::new(tracked.ip, self.endpoints.control_port))
    }

    /// Whether this command should pay for a verification.
    fn claim_verification(&self, id: &DeviceId, now: Instant) -> bool {
        let Some(interval) = self.verify_interval else {
            return false;
        };
        let Ok(mut devices) = self.devices.lock() else {
            return false;
        };
        let Some(tracked) = devices.get_mut(id) else {
            return false;
        };
        if tracked
            .verified_at
            .is_some_and(|at| now.duration_since(at) < interval)
        {
            return false;
        }
        tracked.verified_at = Some(now);
        true
    }

    async fn request_status(
        &self,
        id: &DeviceId,
        request: &Encoded,
        timeout: Duration,
    ) -> Result<DeviceStatus> {
        let now = Instant::now();
        let addr = self.route(id, now)?;

        // Subscribe before sending, or a reply that arrives first is missed.
        let (mut watcher, send_it) = {
            let mut devices = self.devices.lock().map_err(|_| Error::ShutDown)?;
            let tracked = devices
                .get_mut(id)
                .ok_or_else(|| Error::UnknownDevice { id: id.clone() })?;
            let watcher = tracked.status.subscribe();
            let send_it = !tracked.probing;
            tracked.probing = true;
            (watcher, send_it)
        };
        watcher.mark_unchanged();

        if send_it {
            let bytes = datagram(request)?;
            if let Err(e) = self.socket.send_to(&bytes, addr).await {
                self.clear_probe(id);
                return Err(e);
            }
        }

        let outcome = tokio::time::timeout(timeout, watcher.changed()).await;
        self.clear_probe(id);

        match outcome {
            Ok(Ok(())) => {
                let status = watcher.borrow_and_update().clone();
                self.record(id, true, Instant::now());
                status.ok_or_else(|| Error::Unreachable {
                    id: id.clone(),
                    addr,
                    timeout_ms: to_millis(timeout),
                })
            }
            // The device is gone from the map: nothing left to wait on.
            Ok(Err(_)) => Err(Error::UnknownDevice { id: id.clone() }),
            Err(_elapsed) => {
                self.record(id, false, Instant::now());
                Err(Error::Unreachable {
                    id: id.clone(),
                    addr,
                    timeout_ms: to_millis(timeout),
                })
            }
        }
    }

    fn clear_probe(&self, id: &DeviceId) {
        if let Ok(mut devices) = self.devices.lock()
            && let Some(tracked) = devices.get_mut(id)
        {
            tracked.probing = false;
        }
    }

    /// Feed the breaker and publish the transition, if there was one.
    fn record(&self, id: &DeviceId, answered: bool, now: Instant) {
        let transition = {
            let Ok(mut devices) = self.devices.lock() else {
                return;
            };
            let Some(tracked) = devices.get_mut(id) else {
                return;
            };
            if answered {
                tracked.breaker.record_success(now)
            } else {
                tracked.breaker.record_failure(now)
            }
        };
        if transition.changed() {
            tracing::info!(%id, from = %transition.from, to = %transition.to, "lan health changed");
            let _ = self.events.send(Event::HealthChanged {
                id: id.clone(),
                mode: Mode::Lan,
                transition,
            });
        }
    }

    async fn scan(&self, window: Duration) -> Result<Vec<DiscoveredDevice>> {
        let mut replies = self.replies.subscribe();
        self.socket
            .send_to(&scan_request(), self.endpoints.scan_target)
            .await?;

        let deadline = tokio::time::Instant::now() + window;
        let mut found: std::collections::BTreeMap<DeviceId, DiscoveredDevice> =
            std::collections::BTreeMap::new();
        loop {
            match tokio::time::timeout_at(deadline, replies.recv()).await {
                Ok(Ok(device)) => {
                    found.insert(device.id.clone(), device);
                }
                // A slow reader missing replies is not a failed scan; the next
                // one will see the device again.
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {}
                Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
            }
        }

        if !found.is_empty() {
            self.persist_cache();
        }
        Ok(found.into_values().collect())
    }

    /// Write the cache out without blocking the runtime.
    fn persist_cache(&self) {
        let Ok(cache) = self.cache.lock() else {
            return;
        };
        if cache.path().is_none() {
            return;
        }
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = cache.save() {
                tracing::warn!(error = %e, "could not write the device cache");
            }
        });
    }

    /// Route one datagram.
    ///
    /// Dispatch is by shape, not by command name: a payload carrying an
    /// identity, an address and a SKU is a discovery reply, and anything else
    /// from a device already known is a status. Both the documented
    /// `devStatus` and the undocumented `status` of `docs/protocol/lan.md` §2.2
    /// therefore land in the right place without this crate holding a list of
    /// command names.
    fn dispatch(&self, from: SocketAddr, bytes: &[u8]) {
        let Some(reply) = parse_reply(from, bytes) else {
            return;
        };

        if let Some(device) = DiscoveredDevice::from_data(&reply.data) {
            self.register(&device);
            let _ = self.replies.send(device);
            return;
        }

        if !reply.data.is_object() {
            return;
        }
        let Some(id) = self.identify(from.ip()) else {
            tracing::debug!(%from, cmd = %reply.cmd, "reply from an address no device is known at");
            return;
        };

        let status = DeviceStatus::from_data(id, &reply.data);
        if let Ok(devices) = self.devices.lock()
            && let Some(tracked) = devices.get(&status.id)
        {
            let _ = tracked.status.send(Some(status.clone()));
        }
        let _ = self.events.send(Event::Status(status));
    }

    fn identify(&self, ip: IpAddr) -> Option<DeviceId> {
        let by_address = self.by_address.lock().ok()?;
        by_address.get(&ip).cloned()
    }

    /// Record a discovery reply and make the device sendable.
    fn register(&self, device: &DiscoveredDevice) {
        let change = {
            let Ok(mut cache) = self.cache.lock() else {
                return;
            };
            cache.record(device, SystemTime::now())
        };

        {
            let (Ok(mut devices), Ok(mut by_address)) =
                (self.devices.lock(), self.by_address.lock())
            else {
                return;
            };
            match devices.get_mut(&device.id) {
                Some(tracked) => {
                    if tracked.ip != device.ip {
                        by_address.remove(&tracked.ip);
                        tracked.ip = device.ip;
                    }
                    tracked.sku.clone_from(&device.sku);
                }
                None => {
                    devices.insert(
                        device.id.clone(),
                        Tracked {
                            ip: device.ip,
                            sku: device.sku.clone(),
                            breaker: Breaker::new(self.policy),
                            status: watch::Sender::new(None),
                            probing: false,
                            verified_at: None,
                        },
                    );
                }
            }
            by_address.insert(device.ip, device.id.clone());
        }

        if change != Change::Refreshed {
            tracing::info!(id = %device.id, ip = %device.ip, sku = %device.sku, ?change, "device discovered");
        }
        let _ = self.events.send(Event::Discovered {
            device: device.clone(),
            change,
        });
    }

    /// Drop cached devices that have stopped answering scans.
    fn forget_stale(&self, older_than: Duration) {
        let dropped = {
            let Ok(mut cache) = self.cache.lock() else {
                return;
            };
            cache.prune(SystemTime::now(), older_than)
        };
        if dropped.is_empty() {
            return;
        }
        if let (Ok(mut devices), Ok(mut by_address)) = (self.devices.lock(), self.by_address.lock())
        {
            for device in &dropped {
                devices.remove(&device.id);
                by_address.remove(&device.ip);
            }
        }
        for device in dropped {
            let _ = self.events.send(Event::Forgotten { id: device.id });
        }
        self.persist_cache();
    }
}

fn health_of(breaker: &Breaker, now: Instant) -> Health {
    Health {
        state: breaker.state(),
        failures: breaker.failures(),
        available: breaker.allows(now),
    }
}

fn datagram(command: &Encoded) -> Result<Vec<u8>> {
    command.to_bytes().map_err(|e| Error::Serialize {
        cmd: command.cmd.clone(),
        reason: e.to_string(),
    })
}

fn to_millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

async fn receive_loop(shared: Arc<Shared>) {
    let mut buf = vec![0u8; MAX_DATAGRAM];
    loop {
        match shared.socket.recv_from(&mut buf).await {
            Ok((read, from)) => {
                if let Some(bytes) = buf.get(..read) {
                    shared.dispatch(from, bytes);
                } else {
                    tracing::warn!(read, "a datagram larger than the buffer was truncated");
                }
            }
            Err(e) => {
                // A receive error here is the socket's, not a device's, and
                // retrying immediately would spin. Slow down and keep going:
                // the transport surviving a transient error matters more than
                // reporting it.
                tracing::warn!(error = %e, "the lan receive loop failed");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

async fn refresh_loop(
    shared: Arc<Shared>,
    interval: Duration,
    window: Duration,
    forget_after: Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick fires immediately: the startup scan is this one, so a
    // caller does not have to remember to issue it.
    loop {
        ticker.tick().await;
        if let Err(e) = shared.scan(window).await {
            tracing::warn!(error = %e, "background scan failed");
        }
        shared.forget_stale(forget_after);
    }
}
