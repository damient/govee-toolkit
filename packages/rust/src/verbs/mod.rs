//! The commands a person names, reached through a `role:`.
//!
//! `power`, `brightness`, `color` and `color_temp` are things a device does,
//! not entries of one device file. Each method here reads the entry the file
//! marks with the matching [`Role`], and fills the arguments the file marks
//! with an [`ArgRole`]. No command name and no argument name lives in this
//! crate, and a binding gets the same verb without a second implementation.
//!
//! A file that marks no entry for a role fails with [`Error::NoRoleCommand`].
//! Nothing is approximated: a mode that carries no `role: color` entry does
//! not paint the color through another command — see `docs/modes.md`.

mod segment;
mod white;

use crate::codec::catalog::Device;
use crate::codec::{ArgRole, Args, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;

impl DeviceHandle<'_> {
    /// Turn the device on or off.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::send`], plus [`Error::NoRoleCommand`] if the
    /// device file marks no entry `role: power` for the chosen mode, and
    /// [`Error::NoRoleArg`] if that entry marks no argument `role: on`.
    pub async fn power(&self, on: bool) -> Result<Served> {
        let value = i64::from(on);
        self.send_verb(Role::Power, |args, name| args.int(name, value), ArgRole::On)
            .await
    }

    /// Set the brightness, in the unit the device file declares.
    ///
    /// The range is the one the argument's `range:` gives, and it differs per
    /// SKU and per mode. A value outside it is an error, never a clamp.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for the command marked
    /// `role: brightness` and its argument marked `role: brightness`.
    pub async fn brightness(&self, level: i64) -> Result<Served> {
        self.send_verb(
            Role::Brightness,
            |args, name| args.int(name, level),
            ArgRole::Brightness,
        )
        .await
    }

    /// Set one color over the whole device.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for `role: color` and the three
    /// components `role: red`, `role: green` and `role: blue`.
    pub async fn color(&self, rgb: [u8; 3]) -> Result<Served> {
        let mode = self.govee.choose(self.id())?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;
        let command = command_for(&sku, device, mode, Role::Color)?.to_owned();

        let mut args = Args::new();
        for (role, value) in [
            (ArgRole::Red, rgb[0]),
            (ArgRole::Green, rgb[1]),
            (ArgRole::Blue, rgb[2]),
        ] {
            let name = arg_for(&sku, device, mode, &command, role)?;
            args.insert(name, crate::codec::ArgValue::Int(i64::from(value)));
        }

        self.send_on(mode, &command, &args).await
    }

    async fn send_verb(
        &self,
        role: Role,
        fill: impl FnOnce(Args, &str) -> Args,
        arg_role: ArgRole,
    ) -> Result<Served> {
        let mode = self.govee.choose(self.id())?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;
        let command = command_for(&sku, device, mode, role)?.to_owned();
        let name = arg_for(&sku, device, mode, &command, arg_role)?.to_owned();

        self.send_on(mode, &command, &fill(Args::new(), &name))
            .await
    }
}

fn command_for<'a>(sku: &str, device: &'a Device, mode: Mode, role: Role) -> Result<&'a str> {
    device
        .command_for(mode, role)
        .ok_or_else(|| Error::NoRoleCommand {
            sku: sku.to_owned(),
            mode,
            role,
        })
}

fn arg_for<'a>(
    sku: &str,
    device: &'a Device,
    mode: Mode,
    command: &str,
    arg_role: ArgRole,
) -> Result<&'a str> {
    device
        .commands
        .get(mode)
        .get(command)
        .and_then(|spec| spec.arg_for(arg_role))
        .ok_or_else(|| Error::NoRoleArg {
            sku: sku.to_owned(),
            mode,
            command: command.to_owned(),
            arg_role,
        })
}
