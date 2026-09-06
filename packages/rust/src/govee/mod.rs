//! The facade: it holds the catalog, the configuration and the transports, and
//! it is the one place that decides which mode serves a command.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::broadcast;

use crate::codec::{Args, Catalog, Mode};
use crate::config::{Config, Problem};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::{Device, Event};
use crate::govee::events::Forwarder;
use crate::transport::{DeviceId, Health, Transport};

mod events;
mod start;

pub(crate) struct Inner {
    pub(crate) catalog: Catalog,
    pub(crate) config: Config,
    /// One transport per mode it serves. A mode absent here has no transport in
    /// this build — the feature is off, or nothing implements it yet — and the
    /// SDK reports that rather than substitute another mode.
    pub(crate) transports: BTreeMap<Mode, Arc<dyn Transport>>,
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

    /// The modes this build carries a transport for, in preference order.
    #[must_use]
    pub fn modes(&self) -> Vec<Mode> {
        self.inner.transports.keys().copied().collect()
    }

    /// Run a discovery scan on every transport and return what answered.
    ///
    /// Nothing on the send path calls this: it runs at startup and on the
    /// background interval. See `docs/protocol/lan.md` §1, latency notes.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if a request cannot be sent. One transport failing
    /// fails the call: a scan that quietly covered fewer modes than asked would
    /// read as a device that is not there.
    pub async fn scan(&self) -> Result<Vec<Device>> {
        let window = Duration::from_millis(self.inner.config.lan.scan_window_ms);
        let mut found: BTreeMap<DeviceId, String> = BTreeMap::new();
        for transport in self.inner.transports.values() {
            for device in transport.scan(window).await? {
                found.insert(device.id, device.sku);
            }
        }
        Ok(found
            .into_iter()
            .map(|(id, sku)| self.describe(&id, &sku))
            .collect())
    }

    /// Every device known, from discovery or from a cache, across every mode.
    ///
    /// One device reachable over two modes appears once: the identity is the
    /// MAC, and it is the same unit.
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

    /// The transport serving a mode, if this build carries one.
    pub(crate) fn transport(&self, id: &DeviceId, mode: Mode) -> Result<&Arc<dyn Transport>> {
        self.inner
            .transports
            .get(&mode)
            .ok_or_else(|| Error::ModeNotImplemented {
                id: id.clone(),
                mode,
            })
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

    /// The first enabled mode the device can be reached over right now.
    ///
    /// Decided from recorded state alone. Nothing here touches an adapter: a
    /// trial send would cost the fast path a round-trip on every command.
    pub(crate) fn choose(&self, id: &DeviceId) -> Result<Mode> {
        let modes = self.inner.config.modes_for(id);
        let mut unknown_to_every_transport = true;
        for &mode in modes {
            // Reached only when a preferred mode was unavailable, which is
            // exactly when a silent substitution would be wrong.
            let Some(transport) = self.inner.transports.get(&mode) else {
                return Err(Error::ModeNotImplemented {
                    id: id.clone(),
                    mode,
                });
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

    /// The status request for a SKU, built from its device file.
    ///
    /// The command is the one the file marks `role: status` for this mode — no
    /// command name lives here. A file that names none has no status request,
    /// and [`Error::NoRoleCommand`] says so rather than a guess failing later.
    ///
    /// Encoded once per mode and SKU, then shared: it takes no arguments and
    /// the device file does not change at runtime, so the bytes never change.
    /// Every [`DeviceHandle::send`] asks for one, and most discard it, so a
    /// fresh encode would put a frame parse on the send path for nothing.
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
