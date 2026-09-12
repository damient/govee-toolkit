//! `provision`: put a device on a Wi-Fi network over `ble`.
//!
//! Nothing acknowledges the transfer, so a refused one looks like an accepted
//! one here. The report says what was sent, never that the device joined.

use govee_toolkit::{DeviceId, Env, Govee, WifiCredentials};
use serde_json::json;

use crate::output::{Failure, Writer};

const PASSWORD_VAR: &str = "GOVEE_WIFI_PASSWORD";

const SSID_VAR: &str = "GOVEE_WIFI_SSID";

pub(super) struct Secret<'a> {
    pub(super) password: Option<&'a str>,
    pub(super) open: bool,
}

pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    id: &DeviceId,
    ssid: Option<&str>,
    secret: &Secret<'_>,
    utc_offset: (u8, u8),
) -> Result<(), Failure> {
    let env = &govee.config().env;
    let ssid = network(env, ssid)?;
    let credentials = WifiCredentials {
        network: ssid.clone(),
        password: password(env, secret)?,
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

fn network(env: &Env, ssid: Option<&str>) -> Result<String, Failure> {
    match ssid {
        Some(ssid) => Ok(ssid.to_owned()),
        None => env.var(SSID_VAR).map(ToOwned::to_owned).ok_or_else(|| {
            Failure::usage(format!("supply `--ssid`, or set `{SSID_VAR}` in `.env`"))
        }),
    }
}

/// An empty password joins an open network, so it is never a default: the
/// caller asks for one with `--open`.
fn password(env: &Env, secret: &Secret<'_>) -> Result<String, Failure> {
    match (secret.open, secret.password) {
        (true, Some(_)) => Err(Failure::usage(
            "`--open` and `--password` cannot both be given".to_owned(),
        )),
        (true, None) => Ok(String::new()),
        (false, Some(password)) => Ok(password.to_owned()),
        (false, None) => env.var(PASSWORD_VAR).map(ToOwned::to_owned).ok_or_else(|| {
            Failure::usage(format!(
                "supply `--password`, set `{PASSWORD_VAR}` in `.env`, or pass `--open` for a network that has no password"
            ))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_network_name_on_the_command_line_wins() {
        let env = Env::from_pairs([(SSID_VAR, "from-the-file")]);
        assert_eq!(network(&env, Some("home")).ok(), Some("home".to_owned()));
    }

    #[test]
    fn the_variables_supply_the_credentials() {
        let env = Env::from_pairs([(SSID_VAR, "home"), (PASSWORD_VAR, "two words ")]);
        let secret = Secret {
            password: None,
            open: false,
        };
        assert_eq!(network(&env, None).ok(), Some("home".to_owned()));
        // A password is verbatim: the spaces in it are part of it.
        assert_eq!(password(&env, &secret).ok(), Some("two words ".to_owned()));
    }

    #[test]
    fn an_open_network_takes_no_password() {
        let secret = Secret {
            password: None,
            open: true,
        };
        assert_eq!(password(&Env::default(), &secret).ok(), Some(String::new()));
    }

    #[test]
    fn a_password_and_open_together_are_refused() {
        let secret = Secret {
            password: Some("hunter2"),
            open: true,
        };
        assert!(password(&Env::default(), &secret).is_err());
    }

    #[test]
    fn no_password_anywhere_names_what_to_set() {
        let secret = Secret {
            password: None,
            open: false,
        };
        let outcome = password(&Env::default(), &secret);
        assert!(outcome.is_err());
        assert!(format!("{outcome:?}").contains(PASSWORD_VAR));
    }
}
