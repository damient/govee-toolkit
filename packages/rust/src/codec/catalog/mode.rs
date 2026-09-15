//! The modes, and the one name each of them carries.
//!
//! [`Mode`] prints and reads back the name `devices/*.yaml` and `config.yaml`
//! use.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A way of talking to a device. Not a fallback chain — see `docs/modes.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// UDP on the local network. The default, and the only mode that never
    /// leaves it.
    Lan,
    /// Bluetooth Low Energy.
    Ble,
    /// Govee's cloud API.
    Cloud,
}

/// A text that names no [`Mode`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{value}` is not a mode; the modes are {}", Mode::NAMES.join(", "))]
pub struct UnknownMode {
    /// The text that was read.
    pub value: String,
}

impl Mode {
    /// Every mode this crate knows, in preference order.
    pub const ALL: [Mode; 3] = [Mode::Lan, Mode::Ble, Mode::Cloud];

    /// The same list, as the names the files use.
    pub const NAMES: [&'static str; 3] =
        [Mode::Lan.as_str(), Mode::Ble.as_str(), Mode::Cloud.as_str()];

    /// The name this mode carries in `devices/*.yaml` and in `config.yaml`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lan => "lan",
            Self::Ble => "ble",
            Self::Cloud => "cloud",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Mode {
    type Err = UnknownMode;

    /// # Errors
    ///
    /// [`UnknownMode`] where the text names none of [`Mode::NAMES`].
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_str() == text)
            .ok_or_else(|| UnknownMode {
                value: text.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn every_mode_reads_back_from_what_it_prints() {
        for mode in Mode::ALL {
            assert_eq!(mode.to_string().parse(), Ok(mode));
        }
    }

    #[test]
    fn a_text_that_names_no_mode_lists_the_names() {
        let failed = "wifi".parse::<Mode>().unwrap_err();
        assert_eq!(
            failed.to_string(),
            "`wifi` is not a mode; the modes are lan, ble, cloud"
        );
    }
}
