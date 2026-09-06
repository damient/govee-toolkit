//! The runtime configuration: which modes the user enables, per device.
//!
//! This is the second of the two levels in `docs/modes.md`. What the hardware
//! supports is `devices/<SKU>.yaml` and is not editable here; what is *enabled*
//! is this file, and enabling a mode the hardware does not support is a
//! configuration error reported at startup rather than a surprise at send time.
//!
//! YAML, at `~/.config/govee-toolkit/config.yaml` — the same language as the
//! device files, so a contributor reads one syntax and not two.
//!
//! ```yaml
//! catalog:
//!   local_devices: false        # opt-in; see `CatalogConfig`
//!
//! defaults:
//!   modes: [lan]                # any device without an entry below
//!
//! stream:
//!   fallback_hz: 10             # any mode, where the device file measured none
//!
//! devices:
//!   "AA:BB:CC:DD:EE:FF":
//!     modes: [lan]              # one mode means one mode: it never switches
//!   "11:22:33:44:55:66":
//!     modes: [lan, ble]         # lan preferred, may switch to ble
//! ```
//!
//! Unknown keys are refused. A misspelled option that was silently ignored
//! would read as a setting that did not work.
//!
//! The fallback frame rate is `stream.fallback_hz`. It applies to whichever
//! mode a stream opens on, so `lan` does not hold it: `lan.stream_fallback_hz`
//! is an unknown key, and the load fails.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::codec::Mode;
use crate::error::{Error, Result};
use crate::transport::DeviceId;

mod lan;

pub use self::lan::LanConfig;

/// The whole configuration file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Where device files come from.
    pub catalog: CatalogConfig,
    /// What applies to a device with no entry of its own.
    pub defaults: Defaults,
    /// Transport tuning for `lan`.
    pub lan: LanConfig,
    /// Segment streaming settings.
    pub stream: StreamConfig,
    /// Per-device settings, keyed by the MAC the device reports.
    pub devices: BTreeMap<DeviceId, DeviceConfig>,
}

/// Segment streaming settings.
///
/// A stream picks its mode from what the device enables, and this section
/// applies to whichever mode it picked.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StreamConfig {
    /// The rate to send at when the device file records no measurement for the
    /// mode the stream opens on.
    pub fallback_hz: f64,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            fallback_hz: crate::stream::FALLBACK_HZ,
        }
    }
}

/// Where device files come from.
///
/// The catalog compiled into the build is the normal source. A local directory
/// serves someone who reverse-engineers their own unit, and it is **opt-in**:
/// what one person measured on one device must not silently become what
/// everyone's device is assumed to do.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CatalogConfig {
    /// Read `<config dir>/devices/*.yaml` and let them replace what the build
    /// shipped. Every replacement is logged. Off unless asked for.
    pub local_devices: bool,
    /// Where those files are, if not the default directory.
    pub directory: Option<PathBuf>,
}

/// What applies to a device with no entry of its own.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Defaults {
    /// The enabled modes, in preference order.
    pub modes: Vec<Mode>,
}

impl Default for Defaults {
    fn default() -> Self {
        // `lan` alone: the only mode that never leaves the local network.
        // Everything else is opt-in. See docs/modes.md.
        Self {
            modes: vec![Mode::Lan],
        }
    }
}

/// One device's settings.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeviceConfig {
    /// The enabled modes, in preference order. Unset falls back to
    /// [`Defaults::modes`].
    pub modes: Option<Vec<Mode>>,
    /// The SKU to read the device file under, instead of the one discovery
    /// reports. For a device whose reported SKU is not in the catalog but
    /// which is known to behave like one that is.
    pub sku: Option<String>,
    /// A name for logs and user interfaces. Nothing reads it as identity.
    pub name: Option<String>,
}

/// One thing wrong with the configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Which device it is about, if any.
    pub device: Option<DeviceId>,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.device {
            Some(id) => write!(f, "{id}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl Config {
    /// Read the configuration from its default location.
    ///
    /// A missing file is the default configuration — `lan` alone — not an
    /// error: an SDK must work before anyone writes one.
    ///
    /// # Errors
    ///
    /// [`Error::Config`] if the file exists but cannot be read or parsed. A
    /// configuration that does not parse is never guessed at.
    pub fn load() -> Result<Self> {
        Self::load_from(crate::paths::config_file())
    }

    /// Read the configuration from `path`.
    ///
    /// # Errors
    ///
    /// See [`Config::load`].
    pub fn load_from(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(Error::Config {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                });
            }
        };
        serde_norway::from_str(&text).map_err(|e| Error::Config {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// The modes enabled for a device, in preference order.
    #[must_use]
    pub fn modes_for(&self, id: &DeviceId) -> &[Mode] {
        self.devices
            .get(id)
            .and_then(|d| d.modes.as_deref())
            .unwrap_or(&self.defaults.modes)
    }

    /// The SKU a device should be read under, if the configuration pins one.
    #[must_use]
    pub fn sku_for(&self, id: &DeviceId) -> Option<&str> {
        self.devices.get(id).and_then(|d| d.sku.as_deref())
    }

    /// The name a device is shown under, if the configuration gives one.
    #[must_use]
    pub fn name_for(&self, id: &DeviceId) -> Option<&str> {
        self.devices.get(id).and_then(|d| d.name.as_deref())
    }

    /// What is wrong with the configuration on its own terms.
    ///
    /// Checks that do not need to know which devices exist. Whether a device
    /// supports an enabled mode is checked separately, once its SKU is known —
    /// see [`crate::Govee::problems`].
    #[must_use]
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.defaults.modes.is_empty() {
            problems.push(Problem {
                device: None,
                message: "defaults.modes is empty; no device could be reached".to_owned(),
            });
        }
        for (id, device) in &self.devices {
            if device.modes.as_ref().is_some_and(Vec::is_empty) {
                problems.push(Problem {
                    device: Some(id.clone()),
                    message: "modes is empty; this device could never be reached".to_owned(),
                });
            }
            if let Some(modes) = &device.modes {
                let mut seen = Vec::new();
                for mode in modes {
                    if seen.contains(mode) {
                        problems.push(Problem {
                            device: Some(id.clone()),
                            message: format!("`{mode}` is listed twice in modes"),
                        });
                    }
                    seen.push(*mode);
                }
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn parse(yaml: &str) -> Config {
        serde_norway::from_str(yaml).expect("the configuration parses")
    }

    #[test]
    fn an_absent_file_is_lan_alone() {
        let config =
            Config::load_from("/nonexistent/govee/config.yaml").expect("no file, no error");
        assert_eq!(config.defaults.modes, vec![Mode::Lan]);
        assert!(config.devices.is_empty());
        assert!(!config.catalog.local_devices);
    }

    #[test]
    fn reads_the_documented_shape() {
        let config = parse(
            r#"
            catalog:
              local_devices: true
            defaults:
              modes: [lan]
            devices:
              "aa:bb:cc:dd:ee:ff":
                modes: [lan]
              "11:22:33:44:55:66":
                modes: [lan, ble]
                name: "desk"
            "#,
        );
        assert!(config.catalog.local_devices);

        let strict = DeviceId::new("AA:BB:CC:DD:EE:FF");
        assert_eq!(config.modes_for(&strict), [Mode::Lan]);

        let flexible = DeviceId::new("11:22:33:44:55:66");
        assert_eq!(config.modes_for(&flexible), [Mode::Lan, Mode::Ble]);
        assert_eq!(config.name_for(&flexible), Some("desk"));

        assert_eq!(
            config.modes_for(&DeviceId::new("99:99:99:99:99:99")),
            [Mode::Lan]
        );
    }

    #[test]
    fn a_device_key_is_matched_whatever_its_case() {
        let config = parse("devices:\n  \"aa:bb:cc:dd:ee:ff\":\n    modes: [cloud]\n");
        assert_eq!(
            config.modes_for(&DeviceId::new("AA:BB:CC:DD:EE:FF")),
            [Mode::Cloud]
        );
    }

    #[test]
    fn a_misspelled_key_is_refused_rather_than_ignored() {
        let error = serde_norway::from_str::<Config>("defaults:\n  mode: [lan]\n")
            .expect_err("`mode` is not `modes`");
        assert!(error.to_string().contains("mode"), "{error}");
    }

    #[test]
    fn an_unreachable_device_is_a_problem() {
        let config = parse("devices:\n  \"aa:bb:cc:dd:ee:ff\":\n    modes: []\n");
        let problems = config.problems();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("never be reached"));
    }

    #[test]
    fn a_mode_listed_twice_is_a_problem() {
        let config = parse("devices:\n  \"aa:bb:cc:dd:ee:ff\":\n    modes: [lan, lan]\n");
        assert_eq!(config.problems().len(), 1);
    }
}
