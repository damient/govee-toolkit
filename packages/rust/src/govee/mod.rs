//! The facade: it holds the catalog, the configuration and the transports, and
//! it is the one place that decides which mode serves a command.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use futures_util::future::try_join_all;
use tokio::sync::broadcast;

use crate::codec::{Args, Catalog, Mode};
use crate::config::{Config, Problem};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::{Device, Event};
use crate::govee::events::Forwarder;
use crate::transport::{DeviceId, Health, Transport};

mod events;
mod resolve;
mod start;

pub(crate) struct Inner {
    pub(crate) catalog: Catalog,
    pub(crate) config: Config,
    /// One transport per mode. A mode absent here has none in this build, and
    /// the SDK reports that rather than substitute another.
    pub(crate) transports: BTreeMap<Mode, Arc<dyn Transport>>,
    pub(crate) events: broadcast::Sender<Event>,
    /// Encoded status requests, by mode then SKU. See
    /// [`Govee::status_request`].
    status_requests: Mutex<HashMap<Mode, HashMap<String, Arc<crate::codec::Encoded>>>>,
}

/// The SDK. Cheap to clone; every clone shares one catalog, one configuration
/// and one set of transports.
#[derive(Clone)]
pub struct Govee {
    pub(crate) inner: Arc<Inner>,
    _forwarder: Arc<Forwarder>,
}

impl std::fmt::Debug for Govee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Govee")
            .field("modes", &self.inner.transports.keys().collect::<Vec<_>>())
            .field("devices", &self.devices().len())
            .finish_non_exhaustive()
    }
}

impl Govee {
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

    /// The modes this build carries a transport for. Not a preference order:
    /// that is each device's own configuration, on
    /// [`DeviceHandle::modes`](crate::DeviceHandle::modes).
    #[must_use]
    pub fn modes(&self) -> Vec<Mode> {
        self.inner.transports.keys().copied().collect()
    }

    /// Run a discovery scan on every transport and return what answered.
    ///
    /// Each transport listens for its own window, which is a property of the
    /// wire. The windows run at the same time, so the call takes the longest
    /// one and not their sum. Nothing on the send path calls this.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if a request cannot be sent. One transport failing
    /// fails the call: a scan covering fewer modes than asked would read as a
    /// device that is not there.
    pub async fn scan(&self) -> Result<Vec<Device>> {
        let modes: Vec<Mode> = self.inner.transports.keys().copied().collect();
        self.scan_on(&modes).await
    }

    /// Run a discovery scan on the modes named, and return what answered over
    /// one of them.
    ///
    /// A mode this build carries no transport for contributes nothing, and is
    /// not an error: the caller named the modes it wants covered.
    ///
    /// # Errors
    ///
    /// As for [`Govee::scan`].
    pub async fn scan_on(&self, modes: &[Mode]) -> Result<Vec<Device>> {
        let windows = self
            .inner
            .transports
            .iter()
            .filter(|(mode, _)| modes.contains(mode))
            .map(|(_, transport)| transport.scan(transport.scan_window()));
        // In mode order, whatever the order the answers arrive in: two modes
        // that report one MAC must always agree on which SKU wins.
        let per_mode = try_join_all(windows).await?;
        let mut found: BTreeMap<DeviceId, String> = BTreeMap::new();
        for device in per_mode.into_iter().flatten() {
            found.insert(device.id, device.sku);
        }
        Ok(found
            .into_iter()
            .map(|(id, sku)| self.describe(&id, &sku))
            .collect())
    }

    /// Every device known, across every mode. One reachable over two modes
    /// appears once: the identity is the MAC.
    #[must_use]
    pub fn devices(&self) -> Vec<Device> {
        let mut known: BTreeMap<DeviceId, String> = BTreeMap::new();
        for transport in self.inner.transports.values() {
            for device in transport.devices() {
                known.insert(device.id, device.sku);
            }
        }
        known
            .into_iter()
            .map(|(id, sku)| self.describe(&id, &sku))
            .collect()
    }

    /// Release what every transport holds. Call it before the program ends,
    /// or `ble` loses the last frame it wrote. Every other mode does nothing
    /// here, and a transport answers commands again afterwards.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if a transport reports a failure while it
    /// releases. Every transport is closed, whichever one fails.
    pub async fn shutdown(&self) -> Result<()> {
        let mut first = Ok(());
        for transport in self.inner.transports.values() {
            if let Err(e) = transport.close().await
                && first.is_ok()
            {
                first = Err(Error::from(e));
            }
        }
        first
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
        problems.extend(self.missing_credentials());
        problems.extend(self.absent_transports());
        problems.extend(self.check_devices());
        problems
    }

    /// Every enabled mode this build carries no transport for.
    ///
    /// A transport is a cargo feature, so a build carries the modes it was
    /// asked for and no other. Not a startup error: a command over the mode
    /// fails with [`Error::ModeNotImplemented`], and this is what `doctor`
    /// answers before any command is sent. A mode that has a transport in this
    /// build but no credential is [`Govee::missing_credentials`] instead.
    fn absent_transports(&self) -> Vec<Problem> {
        self.inner
            .config
            .enabled_modes()
            .into_iter()
            .filter(|mode| !self.inner.transports.contains_key(mode))
            .filter(|&mode| self.inner.config.missing_credential(mode).is_none())
            .map(|mode| Problem {
                device: None,
                message: format!(
                    "`{mode}` is enabled but this build carries no transport for it; a transport is a cargo feature"
                ),
            })
            .collect()
    }

    /// Every enabled mode whose credential the configuration does not carry.
    ///
    /// Not a startup error: the mode is reported as unavailable, and a command
    /// over it fails with [`Error::MissingCredential`]. It is what `doctor`
    /// answers before any command is sent.
    fn missing_credentials(&self) -> Vec<Problem> {
        self.inner
            .config
            .enabled_modes()
            .into_iter()
            .filter_map(|mode| {
                let remedy = self.inner.config.missing_credential(mode)?;
                Some(Problem {
                    device: None,
                    message: format!("`{mode}` is enabled but carries no credential — {remedy}"),
                })
            })
            .collect()
    }

    /// The transport serving a mode, if this build carries one.
    pub(crate) fn transport(&self, id: &DeviceId, mode: Mode) -> Result<&Arc<dyn Transport>> {
        self.inner
            .transports
            .get(&mode)
            .ok_or_else(|| self.no_transport(id, mode))
    }

    /// Why a mode has no transport: a missing credential, or a mode this
    /// build does not implement.
    fn no_transport(&self, id: &DeviceId, mode: Mode) -> Error {
        match self.inner.config.missing_credential(mode) {
            Some(remedy) => Error::MissingCredential {
                id: id.clone(),
                mode,
                remedy,
            },
            None => Error::ModeNotImplemented {
                id: id.clone(),
                mode,
            },
        }
    }

    /// Check every known device's enabled modes against what its device file
    /// says the hardware supports.
    pub(crate) fn check_devices(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for device in self.devices() {
            let Ok(file) = self.inner.catalog.device(&device.sku) else {
                problems.push(Problem {
                    device: Some(device.id.clone()),
                    message: format!(
                        "reports SKU `{}`, which no device file declares",
                        device.sku
                    ),
                });
                continue;
            };
            for mode in &device.modes {
                // Only `None` is a mistake to report: it states the hardware
                // cannot do this. Whoever probes an unprobed mode enables it on
                // purpose.
                if file.modes.get(*mode).support == crate::codec::Support::None {
                    problems.push(Problem {
                        device: Some(device.id.clone()),
                        message: format!(
                            "`{mode}` is enabled but {} does not support it",
                            device.sku
                        ),
                    });
                }
            }
        }
        problems
    }

    pub(crate) fn describe(&self, id: &DeviceId, reported: &str) -> Device {
        let modes = self.inner.config.modes_for(id).to_vec();
        let health: BTreeMap<Mode, Health> = self
            .inner
            .transports
            .iter()
            .filter(|(mode, _)| modes.contains(mode))
            .filter_map(|(mode, transport)| transport.health(id).map(|h| (*mode, h)))
            .collect();
        Device {
            id: id.clone(),
            sku: self.sku_of(id, reported),
            name: self.inner.config.name_for(id).map(ToOwned::to_owned),
            modes,
            health,
        }
    }

    pub(crate) fn sku_of(&self, id: &DeviceId, reported: &str) -> String {
        self.inner.config.sku_for(id).unwrap_or(reported).to_owned()
    }

    /// The first enabled mode the device can be reached over right now, from
    /// recorded state alone. Nothing here touches an adapter.
    pub(crate) fn choose(&self, id: &DeviceId) -> Result<Mode> {
        let modes = self.inner.config.modes_for(id);
        let mut unknown_to_every_transport = true;
        for &mode in modes {
            // An enabled mode this build has no transport for fails here. To
            // move on to the next one would substitute a mode in silence.
            let Some(transport) = self.inner.transports.get(&mode) else {
                return Err(self.no_transport(id, mode));
            };
            match transport.health(id) {
                Some(health) if health.available => return Ok(mode),
                Some(_) => unknown_to_every_transport = false,
                // This transport has never seen it. A scan on the send path is
                // what must not happen, so this mode is not a candidate.
                None => {}
            }
        }
        if unknown_to_every_transport {
            return Err(Error::Transport(crate::transport::Error::UnknownDevice {
                id: id.clone(),
            }));
        }
        Err(Error::NoModeAvailable {
            id: id.clone(),
            modes: modes.to_vec(),
        })
    }

    /// The SKU a device is encoded against: what the user configured, else what
    /// a transport heard it report.
    pub(crate) fn sku(&self, id: &DeviceId) -> Result<String> {
        if let Some(pinned) = self.inner.config.sku_for(id) {
            return Ok(pinned.to_owned());
        }
        self.inner
            .transports
            .values()
            .find_map(|transport| transport.sku(id))
            .ok_or_else(|| crate::transport::Error::UnknownDevice { id: id.clone() }.into())
    }

    /// Encode against a SKU the caller already resolved, so the send path
    /// resolves it once rather than per call.
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

    /// The entry the file marks `role: status` for this mode, encoded. A file
    /// naming none gives [`Error::NoRoleCommand`].
    ///
    /// Cached per mode and SKU: the bytes never change, and every send asks
    /// for one, so a fresh encode would put a frame parse on the send path.
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
