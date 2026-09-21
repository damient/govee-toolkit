//! One device, bound to the SDK that reaches it.
//!
//! Every method here goes through the mode the user enabled. Nothing falls back
//! to another one — see `docs/modes.md`.

use std::time::Duration;

use crate::codec::coerce::Supplied;
use crate::codec::{Args, Device, Mode};
// Used by the doc comments only.
#[cfg(doc)]
use crate::error::Error;
use crate::error::Result;
use crate::event::Served;
use crate::govee::Govee;
use crate::resolved::Resolved;
use crate::stream::{SegmentStream, StreamOptions};
use crate::transport::{DeviceId, DeviceStatus, Health, Reply, Verify};

/// A borrow of the SDK and one identity, holding no state of its own.
#[derive(Debug, Clone)]
pub struct DeviceHandle<'a> {
    pub(crate) govee: &'a Govee,
    id: DeviceId,
    /// The one mode this handle drives over, where a caller named one.
    pinned: Option<Mode>,
}

impl<'a> DeviceHandle<'a> {
    pub(crate) fn new(govee: &'a Govee, id: DeviceId) -> Self {
        Self::maybe_on(govee, id, None)
    }

    pub(crate) fn on(govee: &'a Govee, id: DeviceId, mode: Mode) -> Self {
        Self::maybe_on(govee, id, Some(mode))
    }

    pub(crate) fn maybe_on(govee: &'a Govee, id: DeviceId, pinned: Option<Mode>) -> Self {
        Self { govee, id, pinned }
    }

    /// The mode a call on this handle goes over: the pinned one, or the first
    /// enabled mode that answers.
    fn mode(&self) -> Result<Mode> {
        match self.pinned {
            Some(mode) => self.govee.choose_on(&self.id, mode),
            None => self.govee.choose(&self.id),
        }
    }

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

    /// The mode a command sent now would go over, for a caller that must know
    /// it before it builds arguments. Read from recorded state, as a send
    /// reads it, so the answer can change before the next send.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], when no enabled mode can serve a
    /// command.
    pub fn serving_mode(&self) -> Result<Mode> {
        self.mode()
    }

    /// How long a command to this device waits behind the one before it, on
    /// the mode a send goes over now. A caller that sends several commands in
    /// a row waits this long between two of them.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::serving_mode`].
    pub fn command_gap(&self) -> Result<Duration> {
        let mode = self.mode()?;
        Ok(self.govee.transport(&self.id, mode)?.command_gap())
    }

    /// What `devices/<SKU>.yaml` declares for this device: the modes, the
    /// capabilities, the commands and their arguments. Reads no hardware.
    ///
    /// # Errors
    ///
    /// [`crate::transport::Error::UnknownDevice`] if no transport knows this
    /// device and the configuration pins no SKU for it, or
    /// [`crate::codec::Error::UnknownSku`] if no device file declares the SKU
    /// it reports.
    pub fn spec(&self) -> Result<&Device> {
        Ok(self.govee.catalog().device(&self.govee.sku(&self.id)?)?)
    }

    /// Resolve the mode, the SKU and the device file once, for a caller
    /// that builds arguments and then sends.
    ///
    /// Every other method here resolves them again per call. Two resolutions
    /// can answer two modes, so a caller that reads the mode and then sends
    /// must hold one [`Resolved`] over both.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::serving_mode`] and [`DeviceHandle::spec`].
    pub fn resolve(&self) -> Result<Resolved<'a>> {
        let mode = self.mode()?;
        let sku = self.govee.sku(&self.id)?;
        let device = self.govee.catalog().device(&sku)?;
        Ok(Resolved::new(self.clone(), mode, sku, device))
    }

    /// Read values a caller supplied under the types the device file
    /// declares, for the mode a send now would go over.
    ///
    /// The mode is resolved first, because one entry name can declare
    /// different arguments on two modes. An entry the mode does not carry
    /// takes no arguments, so that [`DeviceHandle::send`] reports the unknown
    /// command rather than an unknown argument of it.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::resolve`], plus what [`Resolved::args`]
    /// reports.
    pub fn args<I>(&self, command: &str, supplied: I) -> Result<Args>
    where
        I: IntoIterator<Item = (String, Supplied)>,
    {
        self.resolve()?.args(command, supplied)
    }

    /// Send a command, named as the device file names it.
    ///
    /// The mode is chosen first, then the command is encoded **for that
    /// mode**. A command that mode does not carry fails with
    /// [`crate::codec::Error::UnknownCommand`] rather than being
    /// approximated — `docs/modes.md`.
    ///
    /// # Errors
    ///
    /// [`Error::NoModeAvailable`], [`Error::ModeNotImplemented`] or
    /// [`Error::MissingCredential`] if no enabled mode can serve it,
    /// [`Error::Codec`] if the command or its arguments are not valid for this
    /// device, [`Error::Transport`] if the write fails.
    pub async fn send(&self, command: &str, args: &Args) -> Result<Served> {
        let mode = self.mode()?;
        let sku = self.govee.sku(&self.id)?;
        self.send_resolved(mode, &sku, command, args).await
    }

    // The caller resolved the SKU to build `args`, so the send path does not
    // resolve it again.
    pub(crate) async fn send_resolved(
        &self,
        mode: Mode,
        sku: &str,
        command: &str,
        args: &Args,
    ) -> Result<Served> {
        let encoded = self.govee.encode(sku, mode, command, args)?;

        // Fire-and-verify needs a request to verify with. Without a status
        // command in the device file, the command still goes out, unverified.
        // A device with an armed segment channel can answer no status until
        // the disarm (`docs/protocol/lan.md` 2.3), so a request sent there
        // would cost a datagram and record a failure the device did not earn.
        let verification = if self.govee.stream_armed(&self.id, mode) {
            None
        } else {
            self.govee.status_request(sku, mode).ok()
        };
        let verify = verification.map_or(Verify::None, Verify::With);

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
    /// never block. Power the device on first: arming a dark strip paints
    /// nothing (`docs/protocol/lan.md` 2.3).
    ///
    /// The channel holds the colors only while it is armed. Closing the stream,
    /// or dropping it, disarms the channel, and the device goes back to the
    /// color it showed before.
    ///
    /// # Errors
    ///
    /// [`Error::ModeNotImplemented`] if this build carries no transport for
    /// the chosen mode, [`Error::MissingCredential`] if it carries one that
    /// has no credential,
    /// [`Error::NoRoleCommand`] if the device file marks no entry
    /// `role: segment_enable`, and none `role: segment_color` or
    /// `role: segment_color_masked`, or if
    /// [`StreamOptions::gradient`](crate::StreamOptions::gradient) asks for
    /// interpolation the mode carries nowhere,
    /// [`Error::ZoneCountUnknown`] if the count asked for is not recorded for
    /// this unit,
    /// [`Error::NativeZonesUnreachable`] if the mode paints by zone mask and
    /// cannot reach native resolution, [`Error::ZoneCountUnsupported`] if the
    /// mode paints fewer zones than a count the caller states,
    /// [`Error::Codec`] if the zone count is outside what the command declares,
    /// or [`Error::Transport`] if arming cannot be sent.
    pub async fn open_stream(&self, options: StreamOptions) -> Result<SegmentStream> {
        SegmentStream::open(self.govee, &self.id, self.mode()?, options).await
    }

    /// Ask the device for its state and wait for the answer.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus
    /// [`crate::transport::Error::Unreachable`] if nothing answers in time.
    pub async fn status(&self) -> Result<DeviceStatus> {
        self.resolve()?.status().await
    }

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    ///
    /// How a value the SDK does not model reaches a caller. The device file
    /// names the frames, the bytes and the field; none of that lives here.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus
    /// [`crate::transport::Error::NoReplyLayout`] if the command declares no
    /// reply to read or the chosen mode does not answer in frames, and
    /// [`crate::transport::Error::Unreachable`] if nothing answers in time.
    pub async fn read(&self, command: &str, args: &Args) -> Result<Reply> {
        self.resolve()?.read(command, args).await
    }

    /// The last status heard, without asking for a new one.
    ///
    /// Read from the mode that would serve a command right now, and never
    /// from another. `None` if no enabled mode can, or if that transport has
    /// heard nothing.
    #[must_use]
    pub fn last_status(&self) -> Option<DeviceStatus> {
        let mode = self.mode().ok()?;
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
        let mode = self.mode().ok()?;
        self.govee
            .transport(&self.id, mode)
            .ok()?
            .watch_status(&self.id)
    }
}
