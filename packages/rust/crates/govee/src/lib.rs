//! The govee-toolkit facade: configuration, mode selection, events.
//!
//! The layer an application talks to. It reads the user's configuration, holds
//! the catalog and the transports, and decides — for each command — which of
//! the modes the user enabled serves it.
//!
//! What "decides" means here is narrow, and deliberately so
//! (`docs/modes.md`):
//!
//! - The mode is picked from **breaker state already known**, before anything
//!   is encoded. It is never picked by trying one and waiting for a timeout.
//! - A device with one enabled mode is reached over that mode or not at all.
//!   Unreachable is an error, never a quiet switch to something else.
//! - A command the chosen mode does not carry **fails**. It is not
//!   approximated with a command that mode does have.
//! - Every command reports which mode served it, and every health transition is
//!   an event.
//!
//! ```no_run
//! use govee::{Config, Govee};
//! use govee_core::Args;
//!
//! # async fn example() -> Result<(), govee::Error> {
//! let govee = Govee::start(Config::load()?).await?;
//! govee.scan().await?;
//!
//! for device in govee.devices() {
//!     let served = govee
//!         .device(&device.id)
//!         .send("power", &Args::new().int("on", 1))
//!         .await?;
//!     println!("{} served by {}", device.id, served.mode);
//! }
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod error;
pub mod paths;

pub use config::{Config, DeviceConfig, LanConfig, Problem};
pub use error::{Error, Result};
pub use govee_core::{Args, Catalog, Mode};
pub use govee_lan::{DeviceId, DeviceStatus, Health, State};

use std::sync::Arc;
use std::time::Duration;

use govee_lan::Transport;
use tokio::sync::broadcast;

/// Something worth telling the application about.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// Something the `lan` transport reported: a discovery, a status, a health
    /// transition. Health transitions carry the mode they are about, so an
    /// application subscribes once and does not care how many transports exist.
    Lan(govee_lan::Event),
    /// A device answered with a SKU the catalog does not know, so nothing can
    /// be encoded for it. Pin a known SKU in the configuration, or add a device
    /// file — see `devices/README.md`.
    UnknownSku {
        /// The device.
        id: DeviceId,
        /// What it reported.
        sku: String,
    },
}

/// A command that was served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Served {
    /// The device it went to.
    pub id: DeviceId,
    /// The mode that served it. The whole point of returning this: with several
    /// modes enabled, which one ran is not something a caller should guess.
    pub mode: Mode,
    /// The device file entry that was sent.
    pub command: String,
    /// The `msg.cmd` that went on the wire.
    pub cmd: String,
}

/// A device the facade knows about.
#[derive(Debug, Clone)]
pub struct Device {
    /// Its identity.
    pub id: DeviceId,
    /// The SKU it will be encoded under: the one the configuration pins, or the
    /// one it reports.
    pub sku: String,
    /// The name the configuration gives it, if any.
    pub name: Option<String>,
    /// The enabled modes, in preference order.
    pub modes: Vec<Mode>,
    /// Its health in `lan`, if `lan` is one of them.
    pub lan_health: Option<Health>,
}

struct Inner {
    catalog: Catalog,
    config: Config,
    lan: Transport,
    events: broadcast::Sender<Event>,
}

/// The SDK.
///
/// Cheap to clone; every clone shares one catalog, one configuration and one
/// set of transports.
#[derive(Clone)]
pub struct Govee {
    inner: Arc<Inner>,
    _forwarder: Arc<Forwarder>,
}

struct Forwarder(tokio::task::JoinHandle<()>);

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl std::fmt::Debug for Govee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Govee")
            .field("devices", &self.inner.lan.devices().len())
            .finish_non_exhaustive()
    }
}

impl Govee {
    /// Start with the embedded catalog, plus the user's own device files if the
    /// configuration opts into them.
    ///
    /// # Errors
    ///
    /// [`Error::Codec`] if a device file does not parse, [`Error::LocalDevices`]
    /// if the local directory cannot be read, [`Error::Configuration`] if the
    /// configuration cannot be applied, or [`Error::Transport`] if the socket
    /// cannot be bound.
    pub async fn start(config: Config) -> Result<Self> {
        let mut catalog = Catalog::embedded()?;
        if config.catalog.local_devices {
            overlay_local_devices(&mut catalog, &config)?;
        }
        Self::start_with(config, catalog).await
    }

    /// Start with a catalog the caller built.
    ///
    /// # Errors
    ///
    /// See [`Govee::start`].
    pub async fn start_with(config: Config, catalog: Catalog) -> Result<Self> {
        let transport = Transport::start(config.lan.transport_options()?).await?;
        Self::attach(config, catalog, transport)
    }

    /// Use a transport the caller already built.
    ///
    /// The seam for anything that has to bind its own sockets: a test against
    /// the simulator, or a host embedding the SDK beside other UDP traffic.
    ///
    /// # Errors
    ///
    /// [`Error::Configuration`] if the configuration cannot be applied.
    pub fn attach(config: Config, catalog: Catalog, transport: Transport) -> Result<Self> {
        let problems = config.problems();
        if !problems.is_empty() {
            return Err(Error::Configuration(problems));
        }

        let (events, _) = broadcast::channel(256);
        let inner = Arc::new(Inner {
            catalog,
            config,
            lan: transport,
            events: events.clone(),
        });

        let forwarder = tokio::spawn(forward(Arc::clone(&inner), events));
        let govee = Self {
            inner,
            _forwarder: Arc::new(Forwarder(forwarder)),
        };

        // Whatever the cache already holds can be checked against the device
        // files right away, without waiting for a scan.
        let problems = govee.check_devices();
        for problem in &problems {
            tracing::warn!(%problem, "configuration");
        }
        Ok(govee)
    }

    /// Subscribe to events.
    #[must_use]
    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    /// The configuration in force.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// The device catalog in force.
    #[must_use]
    pub fn catalog(&self) -> &Catalog {
        &self.inner.catalog
    }

    /// Run a discovery scan and return what answered.
    ///
    /// Nothing on the send path calls this: it runs at startup and on the
    /// background interval. See `docs/protocol/lan.md` §1, latency notes.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if the request cannot be sent.
    pub async fn scan(&self) -> Result<Vec<Device>> {
        let window = Duration::from_millis(self.inner.config.lan.scan_window_ms);
        let found = self.inner.lan.scan(window).await?;
        Ok(found.iter().map(|d| self.describe(&d.id, &d.sku)).collect())
    }

    /// Every device known, from discovery or from the cache.
    #[must_use]
    pub fn devices(&self) -> Vec<Device> {
        self.inner
            .lan
            .devices()
            .iter()
            .map(|known| self.describe(&known.id, &known.sku))
            .collect()
    }

    /// A handle for one device.
    #[must_use]
    pub fn device(&self, id: &DeviceId) -> DeviceHandle<'_> {
        DeviceHandle {
            govee: self,
            id: id.clone(),
        }
    }

    /// Everything wrong with the configuration, including what could only be
    /// checked once devices were known.
    #[must_use]
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = self.inner.config.problems();
        problems.extend(self.check_devices());
        problems
    }

    /// Check every known device's enabled modes against what its device file
    /// says the hardware supports.
    fn check_devices(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for known in self.inner.lan.devices() {
            let sku = self.sku_of(&known.id, &known.sku);
            let Ok(device) = self.inner.catalog.device(&sku) else {
                problems.push(Problem {
                    device: Some(known.id.clone()),
                    message: format!("reports SKU `{sku}`, which no device file declares"),
                });
                continue;
            };
            for mode in self.inner.config.modes_for(&known.id) {
                if device.modes.get(*mode).support == govee_core::Support::None {
                    problems.push(Problem {
                        device: Some(known.id.clone()),
                        message: format!("`{mode}` is enabled but {sku} does not support it"),
                    });
                }
            }
        }
        problems
    }

    fn describe(&self, id: &DeviceId, reported: &str) -> Device {
        Device {
            id: id.clone(),
            sku: self.sku_of(id, reported),
            name: self.inner.config.name_for(id).map(ToOwned::to_owned),
            modes: self.inner.config.modes_for(id).to_vec(),
            lan_health: self.inner.lan.health(id),
        }
    }

    fn sku_of(&self, id: &DeviceId, reported: &str) -> String {
        self.inner.config.sku_for(id).unwrap_or(reported).to_owned()
    }

    /// The first enabled mode the device can be reached over right now.
    ///
    /// Decided entirely from recorded state. Nothing here touches a socket,
    /// which is the rule the whole design exists for: choosing a mode by
    /// trying one would cost the fast path a round-trip on every command.
    fn choose(&self, id: &DeviceId) -> Result<Mode> {
        let modes = self.inner.config.modes_for(id);
        for &mode in modes {
            match mode {
                Mode::Lan => match self.inner.lan.health(id) {
                    Some(health) if health.available => return Ok(mode),
                    Some(_) => {}
                    // The transport has never seen it. Nothing to choose, and
                    // scanning now is exactly what must not happen.
                    None => {
                        return Err(Error::Transport(govee_lan::Error::UnknownDevice {
                            id: id.clone(),
                        }));
                    }
                },
                // Reached only because a preferred mode was unavailable, which
                // is precisely when substituting silently would be wrong.
                Mode::Ble | Mode::Cloud => {
                    return Err(Error::ModeNotImplemented {
                        id: id.clone(),
                        mode,
                    });
                }
            }
        }
        Err(Error::NoModeAvailable {
            id: id.clone(),
            modes: modes.to_vec(),
        })
    }

    /// Encode one command for a device, in one mode.
    fn encode(
        &self,
        id: &DeviceId,
        mode: Mode,
        command: &str,
        args: &Args,
    ) -> Result<govee_core::Encoded> {
        let sku = match self.inner.lan.sku(id) {
            Some(reported) => self.sku_of(id, &reported),
            None => self
                .inner
                .config
                .sku_for(id)
                .ok_or_else(|| govee_lan::Error::UnknownDevice { id: id.clone() })?
                .to_owned(),
        };
        let device = self.inner.catalog.device(&sku)?;
        Ok(govee_core::encode(device, mode, command, args)?)
    }

    /// The status request for a device, built from its device file.
    fn status_request(&self, id: &DeviceId, mode: Mode) -> Result<govee_core::Encoded> {
        self.encode(
            id,
            mode,
            &self.inner.config.lan.status_command,
            &Args::new(),
        )
    }
}

/// One device, bound to the SDK that reaches it.
#[derive(Debug, Clone)]
pub struct DeviceHandle<'a> {
    govee: &'a Govee,
    id: DeviceId,
}

impl DeviceHandle<'_> {
    /// The device's identity.
    #[must_use]
    pub fn id(&self) -> &DeviceId {
        &self.id
    }

    /// The modes enabled for it, in preference order.
    #[must_use]
    pub fn modes(&self) -> &[Mode] {
        self.govee.inner.config.modes_for(&self.id)
    }

    /// Its health in one mode, if that mode is known to a transport.
    #[must_use]
    pub fn health(&self, mode: Mode) -> Option<Health> {
        match mode {
            Mode::Lan => self.govee.inner.lan.health(&self.id),
            Mode::Ble | Mode::Cloud => None,
        }
    }

    /// Send a command, named as the device file names it.
    ///
    /// The mode is chosen first, then the command is encoded **for that mode**.
    /// A command the chosen mode does not carry fails with
    /// [`govee_core::Error::UnknownCommand`] rather than being approximated —
    /// `docs/modes.md`, capability differences between modes.
    ///
    /// # Errors
    ///
    /// [`Error::NoModeAvailable`] or [`Error::ModeNotImplemented`] if no
    /// enabled mode can serve it, [`Error::Codec`] if the command or its
    /// arguments are not valid for this device, [`Error::Transport`] if the
    /// write fails.
    pub async fn send(&self, command: &str, args: &Args) -> Result<Served> {
        let mode = self.govee.choose(&self.id)?;
        let encoded = self.govee.encode(&self.id, mode, command, args)?;

        // Fire-and-verify needs a request to verify with. A device file that
        // declares no status command simply is not verified — the command still
        // goes out.
        let verification = self.govee.status_request(&self.id, mode).ok();
        let verify = verification
            .as_ref()
            .map_or(govee_lan::Verify::None, govee_lan::Verify::With);

        let sent = match mode {
            Mode::Lan => {
                self.govee
                    .inner
                    .lan
                    .send(&self.id, &encoded, verify)
                    .await?
            }
            Mode::Ble | Mode::Cloud => {
                return Err(Error::ModeNotImplemented {
                    id: self.id.clone(),
                    mode,
                });
            }
        };

        Ok(Served {
            id: sent.id,
            mode: sent.mode,
            command: command.to_owned(),
            cmd: sent.cmd,
        })
    }

    /// Ask the device for its state and wait for the answer.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus [`govee_lan::Error::Unreachable`] if
    /// nothing answers in time.
    pub async fn status(&self) -> Result<DeviceStatus> {
        let mode = self.govee.choose(&self.id)?;
        let request = self.govee.status_request(&self.id, mode)?;
        match mode {
            Mode::Lan => Ok(self.govee.inner.lan.status(&self.id, &request).await?),
            Mode::Ble | Mode::Cloud => Err(Error::ModeNotImplemented {
                id: self.id.clone(),
                mode,
            }),
        }
    }

    /// The last status heard, without asking for a new one.
    #[must_use]
    pub fn last_status(&self) -> Option<DeviceStatus> {
        self.govee.inner.lan.last_status(&self.id)
    }

    /// Watch this device's status as replies arrive.
    #[must_use]
    pub fn watch_status(&self) -> Option<tokio::sync::watch::Receiver<Option<DeviceStatus>>> {
        self.govee.inner.lan.watch_status(&self.id)
    }
}

/// Republish transport events, and flag a device the catalog cannot encode for.
async fn forward(inner: Arc<Inner>, out: broadcast::Sender<Event>) {
    let mut events = inner.lan.events();
    loop {
        match events.recv().await {
            Ok(event) => {
                if let govee_lan::Event::Discovered { device, .. } = &event {
                    let sku = inner
                        .config
                        .sku_for(&device.id)
                        .unwrap_or(&device.sku)
                        .to_owned();
                    if inner.catalog.device(&sku).is_err() {
                        tracing::warn!(id = %device.id, %sku, "no device file declares this SKU");
                        let _ = out.send(Event::UnknownSku {
                            id: device.id.clone(),
                            sku,
                        });
                    }
                }
                let _ = out.send(Event::Lan(event));
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                tracing::warn!(missed, "the facade fell behind the transport's events");
            }
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// Let the user's own device files replace what the build shipped.
fn overlay_local_devices(catalog: &mut Catalog, config: &Config) -> Result<()> {
    let directory = config
        .catalog
        .directory
        .clone()
        .unwrap_or_else(paths::local_devices_dir);

    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        // Opting in without having written any file yet is not an error.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(Error::LocalDevices {
                path: directory.display().to_string(),
                reason: e.to_string(),
            });
        }
    };

    let mut files: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| Error::LocalDevices {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        files.push((path.display().to_string(), text));
    }
    files.sort();

    let replaced = catalog.overlay(files.iter().map(|(f, y)| (f.as_str(), y.as_str())))?;
    for overridden in replaced {
        // An override shadows what everyone else's build ships. It has to be
        // visible, every run.
        tracing::warn!(
            sku = %overridden.sku,
            was = %overridden.was,
            now = %overridden.now,
            "a local device file replaced the one shipped with this build"
        );
    }
    Ok(())
}
