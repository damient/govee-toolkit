//! One device, bound to the SDK that reaches it.
//!
//! Every method here goes through the mode the user enabled. Nothing falls back
//! to another one — see `docs/modes.md`.

use crate::codec::{Args, Mode};
use crate::error::{Error, Result};
use crate::event::Served;
use crate::govee::Govee;
use crate::lan::{DeviceId, DeviceStatus, Health};
use crate::stream::{SegmentStream, StreamOptions};

/// A borrow of the SDK and one identity. It holds no state of its own: every
/// call reads the configuration and the health recorded right now.
#[derive(Debug, Clone)]
pub struct DeviceHandle<'a> {
    govee: &'a Govee,
    id: DeviceId,
}

impl<'a> DeviceHandle<'a> {
    pub(crate) fn new(govee: &'a Govee, id: DeviceId) -> Self {
        Self { govee, id }
    }
}

impl DeviceHandle<'_> {
    /// The device's identity.
    #[must_use]
    pub fn id(&self) -> &DeviceId {
        &self.id
    }

    /// The modes enabled for it, in preference order.
    #[must_use]
    pub fn modes(&self) -> &[Mode] {
        self.govee.inner.config.modes_for(&self.id)
    }

    /// Its health in one mode, if that mode is known to a transport.
    #[must_use]
    pub fn health(&self, mode: Mode) -> Option<Health> {
        match mode {
            Mode::Lan => self.govee.inner.lan.health(&self.id),
            Mode::Ble | Mode::Cloud => None,
        }
    }

    /// Send a command, named as the device file names it.
    ///
    /// The mode is chosen first, then the command is encoded **for that mode**.
    /// A command the chosen mode does not carry fails with
    /// [`crate::codec::Error::UnknownCommand`] rather than being approximated —
    /// `docs/modes.md`, capability differences between modes.
    ///
    /// # Errors
    ///
    /// [`Error::NoModeAvailable`] or [`Error::ModeNotImplemented`] if no
    /// enabled mode can serve it, [`Error::Codec`] if the command or its
    /// arguments are not valid for this device, [`Error::Transport`] if the
    /// write fails.
    pub async fn send(&self, command: &str, args: &Args) -> Result<Served> {
        let mode = self.govee.choose(&self.id)?;
        let sku = self.govee.sku(&self.id)?;
        let encoded = self.govee.encode(&sku, mode, command, args)?;

        // Fire-and-verify needs a request to verify with. A device file that
        // declares no status command is not verified — the command still
        // goes out.
        let verification = self.govee.status_request(&sku, mode).ok();
        let verify = verification
            .as_deref()
            .map_or(crate::lan::Verify::None, crate::lan::Verify::With);

        let sent = match mode {
            Mode::Lan => {
                self.govee
                    .inner
                    .lan
                    .send(&self.id, &encoded, verify)
                    .await?
            }
            Mode::Ble | Mode::Cloud => {
                return Err(Error::ModeNotImplemented {
                    id: self.id.clone(),
                    mode,
                });
            }
        };

        Ok(Served {
            id: sent.id,
            mode: sent.mode,
            command: command.to_owned(),
            cmd: sent.cmd,
        })
    }

    /// Open the raw segment channel and stream colors to it.
    ///
    /// Arms the channel and starts emitting; the writers on [`SegmentStream`]
    /// never block. Power the device on first — `turn(1)` precedes arming
    /// (`docs/protocol/lan.md` 2.3), and this crate names no command of its
    /// own.
    ///
    /// # Errors
    ///
    /// [`Error::ModeNotImplemented`] if the chosen mode is not `lan`,
    /// [`Error::NoRoleCommand`] if the device file marks no entry
    /// `role: segment_enable` or `role: segment_color`,
    /// [`Error::ZoneCountUnknown`] if the count asked for is not recorded for
    /// this unit,
    /// [`Error::Codec`] if the zone count is outside what the command declares,
    /// or [`Error::Transport`] if arming cannot be sent.
    pub async fn open_stream(&self, options: StreamOptions) -> Result<SegmentStream> {
        SegmentStream::open(self.govee, &self.id, options).await
    }

    /// Ask the device for its state and wait for the answer.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus [`crate::lan::Error::Unreachable`]
    /// if nothing answers in time.
    pub async fn status(&self) -> Result<DeviceStatus> {
        let mode = self.govee.choose(&self.id)?;
        let request = self
            .govee
            .status_request(&self.govee.sku(&self.id)?, mode)?;
        match mode {
            Mode::Lan => Ok(self.govee.inner.lan.status(&self.id, &request).await?),
            Mode::Ble | Mode::Cloud => Err(Error::ModeNotImplemented {
                id: self.id.clone(),
                mode,
            }),
        }
    }

    /// The last status heard, without asking for a new one.
    ///
    /// `None` unless `lan` is enabled for this device: the recorded status
    /// belongs to that transport, and handing it back under another mode would
    /// be a silent substitution.
    #[must_use]
    pub fn last_status(&self) -> Option<DeviceStatus> {
        if !self.modes().contains(&Mode::Lan) {
            return None;
        }
        self.govee.inner.lan.last_status(&self.id)
    }

    /// Watch this device's status as replies arrive.
    ///
    /// `None` unless `lan` is enabled for this device, as for
    /// [`DeviceHandle::last_status`].
    #[must_use]
    pub fn watch_status(&self) -> Option<tokio::sync::watch::Receiver<Option<DeviceStatus>>> {
        if !self.modes().contains(&Mode::Lan) {
            return None;
        }
        self.govee.inner.lan.watch_status(&self.id)
    }
}
