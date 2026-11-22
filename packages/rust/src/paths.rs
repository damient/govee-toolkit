//! Where the configuration and the device cache live: the folders that each
//! operating system gives an application.
//!
//! | System  | Configuration and data                    | Cache                          |
//! | ------- | ----------------------------------------- | ------------------------------ |
//! | macOS   | `~/Library/Application Support/govee-toolkit` | `~/Library/Caches/govee-toolkit` |
//! | Linux   | `$XDG_CONFIG_HOME/govee-toolkit`, or `~/.config/govee-toolkit` | `$XDG_CACHE_HOME/govee-toolkit`, or `~/.cache/govee-toolkit` |
//! | Windows | `%APPDATA%\govee-toolkit`                 | `%LOCALAPPDATA%\govee-toolkit` |
//!
//! macOS reads no `XDG_*` variable: the desktop app gets no shell environment,
//! and the CLI must find the same files as the desktop app.
//!
//! `GOVEE_CONFIG` overrides the configuration file outright, which is what a
//! test, a container and a second instance on one host all need. It is a
//! `GOVEE_*` name, so `.env` carries it like any other.

use std::ffi::OsString;
use std::path::PathBuf;

const APP: &str = "govee-toolkit";

/// The variable that names the configuration file.
const CONFIG_ENV: &str = "GOVEE_CONFIG";

/// The folder of the configuration, the credentials and the Studio file.
#[must_use]
pub fn config_dir() -> PathBuf {
    dirs(&|name| std::env::var_os(name)).config
}

/// The folder of the device cache. Its content can be deleted at any time.
#[must_use]
pub fn cache_dir() -> PathBuf {
    dirs(&|name| std::env::var_os(name)).cache
}

/// The configuration file. `GOVEE_CONFIG` wins if it is set. The process
/// environment alone supplies it — [`config_file_from`] reads `.env` too.
#[must_use]
pub fn config_file() -> PathBuf {
    config_file_from(&crate::env::Env::process())
}

/// The configuration file, with `GOVEE_CONFIG` read from `env`.
#[must_use]
pub fn config_file_from(env: &crate::env::Env) -> PathBuf {
    env.var(CONFIG_ENV)
        .map_or_else(|| config_dir().join("config.yaml"), PathBuf::from)
}

/// The credentials file: `GOVEE_*` variables in the `.env` format. The SDK
/// reads it after every other source, and a program that writes it must give
/// it the permissions `0600`.
#[must_use]
pub fn credentials_file() -> PathBuf {
    config_dir().join("credentials.env")
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

struct Dirs {
    config: PathBuf,
    cache: PathBuf,
}

/// `var` reads one environment variable, so a test can supply its own.
fn dirs(var: &dyn Fn(&str) -> Option<OsString>) -> Dirs {
    let absolute = |name: &str| var(name).map(PathBuf::from).filter(|p| p.is_absolute());
    // No home either: the current directory is a poor default, but it is a
    // usable one, and refusing to start over a missing environment variable
    // would be worse.
    let home = || var("HOME").map(PathBuf::from).unwrap_or_default();
    let (config, cache) = if cfg!(target_os = "macos") {
        let library = home().join("Library");
        (library.join("Application Support"), library.join("Caches"))
    } else if cfg!(windows) {
        let profile = || var("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        (
            absolute("APPDATA").unwrap_or_else(|| profile().join("AppData").join("Roaming")),
            absolute("LOCALAPPDATA").unwrap_or_else(|| profile().join("AppData").join("Local")),
        )
    } else {
        (
            absolute("XDG_CONFIG_HOME").unwrap_or_else(|| home().join(".config")),
            absolute("XDG_CACHE_HOME").unwrap_or_else(|| home().join(".cache")),
        )
    };
    Dirs {
        config: config.join(APP),
        cache: cache.join(APP),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(pairs: &[(&str, &str)]) -> Dirs {
        dirs(&|name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        })
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_uses_the_library_and_ignores_xdg() {
        let dirs = with(&[("HOME", "/Users/a"), ("XDG_CONFIG_HOME", "/x")]);
        assert_eq!(
            dirs.config,
            PathBuf::from("/Users/a/Library/Application Support/govee-toolkit")
        );
        assert_eq!(
            dirs.cache,
            PathBuf::from("/Users/a/Library/Caches/govee-toolkit")
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_uses_xdg_and_falls_back_to_the_home() {
        let set = with(&[
            ("HOME", "/home/a"),
            ("XDG_CONFIG_HOME", "/x"),
            ("XDG_CACHE_HOME", "/y"),
        ]);
        assert_eq!(set.config, PathBuf::from("/x/govee-toolkit"));
        assert_eq!(set.cache, PathBuf::from("/y/govee-toolkit"));
        let unset = with(&[("HOME", "/home/a"), ("XDG_CONFIG_HOME", "relative")]);
        assert_eq!(unset.config, PathBuf::from("/home/a/.config/govee-toolkit"));
        assert_eq!(unset.cache, PathBuf::from("/home/a/.cache/govee-toolkit"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_uses_the_roaming_and_the_local_folders() {
        let set = with(&[(r"APPDATA", r"C:\R"), ("LOCALAPPDATA", r"C:\L")]);
        assert_eq!(set.config, PathBuf::from(r"C:\R\govee-toolkit"));
        assert_eq!(set.cache, PathBuf::from(r"C:\L\govee-toolkit"));
        let unset = with(&[("USERPROFILE", r"C:\Users\a")]);
        assert_eq!(
            unset.config,
            PathBuf::from(r"C:\Users\a\AppData\Roaming\govee-toolkit")
        );
    }

    #[test]
    fn the_credentials_sit_beside_the_configuration() {
        assert_eq!(credentials_file().parent(), Some(config_dir().as_path()));
    }
}
