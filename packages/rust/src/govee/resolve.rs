//! The one precondition a command has: the device must be known to a mode.
//!
//! A transport answers `UnknownDevice` for a device no scan has found and no
//! cache holds. Nothing on the send path scans, because a scan costs a window
//! — `docs/modes.md`. So a caller that starts a fresh process asks for the
//! scan here, once, before it sends anything.

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
    /// It looks over the modes the configuration enables for this device, and
    /// only those. The modes run at the same time, and the answer is the
    /// enabled mode that comes first in the configuration, whichever one
    /// answers first: that order is the user's preference, and a faster mode
    /// does not win over it.
    ///
    /// A device a mode already knows costs nothing here, whatever its health:
    /// this call establishes where a device is, and the breaker decides
    /// whether to send to it.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] with
    /// [`UnknownDevice`](crate::transport::Error::UnknownDevice)
    /// if no enabled mode finds the device, or whatever a scan fails with.
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

        // In the order the modes were pushed, which is the configuration's.
        // Every scan keeps running while an earlier mode is awaited, so its
        // answer is already recorded when its turn comes.
        while let Some((mode, found)) = scans.next().await {
            if found?.is_some() {
                return Ok(mode);
            }
        }
        Err(Error::Transport(crate::transport::Error::UnknownDevice {
            id: id.clone(),
        }))
    }

    /// The first of `modes` whose transport has a record for this device.
    ///
    /// Per mode, not per device: a device the `lan` cache answers for is still
    /// unknown to `cloud`, and a scan skipped on the strength of that cache
    /// would fail the command with `UnknownDevice`.
    fn first_mode_holding(&self, id: &DeviceId, modes: &[Mode]) -> Option<Mode> {
        modes.iter().copied().find(|mode| {
            self.inner
                .transports
                .get(mode)
                .is_some_and(|transport| transport.health(id).is_some())
        })
    }
}
