//! One identity, with the mode and the SKU read once.
//!
//! Every call on a [`DeviceHandle`] resolves the mode, the SKU and the device
//! file before it encodes. A caller that builds arguments and then sends pays
//! for that twice, and the second resolution can answer a different mode than
//! the arguments were built for. [`DeviceHandle::resolve`] does it once and
//! hands back what the send path needs.

use crate::codec::coerce::{self, Supplied};
use crate::codec::{Args, Device, Error as CodecError, Mode};
use crate::device::DeviceHandle;
// Used by the doc comments only.
#[cfg(doc)]
use crate::error::Error;
use crate::error::Result;
use crate::event::Served;
use crate::transport::{DeviceStatus, Reply};

/// A device handle whose mode, SKU and device file entry are resolved.
///
/// It is a reading of recorded state at one instant, not a lease: the mode it
/// holds stays the mode it sends over, even where the device stops answering
/// on it. Resolve again for a new command rather than keeping one across a
/// long wait.
#[derive(Debug, Clone)]
pub struct Resolved<'a> {
    handle: DeviceHandle<'a>,
    mode: Mode,
    sku: String,
    device: &'a Device,
}

impl<'a> Resolved<'a> {
    pub(crate) fn new(
        handle: DeviceHandle<'a>,
        mode: Mode,
        sku: String,
        device: &'a Device,
    ) -> Self {
        Self {
            handle,
            mode,
            sku,
            device,
        }
    }

    /// The mode every call on this value goes over.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The SKU the device file was read for: the one the configuration pins,
    /// or the one the device reports.
    #[must_use]
    pub fn sku(&self) -> &str {
        &self.sku
    }

    /// What `devices/<SKU>.yaml` declares for this device.
    #[must_use]
    pub fn spec(&self) -> &'a Device {
        self.device
    }

    /// Whether the entry declares a `reply:` layout, so that
    /// [`Resolved::read`] has something to capture. `false` where the mode
    /// carries no such command.
    #[must_use]
    pub fn answers(&self, command: &str) -> bool {
        self.device
            .commands
            .get(self.mode)
            .get(command)
            .is_some_and(crate::codec::catalog::Command::answers)
    }

    /// Read values a caller supplied under the types the device file
    /// declares.
    ///
    /// An entry this mode does not carry takes no arguments, so that
    /// [`Resolved::send`] reports the unknown command rather than an unknown
    /// argument of it.
    ///
    /// # Errors
    ///
    /// [`crate::codec::Error::UnknownArg`] where the entry declares no such
    /// argument, and whatever
    /// [`coerce::read`](crate::codec::coerce::read) reports for a value that
    /// does not read under the declared type.
    pub fn args<I>(&self, command: &str, supplied: I) -> Result<Args>
    where
        I: IntoIterator<Item = (String, Supplied)>,
    {
        let Some(entry) = self.device.commands.get(self.mode).get(command) else {
            return Ok(Args::new());
        };
        let mut values = Args::new();
        for (name, value) in supplied {
            let spec = entry
                .args
                .get(&name)
                .ok_or_else(|| CodecError::UnknownArg {
                    command: command.to_owned(),
                    arg: name.clone(),
                    declared: entry.declared(),
                })?;
            values.insert(&name, coerce::read(command, &name, spec, value)?);
        }
        Ok(values)
    }

    /// Send a command over the resolved mode.
    ///
    /// # Errors
    ///
    /// [`Error::Codec`] if the command or its arguments are not valid for this
    /// device, [`Error::ModeNotImplemented`] or [`Error::MissingCredential`]
    /// if this build carries no transport for the mode, [`Error::Transport`]
    /// if the write fails.
    pub async fn send(&self, command: &str, args: &Args) -> Result<Served> {
        self.handle
            .send_resolved(self.mode, &self.sku, command, args)
            .await
    }

    /// Ask the device for its state and wait for the answer.
    ///
    /// # Errors
    ///
    /// As for [`Resolved::send`], plus
    /// [`crate::transport::Error::Unreachable`] if nothing answers in time.
    pub async fn status(&self) -> Result<DeviceStatus> {
        let request = self.handle.govee.status_request(&self.sku, self.mode)?;
        Ok(self
            .handle
            .govee
            .transport(self.handle.id(), self.mode)?
            .status(self.handle.id(), &request)
            .await?)
    }

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    ///
    /// # Errors
    ///
    /// As for [`Resolved::send`], plus
    /// [`crate::transport::Error::NoReplyLayout`] if the command declares no
    /// reply to read or the mode does not answer in frames, and
    /// [`crate::transport::Error::Unreachable`] if nothing answers in time.
    pub async fn read(&self, command: &str, args: &Args) -> Result<Reply> {
        let request = self
            .handle
            .govee
            .encode(&self.sku, self.mode, command, args)?;
        Ok(self
            .handle
            .govee
            .transport(self.handle.id(), self.mode)?
            .read(self.handle.id(), &request)
            .await?)
    }
}
