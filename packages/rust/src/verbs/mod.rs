//! The commands a person names, reached through a `role:`.
//!
//! Each method reads the entry the device file marks with the matching
//! [`Role`], and fills the arguments it marks with an [`ArgRole`]. No command
//! name and no argument name lives in this crate.
//!
//! A file that marks no entry for a role fails with [`Error::NoRoleCommand`].
//! Nothing is approximated — see `docs/modes.md`.

mod music;
mod segment;
mod white;

pub use music::Music;
pub use segment::Paint;

use crate::codec::catalog::{Command, Device};
use crate::codec::{ArgRole, ArgValue, Args, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;

impl DeviceHandle<'_> {
    /// Turn the device on or off.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus [`Error::NoRoleCommand`] where the
    /// device file marks no entry `role: power` for the chosen mode, and
    /// [`Error::NoRoleArg`] where that entry marks no argument `role: on`.
    pub async fn power(&self, on: bool) -> Result<Served> {
        self.one_arg(Role::Power, ArgRole::On, i64::from(on)).await
    }

    /// Set the brightness, in the unit the argument's `range:` gives. That
    /// range differs per SKU and per mode, and a value outside it is an error,
    /// never a clamp.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for the command marked
    /// `role: brightness` and its argument marked `role: brightness`.
    pub async fn brightness(&self, level: i64) -> Result<Served> {
        self.one_arg(Role::Brightness, ArgRole::Brightness, level)
            .await
    }

    /// Set one color over the whole device.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for `role: color` and the three
    /// components `role: red`, `role: green` and `role: blue`.
    pub async fn color(&self, rgb: [u8; 3]) -> Result<Served> {
        let entry = self.resolve(Role::Color)?;

        let mut args = Args::new();
        for (arg_role, value) in [
            (ArgRole::Red, rgb[0]),
            (ArgRole::Green, rgb[1]),
            (ArgRole::Blue, rgb[2]),
        ] {
            let name = entry.arg(arg_role)?;
            args.insert(name, ArgValue::Int(i64::from(value)));
        }

        entry.send(self, &args).await
    }

    /// Resolve `role` for the mode a send would go over now.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus [`Error::NoRoleCommand`].
    fn resolve(&self, role: Role) -> Result<Resolved<'_>> {
        let mode = self.serving_mode()?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;
        let (command, spec) = device
            .entry_for(mode, role)
            .ok_or_else(|| Error::NoRoleCommand {
                sku: sku.clone(),
                mode,
                role,
            })?;
        Ok(Resolved {
            mode,
            sku,
            device,
            command: command.to_owned(),
            spec,
        })
    }

    async fn one_arg(&self, role: Role, arg_role: ArgRole, value: i64) -> Result<Served> {
        let entry = self.resolve(role)?;
        let args = Args::new().int(entry.arg(arg_role)?, value);
        entry.send(self, &args).await
    }
}

/// One entry of a device file, resolved for the mode that carries it. It holds
/// the SKU it was read for, so the send does not resolve it twice.
pub(crate) struct Resolved<'a> {
    pub(crate) mode: Mode,
    pub(crate) sku: String,
    pub(crate) device: &'a Device,
    pub(crate) command: String,
    spec: &'a Command,
}

impl Resolved<'_> {
    /// [`Error::NoRoleArg`] where the entry marks no such argument.
    pub(crate) fn arg(&self, arg_role: ArgRole) -> Result<&str> {
        self.marked(arg_role).ok_or_else(|| Error::NoRoleArg {
            sku: self.sku.clone(),
            mode: self.mode,
            command: self.command.clone(),
            arg_role,
        })
    }

    pub(crate) fn marked(&self, arg_role: ArgRole) -> Option<&str> {
        self.spec.arg_for(arg_role)
    }

    pub(crate) async fn send(&self, handle: &DeviceHandle<'_>, args: &Args) -> Result<Served> {
        handle
            .send_resolved(self.mode, &self.sku, &self.command, args)
            .await
    }
}

/// [`Error::NoRoleArg`] where the entry marks none.
fn arg_for<'a>(
    sku: &str,
    device: &'a Device,
    mode: Mode,
    command: &str,
    arg_role: ArgRole,
) -> Result<&'a str> {
    marked(device, mode, command, arg_role).ok_or_else(|| Error::NoRoleArg {
        sku: sku.to_owned(),
        mode,
        command: command.to_owned(),
        arg_role,
    })
}

fn marked<'a>(device: &'a Device, mode: Mode, command: &str, arg_role: ArgRole) -> Option<&'a str> {
    device
        .commands
        .get(mode)
        .get(command)
        .and_then(|spec| spec.arg_for(arg_role))
}
