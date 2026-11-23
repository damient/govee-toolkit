use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use govee_toolkit::config::{self, DeviceEdit};
use govee_toolkit::env::Env;
use govee_toolkit::{Config, paths};
use serde::Serialize;

/// The files that the server reads and writes.
#[derive(Debug, Clone)]
pub struct Storage {
    /// The folder of `studio.json`, `rooms.json` and `credentials.env`.
    pub dir: PathBuf,
    /// The configuration file of the SDK.
    pub config: PathBuf,
    /// `false` reads the `credentials.env` of [`Storage::dir`] alone.
    pub system_env: bool,
}

impl Storage {
    /// The files that the SDK reads, `GOVEE_CONFIG` included.
    ///
    /// # Errors
    ///
    /// `InvalidData` when a `.env` file does not parse.
    pub fn system() -> io::Result<Self> {
        let env = Env::load().map_err(invalid)?;
        Ok(Self {
            dir: paths::config_dir(),
            config: paths::config_file_from(&env),
            system_env: true,
        })
    }

    /// Every file in `dir`, and no variable from the process.
    #[must_use]
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            config: dir.join("config.yaml"),
            dir,
            system_env: false,
        }
    }

    pub(crate) fn studio(&self) -> PathBuf {
        self.dir.join("studio.json")
    }

    pub(crate) fn rooms(&self) -> PathBuf {
        self.dir.join("rooms.json")
    }

    pub(crate) fn credentials(&self) -> PathBuf {
        self.dir.join("credentials.env")
    }

    pub(crate) fn env(&self) -> govee_toolkit::Result<Env> {
        if self.system_env {
            Env::load()
        } else {
            Env::from_file_alone(self.credentials())
        }
    }

    pub(crate) fn sdk_config(&self) -> govee_toolkit::Result<Config> {
        Config::load_from_with(&self.config, self.env()?)
    }
}

pub(crate) fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

/// `false` when nothing changes. `InvalidData` for an invalid configuration.
pub(crate) fn edit_config(path: &Path, edits: &[DeviceEdit]) -> io::Result<bool> {
    let text = read(path)?.unwrap_or_default();
    match config::edit_devices(path, &text, edits).map_err(invalid)? {
        Some(text) => write(path, text.as_bytes(), false).map(|()| true),
        None => Ok(false),
    }
}

/// A write waits for the disk. The `lan` send path shares the async workers.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(io::Error::other)?
}

pub(crate) fn json_text(value: &impl Serialize) -> io::Result<String> {
    let mut text = serde_json::to_string_pretty(value).map_err(invalid)?;
    text.push('\n');
    Ok(text)
}

/// `None` deletes the file. `keep` makes a [`keep`] copy first.
pub(crate) fn put(path: &Path, text: Option<&str>, private: bool, keep: bool) -> io::Result<()> {
    if keep {
        self::keep(path)?;
    }
    match text {
        Some(text) => write(path, text.as_bytes(), private),
        None => remove(path),
    }
}

pub(crate) fn read(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// A name of its own: a `.bak` beside the file can belong to the user.
pub(crate) fn bak(path: &Path) -> PathBuf {
    let mut bak = path.as_os_str().to_owned();
    bak.push(".import.bak");
    PathBuf::from(bak)
}

/// When `path` does not exist, an old copy goes: no copy means no file.
pub(crate) fn keep(path: &Path) -> io::Result<()> {
    match fs::copy(path, bak(path)) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => remove(&bak(path)),
        Err(e) => Err(e),
    }
}

/// `private` is `0600` on Unix. On Windows, the user folder keeps others out.
pub(crate) fn write(path: &Path, bytes: &[u8], private: bool) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .ok_or_else(|| invalid(format!("{}: not a file", path.display())))?;
    let temporary = dir.join(format!(
        ".{}.{}-{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    // A file left by a crash keeps its permissions: `mode` applies on create.
    let _ = fs::remove_file(&temporary);
    let result = (|| {
        let mut file = options(private).open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn options(private: bool) -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    options
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("govee-server-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_write_replaces_the_file_and_leaves_no_temporary() {
        let dir = dir("write");
        let path = dir.join("a.json");
        write(&path, b"one", false).unwrap();
        write(&path, b"two", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keep_copies_the_file_beside_it() {
        let dir = dir("keep");
        let path = dir.join("a.json");
        keep(&path).unwrap();
        write(&path, b"old", false).unwrap();
        keep(&path).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("a.json.import.bak")).unwrap(),
            "old"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_private_file_is_for_the_user_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir("private");
        let path = dir.join("credentials.env");
        write(&path, b"GOVEE_API_KEY='k'\n", true).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        fs::remove_dir_all(&dir).unwrap();
    }
}
