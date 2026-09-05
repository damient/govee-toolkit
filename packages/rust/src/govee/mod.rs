//! The facade: it holds the catalog, the configuration and the transports, and
//! it is the one place that decides which mode serves a command.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::broadcast;

use crate::codec::{Args, Catalog, Mode};
use crate::config::{Config, Problem};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::{Device, Event};
use crate::govee::events::{Forwarder, forward};
use crate::lan::{DeviceId, Transport};
use crate::paths;

mod events;

pub(crate) struct Inner {
    pub(crate) catalog: Catalog,
    pub(crate) config: Config,
    pub(crate) lan: Transport,
    pub(crate) events: broadcast::Sender<Event>,
    /// Encoded status requests, by mode then SKU. See
    /// [`Govee::status_request`].
    status_requests: Mutex<HashMap<Mode, HashMap<String, Arc<crate::codec::Encoded>>>>,
}

/// The SDK.
///
/// Cheap to clone; every clone shares one catalog, one configuration and one
/// set of transports.
#[derive(Clone)]
pub struct Govee {
    pub(crate) inner: Arc<Inner>,
    _forwarder: Arc<Forwarder>,
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
    /// [`Error::Codec`] if a device file does not parse,
    /// [`Error::LocalDevices`] if the local directory cannot be read,
    /// [`Error::Configuration`] if the configuration cannot be applied, or
    /// [`Error::Transport`] if the socket cannot be bound.
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
            status_requests: Mutex::new(HashMap::new()),
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
        DeviceHandle::new(self, id.clone())
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
    pub(crate) fn check_devices(&self) -> Vec<Problem> {
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
                // Only `None` is a mistake to report: it is a statement that
                // the hardware cannot do this. An unprobed mode is enabled on
                // purpose by whoever is probing it.
                if device.modes.get(*mode).support == crate::codec::Support::None {
                    problems.push(Problem {
                        device: Some(known.id.clone()),
                        message: format!("`{mode}` is enabled but {sku} does not support it"),
                    });
                }
            }
        }
        problems
    }

    pub(crate) fn describe(&self, id: &DeviceId, reported: &str) -> Device {
        Device {
            id: id.clone(),
            sku: self.sku_of(id, reported),
            name: self.inner.config.name_for(id).map(ToOwned::to_owned),
            modes: self.inner.config.modes_for(id).to_vec(),
            lan_health: self.inner.lan.health(id),
        }
    }

    pub(crate) fn sku_of(&self, id: &DeviceId, reported: &str) -> String {
        self.inner.config.sku_for(id).unwrap_or(reported).to_owned()
    }

    /// The first enabled mode the device can be reached over right now.
    ///
    /// Decided entirely from recorded state. Nothing here touches a socket,
    /// which is the rule the whole design exists for: choosing a mode by
    /// trying one would cost the fast path a round-trip on every command.
    pub(crate) fn choose(&self, id: &DeviceId) -> Result<Mode> {
        let modes = self.inner.config.modes_for(id);
        for &mode in modes {
            match mode {
                Mode::Lan => match self.inner.lan.health(id) {
                    Some(health) if health.available => return Ok(mode),
                    Some(_) => {}
                    // The transport has never seen it. Nothing to choose, and
                    // scanning now is exactly what must not happen.
                    None => {
                        return Err(Error::Transport(crate::lan::Error::UnknownDevice {
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

    /// The SKU a device is encoded against: what the user configured, else what
    /// the device reported in its scan reply.
    pub(crate) fn sku(&self, id: &DeviceId) -> Result<String> {
        match self.inner.lan.sku(id) {
            Some(reported) => Ok(self.sku_of(id, &reported)),
            None => Ok(self
                .inner
                .config
                .sku_for(id)
                .ok_or_else(|| crate::lan::Error::UnknownDevice { id: id.clone() })?
                .to_owned()),
        }
    }

    /// Encode against a SKU the caller already resolved. The send path
    /// resolves it once and encodes against it, rather than per call.
    pub(crate) fn encode(
        &self,
        sku: &str,
        mode: Mode,
        command: &str,
        args: &Args,
    ) -> Result<crate::codec::Encoded> {
        let device = self.inner.catalog.device(sku)?;
        Ok(crate::codec::encode(device, mode, command, args)?)
    }

    /// The status request for a SKU, built from its device file.
    ///
    /// The command is the one the file marks `role: status` for this mode — no
    /// command name lives here. A file that names none has no status request,
    /// and [`Error::NoRoleCommand`] says so rather than a guess failing later.
    ///
    /// Encoded once per mode and SKU, then shared: it takes no arguments and
    /// the device file does not change at runtime, so the bytes never change.
    /// Every [`DeviceHandle::send`] asks for one, and most discard it —
    /// building it each time would put a frame parse on the send path for
    /// nothing.
    pub(crate) fn status_request(
        &self,
        sku: &str,
        mode: Mode,
    ) -> Result<Arc<crate::codec::Encoded>> {
        if let Ok(cache) = self.inner.status_requests.lock()
            && let Some(hit) = cache.get(&mode).and_then(|by_sku| by_sku.get(sku))
        {
            return Ok(Arc::clone(hit));
        }

        let device = self.inner.catalog.device(sku)?;
        let command = device
            .status_command(mode)
            .ok_or_else(|| Error::NoRoleCommand {
                sku: sku.to_owned(),
                mode,
                role: crate::codec::Role::Status,
            })?;
        let request = Arc::new(crate::codec::encode(device, mode, command, &Args::new())?);
        if let Ok(mut cache) = self.inner.status_requests.lock() {
            cache
                .entry(mode)
                .or_default()
                .insert(sku.to_owned(), Arc::clone(&request));
        }
        Ok(request)
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
