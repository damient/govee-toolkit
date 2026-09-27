//! The one resolver: targets in, device handles out.
//!
//! An identity selects itself. A name and a group read the configuration. A
//! SKU, and [`Filter::all`], read the devices a scan found: the first call
//! that needs one scans, and later calls read what that scan found.

use std::collections::BTreeMap;

#[cfg(doc)]
use super::Error;
use super::{Selector, matches, resolve};
use crate::codec::{Catalog, Mode};
use crate::device::DeviceHandle;
use crate::devices::Devices;
use crate::error::Result;
use crate::event::Device;
use crate::govee::Govee;
use crate::transport::DeviceId;

/// What [`Govee::device`] takes: a target a person wrote, or an identity the
/// caller holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'t> {
    /// Read as [`Selector::resolve`] reads it.
    Written(&'t str),
    /// Selects this device, with nothing parsed.
    Id(&'t DeviceId),
}

impl<'t> From<&'t str> for Target<'t> {
    fn from(written: &'t str) -> Self {
        Self::Written(written)
    }
}

impl<'t> From<&'t String> for Target<'t> {
    fn from(written: &'t String) -> Self {
        Self::Written(written)
    }
}

impl<'t> From<&'t DeviceId> for Target<'t> {
    fn from(id: &'t DeviceId) -> Self {
        Self::Id(id)
    }
}

/// The devices that [`Govee::devices`] selects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// `None` selects every device a scan finds.
    targets: Option<Vec<Wanted>>,
    enables: Option<Mode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Wanted {
    Written(String),
    Id(DeviceId),
}

impl Filter {
    /// Every device that a scan finds.
    #[must_use]
    pub fn all() -> Self {
        Self::default()
    }

    /// The devices that the targets name, in the order written. An empty list
    /// selects no device.
    #[must_use]
    pub fn targets<I, T>(targets: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Self {
            targets: Some(
                targets
                    .into_iter()
                    .map(|target| Wanted::Written(target.into()))
                    .collect(),
            ),
            enables: None,
        }
    }

    /// These identities, in this order. It parses nothing and scans nothing.
    #[must_use]
    pub fn ids<I>(ids: I) -> Self
    where
        I: IntoIterator<Item = DeviceId>,
    {
        Self {
            targets: Some(ids.into_iter().map(Wanted::Id).collect()),
            enables: None,
        }
    }

    /// Keep the devices that the configuration enables `mode` for. A SKU, a
    /// name or a group that then keeps no device is
    /// [`Error::NotOnMode`]. An identity is kept whatever it enables.
    #[must_use]
    pub fn enables(mut self, mode: Mode) -> Self {
        self.enables = Some(mode);
        self
    }

    /// A target that fails to parse fails later, in the resolver, so it
    /// asks for no scan here.
    fn needs_scan(&self, catalog: &Catalog) -> bool {
        self.targets.as_ref().is_none_or(|targets| {
            targets.iter().any(|wanted| match wanted {
                Wanted::Written(written) => {
                    matches!(Selector::parse(written, catalog), Ok(Selector::Sku(_)))
                }
                Wanted::Id(_) => false,
            })
        })
    }
}

impl Govee {
    /// A handle for the one device that `target` names: an identity, or a
    /// name that the configuration gives. It reads the configuration alone
    /// and scans nothing.
    ///
    /// `mode` pins every call on the handle to that mode. With `None`, each
    /// call goes over the first enabled mode that answers.
    ///
    /// # Errors
    ///
    /// [`Error::NotOne`] for a SKU or a group, and every other
    /// [`Error`] that [`Selector::resolve`] reports. An identity never fails.
    pub fn device<'t>(
        &self,
        target: impl Into<Target<'t>>,
        mode: Option<Mode>,
    ) -> Result<DeviceHandle<'_>> {
        let id = match target.into() {
            Target::Id(id) => id.clone(),
            Target::Written(written) => Selector::one(written, self.catalog(), self.config())?,
        };
        Ok(DeviceHandle::new(self, id, mode))
    }

    /// The devices that `filter` selects, as one handle.
    ///
    /// A device that two targets name appears once, where it was named first.
    /// `mode` pins every member as for [`Govee::device`]: a member that does
    /// not enable it fails alone, on each call.
    ///
    /// The first call that reads a SKU, or [`Filter::all`], scans the modes it
    /// reads: the mode of [`Filter::enables`], else `mode`, else every mode.
    /// [`Govee::scan`] counts as that scan.
    ///
    /// # Errors
    ///
    /// [`Error::NoMatch`] where a SKU, a name or a group matches no device,
    /// [`Error::NotOnMode`] as for [`Filter::enables`], every other
    /// [`Error`] that [`Selector::resolve`] reports, and what
    /// [`Govee::scan`] reports.
    pub async fn devices(&self, filter: Filter, mode: Option<Mode>) -> Result<Devices<'_>> {
        let ids = self.select(&filter, mode).await?;
        Ok(Devices::new(
            ids.into_iter()
                .map(|id| DeviceHandle::new(self, id, mode))
                .collect(),
        ))
    }

    pub(crate) async fn select(
        &self,
        filter: &Filter,
        mode: Option<Mode>,
    ) -> Result<Vec<DeviceId>> {
        if filter.needs_scan(self.catalog()) {
            self.scan_once(filter.enables.or(mode)).await?;
        }
        let Some(targets) = &filter.targets else {
            return Ok(self
                .known()
                .into_iter()
                .filter(|device| {
                    filter
                        .enables
                        .is_none_or(|mode| device.modes.contains(&mode))
                })
                .map(|device| device.id)
                .collect());
        };
        let candidates = self.candidates();
        let mut chosen: Vec<DeviceId> = Vec::new();
        for wanted in targets {
            let ids = match wanted {
                Wanted::Id(id) => vec![id.clone()],
                Wanted::Written(written) => {
                    let selector = resolve(written, Some(self.catalog()), candidates.as_slice())?;
                    matches(&selector, &candidates, filter.enables)?
                }
            };
            for id in ids {
                if !chosen.contains(&id) {
                    chosen.push(id);
                }
            }
        }
        Ok(chosen)
    }

    /// The devices a transport knows, and the devices the configuration
    /// declares. A declared device that no transport knows carries the SKU
    /// the configuration pins, or an empty one that no SKU matches.
    fn candidates(&self) -> Vec<Device> {
        let mut all = self.known_skus();
        for id in self.inner.config.devices.keys() {
            all.entry(id.clone()).or_default();
        }
        all.into_iter()
            .map(|(id, sku)| self.describe(&id, &sku))
            .collect()
    }

    /// Scan once per mode and per SDK. The lock stays held over the scan,
    /// so two calls at the same time scan once between them.
    async fn scan_once(&self, mode: Option<Mode>) -> Result<()> {
        let mut scanned = self.inner.scanned.lock().await;
        let missing: Vec<Mode> = match mode {
            Some(mode) => vec![mode],
            None => self.modes(),
        }
        .into_iter()
        .filter(|mode| !scanned.contains(mode))
        .collect();
        if missing.is_empty() {
            return Ok(());
        }
        self.scan_windows(Some(&missing)).await?;
        scanned.extend(missing);
        Ok(())
    }

    pub(crate) fn known_skus(&self) -> BTreeMap<DeviceId, String> {
        let mut known = BTreeMap::new();
        for transport in self.inner.transports.values() {
            for device in transport.devices() {
                known.insert(device.id, device.sku);
            }
        }
        known
    }

    /// Every device a transport knows. One that two modes reach appears once:
    /// the identity is the MAC.
    pub(crate) fn known(&self) -> Vec<Device> {
        self.known_skus()
            .into_iter()
            .map(|(id, sku)| self.describe(&id, &sku))
            .collect()
    }
}
