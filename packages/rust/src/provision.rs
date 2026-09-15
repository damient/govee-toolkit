//! Put a device on a Wi-Fi network over `ble`.
//!
//! The order lives here and the bytes stay in `devices/*.yaml`. Each step
//! looks its entry up by [`Role`] and fills the arguments by [`ArgRole`], so a
//! family whose frames differ changes its device file and no code. See
//! `docs/architecture.md`.

use std::time::Duration;

use crate::codec::{ArgRole, ArgValue, Args, Device, Encoded, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};

/// How long the Wi-Fi module takes to come up. `docs/protocol/ble.md` 4.
const WAKE_DELAY: Duration = Duration::from_secs(3);

/// What the firmware takes for `run_mode` and `iot_version` in production.
const PRODUCTION: i64 = 0;

/// A device that is not on a network yet has no `lan`.
const MODE: Mode = Mode::Ble;

/// The status an acknowledgement carries when the device takes the transfer.
const ACCEPTED: i64 = 0;

/// What a transfer reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provisioned {
    /// The device acknowledged the transfer and took the credentials. It does
    /// not say the device joined the network.
    Accepted,
    /// The writes went out. The device file declares no acknowledgement for
    /// this transfer, so a refused one is indistinguishable from this.
    Sent,
}

impl Provisioned {
    /// The name this outcome carries wherever a surface reports it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Sent => "sent",
        }
    }
}

/// What a device needs to join a network. The password travels in plaintext,
/// with no key exchange and no session token: anything in Bluetooth range
/// during provisioning reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiCredentials {
    /// The network name. 2.4 GHz: no Govee device joins a 5 GHz network.
    pub network: String,
    /// The password. Empty for an open network.
    pub password: String,
    /// Whole hours of the device's UTC offset. The SDK does not read the host
    /// clock. Nobody observed a negative offset on the wire.
    pub utc_offset_hours: u8,
    /// The remaining minutes of that offset. `0` on a whole-hour zone.
    pub utc_offset_minutes: u8,
}

impl DeviceHandle<'_> {
    /// Put this device on a Wi-Fi network, then release the Wi-Fi module.
    ///
    /// The device must be in Bluetooth range and closed in the phone
    /// controller, which holds the one connection the radio accepts. The
    /// device joins the network, and reaches `lan` only once the user
    /// enables LAN Control.
    ///
    /// [`Provisioned::Accepted`] needs a file that declares the
    /// acknowledgement to read.
    ///
    /// # Errors
    ///
    /// [`Error::NoModeAvailable`] if `ble` is not enabled for this device,
    /// [`Error::NoRoleCommand`] if the device file claims no provisioning
    /// role, [`Error::ModeNotImplemented`] if this build carries no `ble`
    /// transport, [`Error::Codec`] if an argument is outside what the file
    /// declares, [`Error::ProvisionRefused`] if the device refuses the
    /// transfer, and [`Error::Transport`] if a write fails or the
    /// acknowledgement does not arrive.
    pub async fn provision_wifi(&self, credentials: &WifiCredentials) -> Result<Provisioned> {
        let modes = self.modes();
        if !modes.contains(&MODE) {
            return Err(Error::NoModeAvailable {
                id: self.id().clone(),
                modes: modes.to_vec(),
            });
        }
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;

        let api_url = self.read_api_url(&sku, device).await?;
        let (command, args) = credentials.encode(&sku, device, api_url.as_deref())?;

        self.set_wifi_link(&sku, device, 1).await?;
        tokio::time::sleep(WAKE_DELAY).await;
        let transfer = self.transfer(&sku, &command, &args).await;
        // Release the module whatever the transfer did, so the radio does not
        // stay awake.
        let release = self.set_wifi_link(&sku, device, 0).await;
        let outcome = transfer?;
        release?;
        Ok(outcome)
    }

    /// Send the transfer, and read its acknowledgement where the device file
    /// declares one.
    async fn transfer(&self, sku: &str, command: &str, args: &Args) -> Result<Provisioned> {
        let encoded = self.govee.encode(sku, MODE, command, args)?;
        let transport = self.govee.transport(self.id(), MODE)?;
        if !encoded.answers() {
            transport
                .send(self.id(), &encoded, crate::transport::Verify::None)
                .await?;
            return Ok(Provisioned::Sent);
        }
        let field = captured_status(sku, command, &encoded)?;
        let reply = transport.read(self.id(), &encoded).await?;
        match reply.fields.get(&field) {
            Some(ArgValue::Int(ACCEPTED)) => Ok(Provisioned::Accepted),
            Some(ArgValue::Int(status)) => Err(Error::ProvisionRefused {
                id: self.id().clone(),
                status: *status,
            }),
            _ => Err(no_role_arg(sku, command, ArgRole::AckStatus)),
        }
    }

    /// The endpoint this device asks for, or `None` where its file claims no
    /// [`Role::WifiApiType`] entry.
    async fn read_api_url(&self, sku: &str, device: &Device) -> Result<Option<String>> {
        let Some(command) = device.command_for(MODE, Role::WifiApiType) else {
            return Ok(None);
        };
        let field = crate::verbs::arg_for(sku, device, MODE, command, ArgRole::ApiType)?.to_owned();
        let request = self.govee.encode(sku, MODE, command, &Args::new())?;
        let reply = self
            .govee
            .transport(self.id(), MODE)?
            .read(self.id(), &request)
            .await?;
        let Some(ArgValue::Int(api_type)) = reply.fields.get(&field) else {
            return Ok(None);
        };
        Ok(endpoint(*api_type).map(ToOwned::to_owned))
    }

    async fn set_wifi_link(&self, sku: &str, device: &Device, on: i64) -> Result<()> {
        let command = command_named(sku, device, Role::WifiLink)?;
        let arg = crate::verbs::arg_for(sku, device, MODE, command, ArgRole::Enable)?;
        let args = Args::new().int(arg, on);
        self.send_role(sku, command, &args).await
    }

    async fn send_role(&self, sku: &str, command: &str, args: &Args) -> Result<()> {
        let encoded = self.govee.encode(sku, MODE, command, args)?;
        self.govee
            .transport(self.id(), MODE)?
            .send(self.id(), &encoded, crate::transport::Verify::None)
            .await?;
        Ok(())
    }
}

impl WifiCredentials {
    /// The entry to send and the arguments to fill it with. The language
    /// carries no optional field, so the transfer that ends in an endpoint is
    /// an entry of its own.
    fn encode(&self, sku: &str, device: &Device, api_url: Option<&str>) -> Result<(String, Args)> {
        let role = match api_url {
            Some(_) => Role::WifiProvisionWithApi,
            None => Role::WifiProvision,
        };
        let command = command_named(sku, device, role)?.to_owned();
        let named = |arg_role| crate::verbs::arg_for(sku, device, MODE, &command, arg_role);

        let mut args = Args::new()
            .text(named(ArgRole::Network)?, &self.network)
            .text(named(ArgRole::Password)?, &self.password)
            .int(named(ArgRole::RunMode)?, PRODUCTION)
            .int(named(ArgRole::IotVersion)?, PRODUCTION)
            .int(named(ArgRole::TimezoneHours)?, self.utc_offset_hours.into())
            .int(
                named(ArgRole::TimezoneMinutes)?,
                self.utc_offset_minutes.into(),
            );
        if let Some(url) = api_url {
            args = args.text(named(ArgRole::ApiUrl)?, url);
        }
        Ok((command, args))
    }
}

/// The endpoint a device asking for this type must be handed. An unknown type
/// gets none, and provisioning goes out without the block.
fn endpoint(api_type: i64) -> Option<&'static str> {
    match api_type {
        1 => Some("http://app.govee.com"),
        2 => Some("https://device.govee.com"),
        _ => None,
    }
}

/// The field the transfer's acknowledgement reports its status in.
fn captured_status(sku: &str, command: &str, encoded: &Encoded) -> Result<String> {
    encoded
        .roles
        .iter()
        .find(|(_, role)| **role == ArgRole::AckStatus)
        .map(|(field, _)| field.clone())
        .ok_or_else(|| no_role_arg(sku, command, ArgRole::AckStatus))
}

fn no_role_arg(sku: &str, command: &str, arg_role: ArgRole) -> Error {
    Error::NoRoleArg {
        sku: sku.to_owned(),
        mode: MODE,
        command: command.to_owned(),
        arg_role,
    }
}

fn command_named<'a>(sku: &str, device: &'a Device, role: Role) -> Result<&'a str> {
    device
        .command_for(MODE, role)
        .ok_or_else(|| Error::NoRoleCommand {
            sku: sku.to_owned(),
            mode: MODE,
            role,
        })
}
