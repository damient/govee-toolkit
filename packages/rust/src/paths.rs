//! Where the configuration and the device cache live: docs/modes.md, "Where
//! the files are".
//!
//! macOS reads no `XDG_*` variable: the desktop app gets no shell environment,
//! and the CLI must find the same files.
//!
//! `GOVEE_CONFIG` overrides the configuration file outright, which is what a
//! test, a container and a second instance on one host all need. It is a
//! `GOVEE_*` name, so `.env` carries it like any other.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

const APP: &str = "govee-toolkit";

/// The variable that names the configuration file.
const CONFIG_ENV: &str = "GOVEE_CONFIG";

/// The configuration folder. It can be the deprecated one: [`legacy_warning`].
#[must_use]
pub fn config_dir() -> PathBuf {
    let var = |name: &str| std::env::var_os(name);
    match legacy(&var, &Path::is_dir) {
        Legacy::InUse { old, .. } => old,
        Legacy::Absent | Legacy::Ignored { .. } => dirs(&var).config,
    }
}

/// The warning for a configuration folder of 1.17 or earlier still on disk.
/// The SDK writes it nowhere: the caller shows it once at startup.
#[must_use]
pub fn legacy_warning() -> Option<String> {
    legacy(&|name| std::env::var_os(name), &Path::is_dir).message()
}

/// The cache folder. Its content can be deleted at any time.
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

/// The credentials file, in the `.env` format. A program that writes it must
/// give it the permissions `0600`.
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

enum Legacy {
    Absent,
    InUse { old: PathBuf, new: PathBuf },
    Ignored { old: PathBuf, new: PathBuf },
}

impl Legacy {
    fn message(&self) -> Option<String> {
        match self {
            Self::Absent => None,
            Self::InUse { old, new } => Some(format!(
                "the configuration folder {} is deprecated, and a major version stops \
                 reading it: move it to {}",
                old.display(),
                new.display()
            )),
            Self::Ignored { old, new } => Some(format!(
                "the configuration folder {} is ignored, because {} exists: move what you \
                 need from the first one to the second one, then delete the first one",
                old.display(),
                new.display()
            )),
        }
    }
}

fn legacy(var: &dyn Fn(&str) -> Option<OsString>, exists: &dyn Fn(&Path) -> bool) -> Legacy {
    if !cfg!(any(target_os = "macos", windows)) {
        return Legacy::Absent;
    }
    // The rule of 1.17 and earlier, on every system.
    let root = var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".config")));
    let Some(old) = root.map(|root| root.join(APP)) else {
        return Legacy::Absent;
    };
    let new = dirs(var).config;
    if old == new || !exists(&old) {
        Legacy::Absent
    } else if exists(&new) {
        Legacy::Ignored { old, new }
    } else {
        Legacy::InUse { old, new }
    }
}

struct Dirs {
    config: PathBuf,
    cache: PathBuf,
}

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

    #[cfg(any(target_os = "macos", windows))]
    #[test]
    fn the_old_folder_is_read_while_it_is_the_only_one() {
        let var = |name: &str| (name == "HOME").then(|| OsString::from("/h"));
        let old = PathBuf::from("/h/.config/govee-toolkit");
        let new = dirs(&var).config;

        let only_old = legacy(&var, &|p| p == old);
        assert!(matches!(&only_old, Legacy::InUse { old: o, .. } if *o == old));
        assert!(only_old.message().is_some());

        let both = legacy(&var, &|_| true);
        assert!(matches!(both, Legacy::Ignored { .. }));

        assert!(matches!(legacy(&var, &|p| p == new), Legacy::Absent));
        assert!(matches!(legacy(&var, &|_| false), Legacy::Absent));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_has_no_old_folder() {
        let var = |name: &str| (name == "HOME").then(|| OsString::from("/h"));
        assert!(matches!(legacy(&var, &|_| true), Legacy::Absent));
    }

    #[test]
    fn the_credentials_sit_beside_the_configuration() {
        assert_eq!(credentials_file().parent(), Some(config_dir().as_path()));
    }
}
