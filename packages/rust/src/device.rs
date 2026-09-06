//! One device, bound to the SDK that reaches it.
//!
//! Every method here goes through the mode the user enabled. Nothing falls back
//! to another one — see `docs/modes.md`.

use crate::codec::{Args, Mode};
// Referenced from the doc comments below, nowhere else.
#[cfg(doc)]
use crate::error::Error;
use crate::error::Result;
use crate::event::Served;
use crate::govee::Govee;
use crate::stream::{SegmentStream, StreamOptions};
use crate::transport::{DeviceId, DeviceStatus, Health, Reply, Verify};

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

    /// Its health in one mode.
    ///
    /// `None` when this build carries no transport for that mode, or when the
    /// transport that serves it has never heard from this device.
    #[must_use]
    pub fn health(&self, mode: Mode) -> Option<Health> {
        self.govee.transport(&self.id, mode).ok()?.health(&self.id)
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
        let verify = verification.as_deref().map_or(Verify::None, Verify::With);

        let sent = self
            .govee
            .transport(&self.id, mode)?
            .send(&self.id, &encoded, verify)
            .await?;

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
    /// [`Error::ModeNotImplemented`] if this build carries no transport for
    /// the chosen mode,
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
    /// As for [`DeviceHandle::send`], plus
    /// [`crate::transport::Error::Unreachable`] if nothing answers in time.
    pub async fn status(&self) -> Result<DeviceStatus> {
        let mode = self.govee.choose(&self.id)?;
        let request = self
            .govee
            .status_request(&self.govee.sku(&self.id)?, mode)?;
        Ok(self
            .govee
            .transport(&self.id, mode)?
            .status(&self.id, &request)
            .await?)
    }

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    ///
    /// This is how a value the SDK does not model reaches a caller: the device
    /// file says which frames ask for it, which bytes carry it and under what
    /// name, and nothing about any of that lives in this crate.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus
    /// [`crate::transport::Error::NoReplyLayout`] if the command declares no
    /// reply to read or the chosen mode does not answer in frames, and
    /// [`crate::transport::Error::Unreachable`] if nothing answers in time.
    pub async fn read(&self, command: &str, args: &Args) -> Result<Reply> {
        let mode = self.govee.choose(&self.id)?;
        let sku = self.govee.sku(&self.id)?;
        let request = self.govee.encode(&sku, mode, command, args)?;
        Ok(self
            .govee
            .transport(&self.id, mode)?
            .read(&self.id, &request)
            .await?)
    }

    /// The last status heard, without asking for a new one.
    ///
    /// Read from the mode that would serve a command right now. `None` if no
    /// enabled mode can, or if that transport has heard nothing yet: a status
    /// recorded by one transport handed back under another mode would be a
    /// silent substitution.
    #[must_use]
    pub fn last_status(&self) -> Option<DeviceStatus> {
        let mode = self.govee.choose(&self.id).ok()?;
        self.govee
            .transport(&self.id, mode)
            .ok()?
            .last_status(&self.id)
    }

    /// Watch this device's status as answers arrive.
    ///
    /// From the same mode as [`DeviceHandle::last_status`], and `None` under
    /// the same conditions.
    #[must_use]
    pub fn watch_status(&self) -> Option<tokio::sync::watch::Receiver<Option<DeviceStatus>>> {
        let mode = self.govee.choose(&self.id).ok()?;
        self.govee
            .transport(&self.id, mode)
            .ok()?
            .watch_status(&self.id)
    }
}
