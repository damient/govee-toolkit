//! Which mode serves a device, and the transport behind it.
//!
//! Every answer here is read from recorded state: a scan on the send path is
//! what must not happen — `docs/modes.md`. A mode is never substituted for
//! another, so a mode that cannot serve the device is an error and not a step
//! towards the next one.

use std::sync::Arc;

use crate::codec::Mode;
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::transport::{DeviceId, Transport};

impl Govee {
    /// The transport serving a mode, if this build carries one.
    pub(crate) fn transport(&self, id: &DeviceId, mode: Mode) -> Result<&Arc<dyn Transport>> {
        self.inner
            .transports
            .get(&mode)
            .ok_or_else(|| self.no_transport(id, mode))
    }

    /// Why a mode has no transport: a missing credential, or a mode this
    /// build does not implement.
    pub(crate) fn no_transport(&self, id: &DeviceId, mode: Mode) -> Error {
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

    /// Resolve one named mode for a device, for a caller that drives over
    /// that mode alone.
    ///
    /// It never falls back: a caller that names a mode gets that mode or an
    /// error. The mode must be one the configuration enables, because a
    /// caller does not override what the user enabled.
    pub(crate) fn choose_on(&self, id: &DeviceId, mode: Mode) -> Result<Mode> {
        if !self.inner.config.modes_for(id).contains(&mode) {
            return Err(Error::ModeNotEnabled {
                id: id.clone(),
                mode,
            });
        }
        let Some(transport) = self.inner.transports.get(&mode) else {
            return Err(self.no_transport(id, mode));
        };
        match transport.health(id) {
            Some(health) if health.available => Ok(mode),
            Some(_) => Err(Error::NoModeAvailable {
                id: id.clone(),
                modes: vec![mode],
            }),
            None => Err(Error::Transport(crate::transport::Error::UnknownDevice {
                id: id.clone(),
            })),
        }
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
}
