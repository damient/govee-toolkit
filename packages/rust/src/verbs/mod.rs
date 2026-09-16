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
use crate::resolved::Resolved;

impl<'a> DeviceHandle<'a> {
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
        let entry = self.role_entry(Role::Color)?;

        let mut args = Args::new();
        for (arg_role, value) in [
            (ArgRole::Red, rgb[0]),
            (ArgRole::Green, rgb[1]),
            (ArgRole::Blue, rgb[2]),
        ] {
            let name = entry.arg(arg_role)?;
            args.insert(name, ArgValue::Int(i64::from(value)));
        }

        entry.send(&args).await
    }

    /// Resolve `role` for the mode a send would go over now.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus [`Error::NoRoleCommand`].
    fn role_entry(&self, role: Role) -> Result<RoleEntry<'a>> {
        let call = self.resolve()?;
        let (command, spec) =
            call.spec()
                .entry_for(call.mode(), role)
                .ok_or_else(|| Error::NoRoleCommand {
                    sku: call.sku().to_owned(),
                    mode: call.mode(),
                    role,
                })?;
        let command = command.to_owned();
        Ok(RoleEntry {
            call,
            command,
            spec,
        })
    }

    async fn one_arg(&self, role: Role, arg_role: ArgRole, value: i64) -> Result<Served> {
        let entry = self.role_entry(role)?;
        let args = Args::new().int(entry.arg(arg_role)?, value);
        entry.send(&args).await
    }
}

/// One entry of a device file, on the mode that carries it. It holds the
/// resolution the entry was read under, so the send does not resolve twice.
pub(crate) struct RoleEntry<'a> {
    call: Resolved<'a>,
    pub(crate) command: String,
    spec: &'a Command,
}

impl<'a> RoleEntry<'a> {
    pub(crate) fn mode(&self) -> Mode {
        self.call.mode()
    }

    pub(crate) fn sku(&self) -> &str {
        self.call.sku()
    }

    pub(crate) fn device(&self) -> &'a Device {
        self.call.spec()
    }

    /// Send another command of the same device file, under the resolution
    /// this entry was read with.
    pub(crate) async fn send_command(&self, command: &str, args: &Args) -> Result<Served> {
        self.call.send(command, args).await
    }

    /// [`Error::NoRoleArg`] where the entry marks no such argument.
    pub(crate) fn arg(&self, arg_role: ArgRole) -> Result<&str> {
        self.marked(arg_role).ok_or_else(|| Error::NoRoleArg {
            sku: self.sku().to_owned(),
            mode: self.mode(),
            command: self.command.clone(),
            arg_role,
        })
    }

    pub(crate) fn marked(&self, arg_role: ArgRole) -> Option<&str> {
        self.spec.arg_for(arg_role)
    }

    pub(crate) async fn send(&self, args: &Args) -> Result<Served> {
        self.call.send(&self.command, args).await
    }
}

/// [`Error::NoRoleArg`] where the entry marks none.
pub(crate) fn arg_for<'a>(
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
