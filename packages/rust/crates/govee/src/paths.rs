//! Where the configuration and the device cache live.
//!
//! XDG on every platform, rather than one convention per operating system:
//! this project is as likely to run on a Raspberry Pi as on a laptop, and a
//! single set of paths is one less thing to explain in every binding.
//!
//! `GOVEE_CONFIG` overrides the configuration file outright, which is what a
//! test, a container and a second instance on one host all need.

use std::path::PathBuf;

/// The directory name used under the config and cache roots.
const APP: &str = "govee-toolkit";

/// `$XDG_CONFIG_HOME/govee-toolkit`, or `~/.config/govee-toolkit`.
#[must_use]
pub fn config_dir() -> PathBuf {
    root("XDG_CONFIG_HOME", ".config").join(APP)
}

/// `$XDG_CACHE_HOME/govee-toolkit`, or `~/.cache/govee-toolkit`.
#[must_use]
pub fn cache_dir() -> PathBuf {
    root("XDG_CACHE_HOME", ".cache").join(APP)
}

/// The configuration file. `GOVEE_CONFIG` wins if it is set.
#[must_use]
pub fn config_file() -> PathBuf {
    std::env::var_os("GOVEE_CONFIG").map_or_else(|| config_dir().join("config.yaml"), PathBuf::from)
}

/// The user's own device files, consulted only when the configuration opts in.
#[must_use]
pub fn local_devices_dir() -> PathBuf {
    config_dir().join("devices")
}

/// Where discovery results are written between runs.
#[must_use]
pub fn device_cache_file() -> PathBuf {
    cache_dir().join("devices.json")
}

fn root(variable: &str, fallback: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os(variable) {
        let dir = PathBuf::from(dir);
        if dir.is_absolute() {
            return dir;
        }
    }
    // No home either: the current directory is a poor default, but it is a
    // usable one, and refusing to start over a missing environment variable
    // would be worse.
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(fallback)
}
