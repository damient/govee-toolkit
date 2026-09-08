//! Put a device on a Wi-Fi network over `ble`.
//!
//! The bytes stay in `devices/*.yaml`, as everywhere else. What lives here is
//! the **order**: the module must be woken, the transfer follows a delay, and
//! the module is released after. The device file cannot express that today,
//! and the order is the same for every device that provisions this way, so a
//! per-SKU entry would repeat it once per file.
//!
//! Nothing here names a command or a SKU. Each step looks its entry up by
//! [`Role`], and fills that entry's arguments by [`ArgRole`], so a family whose
//! frames differ changes its device file and no code. See
//! `docs/architecture.md`.

use std::time::Duration;

use crate::codec::{ArgRole, ArgValue, Args, Device, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};

/// How long the Wi-Fi module takes to come up.
///
/// Measured on one unit: a transfer sent with no delay after the wake left it
/// off the network, and 3 s put it on. The vendor app waits the same.
const WAKE_DELAY: Duration = Duration::from_secs(3);

/// What the firmware takes for `run_mode` and `iot_version` in production.
const PRODUCTION: i64 = 0;

/// Provisioning runs over `ble` and over nothing else: a device that is not on
/// a network yet cannot be reached over `lan`.
const MODE: Mode = Mode::Ble;

/// What a device needs to join a network.
///
/// The password travels in plaintext, with no key exchange and no session
/// token. Anything in Bluetooth range during provisioning reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiCredentials {
    /// The network name. 2.4 GHz: no Govee device joins a 5 GHz network.
    pub network: String,
    /// The password. Empty for an open network.
    pub password: String,
    /// Whole hours of the device's UTC offset. The SDK does not read the host
    /// clock, and what a negative offset looks like on the wire was never
    /// observed.
    pub utc_offset_hours: u8,
    /// The remaining minutes of that offset. `0` on a whole-hour zone.
    pub utc_offset_minutes: u8,
}

impl DeviceHandle<'_> {
    /// Put this device on a Wi-Fi network, then release the Wi-Fi module.
    ///
    /// The device must be in Bluetooth range and closed in the vendor app,
    /// which holds the one connection the radio accepts. On success the device
    /// joins the network; it does not become reachable over `lan` until the
    /// user enables LAN Control for it.
    ///
    /// This reports what the writes did, not what the firmware made of them.
    /// The device answers a status byte, and the codec reads no reply on a
    /// chunked command, so a refused transfer looks like an accepted one here.
    /// Whether the device joined is what says it worked.
    ///
    /// # Errors
    ///
    /// [`Error::NoModeAvailable`] if `ble` is not enabled for this device,
    /// [`Error::NoRoleCommand`] if the device file claims no provisioning
    /// role, [`Error::ModeNotImplemented`] if this build carries no `ble`
    /// transport, [`Error::Codec`] if an argument is outside what the file
    /// declares, and [`Error::Transport`] if a write fails.
    pub async fn provision_wifi(&self, credentials: &WifiCredentials) -> Result<()> {
        // Enabling a mode stays the user's call here as everywhere else: this
        // runs over `ble`, so `ble` must be enabled for the device.
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
        let transfer = self.send_role(&sku, &command, &args).await;
        // Release the module whatever the transfer did, so a failed one does
        // not leave the radio awake.
        let release = self.set_wifi_link(&sku, device, 0).await;
        transfer.and(release)
    }

    /// The endpoint this device asks for, or `None` where its file claims no
    /// [`Role::WifiApiType`] entry.
    async fn read_api_url(&self, sku: &str, device: &Device) -> Result<Option<String>> {
        let Some(command) = device.command_for(MODE, Role::WifiApiType) else {
            return Ok(None);
        };
        let field = arg_named(sku, device, command, ArgRole::ApiType)?.to_owned();
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

    /// Wake the Wi-Fi module, or release it.
    async fn set_wifi_link(&self, sku: &str, device: &Device, on: i64) -> Result<()> {
        let command = command_named(sku, device, Role::WifiLink)?;
        let arg = arg_named(sku, device, command, ArgRole::Enable)?;
        let args = Args::new().int(arg, on);
        self.send_role(sku, command, &args).await
    }

    /// Encode one entry for `ble` and write it.
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
    /// The entry to send and the arguments to fill it with.
    ///
    /// Which entry depends on the endpoint: the device-file language carries
    /// no optional field, so the transfer that ends in an endpoint is an entry
    /// of its own.
    fn encode(&self, sku: &str, device: &Device, api_url: Option<&str>) -> Result<(String, Args)> {
        let role = match api_url {
            Some(_) => Role::WifiProvisionWithApi,
            None => Role::WifiProvision,
        };
        let command = command_named(sku, device, role)?.to_owned();

        let mut args = Args::new()
            .text(
                arg_named(sku, device, &command, ArgRole::Network)?,
                &self.network,
            )
            .text(
                arg_named(sku, device, &command, ArgRole::Password)?,
                &self.password,
            )
            .int(
                arg_named(sku, device, &command, ArgRole::RunMode)?,
                PRODUCTION,
            )
            .int(
                arg_named(sku, device, &command, ArgRole::IotVersion)?,
                PRODUCTION,
            )
            .int(
                arg_named(sku, device, &command, ArgRole::TimezoneHours)?,
                self.utc_offset_hours.into(),
            )
            .int(
                arg_named(sku, device, &command, ArgRole::TimezoneMinutes)?,
                self.utc_offset_minutes.into(),
            );
        if let Some(url) = api_url {
            args = args.text(arg_named(sku, device, &command, ArgRole::ApiUrl)?, url);
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

/// The entry a device file marks with this role.
fn command_named<'a>(sku: &str, device: &'a Device, role: Role) -> Result<&'a str> {
    device
        .command_for(MODE, role)
        .ok_or_else(|| Error::NoRoleCommand {
            sku: sku.to_owned(),
            mode: MODE,
            role,
        })
}

/// The argument an entry marks with this role.
fn arg_named<'a>(sku: &str, device: &'a Device, command: &str, role: ArgRole) -> Result<&'a str> {
    device
        .commands
        .get(MODE)
        .get(command)
        .and_then(|spec| spec.arg_for(role))
        .ok_or_else(|| {
            Error::Codec(crate::codec::Error::UnknownArg {
                command: command.to_owned(),
                arg: format!("one marked `role: {role}` on `{sku}`"),
            })
        })
}
