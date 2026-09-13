//! The one precondition a command has: the device must be known to a mode.
//!
//! Nothing on the send path scans, because a scan costs a window —
//! `docs/modes.md`. A caller in a fresh process asks for the scan here, once.

use futures_util::StreamExt;
use futures_util::stream::FuturesOrdered;

use crate::codec::Mode;
use crate::error::{Error, Result};
use crate::govee::Govee;
use crate::transport::DeviceId;

impl Govee {
    /// Make one device reachable, with a scan where a scan is needed.
    ///
    /// Call it once per device before the first command, and never between
    /// commands: it is the precondition the send path refuses to pay for.
    ///
    /// The modes the configuration enables for this device scan at the same
    /// time, and the answer is the one that comes first in the configuration,
    /// not the one that answers first: that order is the user's preference.
    ///
    /// A device a mode already knows costs nothing here, whatever its health.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] with
    /// [`UnknownDevice`](crate::transport::Error::UnknownDevice)
    /// if no enabled mode finds the device, or whatever a scan fails with.
    /// [`Error::ModeNotImplemented`] or [`Error::MissingCredential`] where no
    /// enabled mode has a transport in this build.
    pub async fn ensure_known(&self, id: &DeviceId) -> Result<Mode> {
        let modes = self.inner.config.modes_for(id).to_vec();
        if let Some(mode) = self.first_mode_holding(id, &modes) {
            return Ok(mode);
        }

        let mut scans: FuturesOrdered<_> = modes
            .iter()
            .filter_map(|&mode| {
                let transport = self.inner.transports.get(&mode)?;
                Some(async move {
                    let found = transport.scan_for(id, transport.scan_window()).await;
                    (mode, found)
                })
            })
            .collect();

        // No enabled mode has a transport here. The device is not unknown: the
        // build carries nothing that can look for it, and that is what to say.
        if let (true, Some(&mode)) = (scans.is_empty(), modes.first()) {
            return Err(self.no_transport(id, mode));
        }

        // Awaited in the configuration's order. Every scan keeps running
        // while an earlier mode is awaited.
        while let Some((mode, found)) = scans.next().await {
            if found?.is_some() {
                return Ok(mode);
            }
        }
        Err(Error::Transport(crate::transport::Error::UnknownDevice {
            id: id.clone(),
        }))
    }

    /// Per mode, not per device: a device the `lan` cache answers for is still
    /// unknown to `cloud`, and a scan skipped on that cache would fail the
    /// command with `UnknownDevice`.
    fn first_mode_holding(&self, id: &DeviceId, modes: &[Mode]) -> Option<Mode> {
        modes.iter().copied().find(|mode| {
            self.inner
                .transports
                .get(mode)
                .is_some_and(|transport| transport.health(id).is_some())
        })
    }
}
