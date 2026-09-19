//! The SDK one handle drives through, and the mode it was pinned to.
//!
//! A handle from `Govee.device()` drives over the first enabled mode that
//! answers. A handle from `Govee.device_on()` drives over one mode alone: every
//! call on it goes over that mode or fails. Nothing here substitutes a mode.

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceHandle, DeviceId, Error, Govee};

/// One SDK, and the mode a handle over it was pinned to.
#[derive(Debug, Clone)]
pub(crate) struct Driver {
    govee: Govee,
    pinned: Option<Mode>,
}

impl Driver {
    pub(crate) const fn new(govee: Govee, pinned: Option<Mode>) -> Self {
        Self { govee, pinned }
    }

    /// The core handle: pinned to one mode where the caller named one.
    pub(crate) fn device(&self, id: &DeviceId) -> DeviceHandle<'_> {
        match self.pinned {
            Some(mode) => self.govee.device_on(id, mode),
            None => self.govee.device(id),
        }
    }

    /// Scan for the device if no mode knows it yet, then answer the mode a
    /// command would go over. The scan covers every enabled mode, whatever
    /// this handle is pinned to.
    ///
    /// # Errors
    ///
    /// As for [`Govee::ensure_known`].
    pub(crate) async fn ensure_known(&self, id: &DeviceId) -> Result<Mode, Error> {
        self.govee.ensure_known(id).await
    }
}
