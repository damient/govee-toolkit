//! `provision`: put a device on a Wi-Fi network over `ble`.
//!
//! Nothing acknowledges the transfer, so a refused one looks like an accepted
//! one here. The report says what was sent, never that the device joined.

use govee_toolkit::{DeviceId, Govee, WifiCredentials};
use serde_json::json;

use crate::output::{Failure, Writer};

/// The environment variable that supplies the password.
const PASSWORD_VAR: &str = "GOVEE_WIFI_PASSWORD";

/// What the caller typed about the password.
pub(super) struct Secret<'a> {
    /// `--password`.
    pub(super) password: Option<&'a str>,
    /// `--open`, for a network that has none.
    pub(super) open: bool,
}

/// Send the credentials, then report what went out.
pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    id: &DeviceId,
    ssid: &str,
    secret: &Secret<'_>,
    utc_offset: (u8, u8),
) -> Result<(), Failure> {
    let credentials = WifiCredentials {
        network: ssid.to_owned(),
        password: password(secret)?,
        utc_offset_hours: utc_offset.0,
        utc_offset_minutes: utc_offset.1,
    };

    govee.device(id).provision_wifi(&credentials).await?;

    writer.emit(
        &json!({
            "id": id.to_string(),
            "network": ssid,
            "sent": true,
        }),
        &format!(
            "{id}  credentials for `{ssid}` were sent; nothing acknowledges the transfer, so check the network"
        ),
    );
    Ok(())
}

/// The password, from the command line or from the environment.
///
/// An empty password joins an open network, so it is never a default: the
/// caller asks for one with `--open`.
fn password(secret: &Secret<'_>) -> Result<String, Failure> {
    match (secret.open, secret.password) {
        (true, Some(_)) => Err(Failure::usage(
            "`--open` and `--password` cannot both be given".to_owned(),
        )),
        (true, None) => Ok(String::new()),
        (false, Some(password)) => Ok(password.to_owned()),
        (false, None) => std::env::var(PASSWORD_VAR).map_err(|_| {
            Failure::usage(format!(
                "supply `--password`, set `{PASSWORD_VAR}`, or pass `--open` for a network that has no password"
            ))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_open_network_takes_no_password() {
        let secret = Secret {
            password: None,
            open: true,
        };
        assert_eq!(password(&secret).ok(), Some(String::new()));
    }

    #[test]
    fn a_password_and_open_together_are_refused() {
        let secret = Secret {
            password: Some("hunter2"),
            open: true,
        };
        assert!(password(&secret).is_err());
    }
}
