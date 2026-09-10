//! The open connections, one slot per device.
//!
//! Opening a connection costs seconds: a scan, then the connection itself. The
//! map lock covers the lookup alone, so two devices connect at the same time
//! and two commands to one device queue on that device's slot.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ble::link::Link;
use crate::transport::DeviceId;
use crate::transport::error::{Error, Result};

/// A device's connection, and the lock a caller holds while it opens one.
pub(super) type Slot = tokio::sync::Mutex<Option<Arc<Link>>>;

pub(super) struct Links(Mutex<HashMap<DeviceId, Arc<Slot>>>);

impl Links {
    pub(super) fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    /// The device's slot, empty and new if this is the first command for it.
    ///
    /// # Errors
    ///
    /// [`Error::ShutDown`] if the lock is poisoned.
    pub(super) fn slot(&self, id: &DeviceId) -> Result<Arc<Slot>> {
        let mut links = self.0.lock().map_err(|_| Error::ShutDown)?;
        Ok(Arc::clone(links.entry(id.clone()).or_default()))
    }

    /// Take every slot out of the map. A command that arrives afterwards
    /// finds it empty and opens its own connection.
    pub(super) fn take_all(&self) -> Vec<(DeviceId, Arc<Slot>)> {
        let Ok(mut links) = self.0.lock() else {
            return Vec::new();
        };
        links.drain().collect()
    }

    /// Empty the device's slot, if it still holds `link`.
    ///
    /// A slot another caller holds is left alone: that caller is opening a
    /// connection to put in it, and `link` is the one it replaces.
    pub(super) fn forget(&self, id: &DeviceId, link: &Arc<Link>) {
        let Ok(links) = self.0.lock() else {
            return;
        };
        let Some(slot) = links.get(id).map(Arc::clone) else {
            return;
        };
        drop(links);
        let Ok(mut open) = slot.try_lock() else {
            return;
        };
        if open.as_ref().is_some_and(|held| Arc::ptr_eq(held, link)) {
            *open = None;
        }
    }
}
