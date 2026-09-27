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
    /// [`DeviceHandle::ensure_known`](crate::DeviceHandle::ensure_known)
    /// over every mode the configuration enables for `id`.
    pub(crate) async fn ensure_known(&self, id: &DeviceId) -> Result<Mode> {
        self.known_over(id, self.inner.config.modes_for(id)).await
    }

    /// [`Govee::ensure_known`] over `mode` alone, or
    /// [`Error::ModeNotEnabled`].
    pub(crate) async fn ensure_known_on(&self, id: &DeviceId, mode: Mode) -> Result<Mode> {
        if !self.inner.config.modes_for(id).contains(&mode) {
            return Err(Error::ModeNotEnabled {
                id: id.clone(),
                mode,
            });
        }
        self.known_over(id, &[mode]).await
    }

    async fn known_over(&self, id: &DeviceId, modes: &[Mode]) -> Result<Mode> {
        if let Some(mode) = self.first_mode_holding(id, modes) {
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

        // The device is not unknown: the build carries nothing that can look
        // for it, and that is what to say.
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
