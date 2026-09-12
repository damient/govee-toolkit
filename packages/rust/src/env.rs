//! The `GOVEE_*` variables one run reads, and the `.env` file that can supply
//! them.
//!
//! The values are collected into an [`Env`] and never exported into the
//! process: `std::env::set_var` is `unsafe` under edition 2024, and this
//! workspace forbids `unsafe`. A value the caller holds is also deterministic
//! under a test, which a process-wide variable is not.
//!
//! The process environment wins over the file, so a variable set on the command
//! line overrides what `.env` carries.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The variable that names a file to read instead of the discovered one.
pub const FILE_ENV: &str = "GOVEE_ENV_FILE";

/// The file name the search looks for in each directory.
pub const FILE_NAME: &str = ".env";

/// The prefix a name must carry to be read. A `.env` written for another
/// project can hold anything, and nothing outside this prefix reaches the
/// toolkit.
const PREFIX: &str = "GOVEE_";

/// The `GOVEE_*` variables one run reads.
///
/// Build one with [`Env::load`], which is what
/// [`Config::load`](crate::Config::load) does. [`Env::process`] reads no file,
/// and [`Env::from_pairs`] takes values the caller already holds.
#[derive(Clone, Default)]
pub struct Env {
    vars: BTreeMap<String, String>,
    source: Option<PathBuf>,
}

// The values are credentials. The names alone are what a bug report may carry.
impl std::fmt::Debug for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Env")
            .field("source", &self.source)
            .field("names", &self.vars.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Env {
    /// The process environment, plus the first `.env` that the search finds.
    ///
    /// The search starts in the working directory and goes up. It stops after
    /// the directory that holds `.git`, after the home directory, or at the
    /// root of the file system. `~/.config/govee-toolkit/.env` is read last.
    /// [`FILE_ENV`] replaces the search.
    ///
    /// A file the search does not find is not an error: `lan` and `ble` need
    /// no credential. A file that [`FILE_ENV`] names is an error when it is
    /// absent, because somebody asked for that file.
    ///
    /// # Errors
    ///
    /// [`Error::Env`] if a file cannot be read or does not parse.
    pub fn load() -> Result<Self> {
        let mut env = Self::process();
        if let Some(path) = std::env::var_os(FILE_ENV) {
            env.merge(&PathBuf::from(path), true)?;
            return Ok(env);
        }
        for path in search_path() {
            if env.merge(&path, false)? {
                break;
            }
        }
        Ok(env)
    }

    /// The process environment alone. No file is read.
    #[must_use]
    pub fn process() -> Self {
        let vars = std::env::vars_os()
            .filter_map(|(name, value)| {
                let name = name.into_string().ok()?;
                let value = value.into_string().ok()?;
                (name.starts_with(PREFIX) && !value.trim().is_empty()).then_some((name, value))
            })
            .collect();
        Self { vars, source: None }
    }

    /// The process environment, plus the file at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::Env`] if the file is absent, cannot be read, or does not
    /// parse. An absent file is an error here: the caller named it.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let mut env = Self::process();
        env.merge(path.as_ref(), true)?;
        Ok(env)
    }

    /// An [`Env`] from names and values the caller holds. Nothing is read from
    /// the process or from a file, and nothing is filtered.
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            vars: pairs
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
            source: None,
        }
    }

    /// The value of `name`, or `None` where nothing sets it. A value that is
    /// empty or blank answers `None`, since `.env.example` ships blank
    /// placeholders. The value is otherwise verbatim, spaces included.
    #[must_use]
    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// The file the values came from, where one was read.
    #[must_use]
    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    /// Read `path` into the names the process does not set. Answers whether
    /// the file was there.
    fn merge(&mut self, path: &Path, required: bool) -> Result<bool> {
        let entries = match dotenvy::from_path_iter(path) {
            Ok(entries) => entries,
            Err(e) if e.not_found() && !required => return Ok(false),
            Err(e) => return Err(fault(path, &e)),
        };
        for entry in entries {
            let (name, value) = entry.map_err(|e| fault(path, &e))?;
            if !name.starts_with(PREFIX) || value.trim().is_empty() {
                continue;
            }
            self.vars.entry(name).or_insert(value);
        }
        self.source = Some(path.to_owned());
        Ok(true)
    }
}

/// The files the search looks at, in order.
fn search_path() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            paths.push(dir.join(FILE_NAME));
            // The repository root and the home directory both end the walk: a
            // `.env` above either one belongs to something else.
            if dir.join(".git").exists() || home.as_ref().is_some_and(|home| *home == dir) {
                break;
            }
            let Some(parent) = dir.parent().map(Path::to_path_buf) else {
                break;
            };
            dir = parent;
        }
    }
    paths.push(crate::paths::config_dir().join(FILE_NAME));
    paths
}

fn fault(path: &Path, e: &dotenvy::Error) -> Error {
    Error::Env {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// Write `text` to a file of its own and answer the path.
    fn file(name: &str, text: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("govee-toolkit-test-{name}"));
        std::fs::write(&path, text).expect("the temporary file is writable");
        path
    }

    #[test]
    fn a_file_supplies_the_prefixed_names_alone() {
        let path = file(
            "env-prefix",
            "GOVEE_API_KEY=a-key\nAWS_SECRET_ACCESS_KEY=not-ours\n",
        );
        let env = Env::from_file(&path).expect("the file reads");
        assert_eq!(env.var("GOVEE_API_KEY"), Some("a-key"));
        assert_eq!(env.var("AWS_SECRET_ACCESS_KEY"), None);
        assert_eq!(env.source(), Some(path.as_path()));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_blank_value_is_a_placeholder_and_not_a_value() {
        let path = file("env-blank", "GOVEE_WIFI_SSID=\nGOVEE_WIFI_PASSWORD=   \n");
        let env = Env::from_file(&path).expect("the file reads");
        assert_eq!(env.var("GOVEE_WIFI_SSID"), None);
        assert_eq!(env.var("GOVEE_WIFI_PASSWORD"), None);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_quoted_value_keeps_its_spaces() {
        let path = file("env-quoted", "GOVEE_WIFI_PASSWORD=\"two words \"\n");
        let env = Env::from_file(&path).expect("the file reads");
        assert_eq!(env.var("GOVEE_WIFI_PASSWORD"), Some("two words "));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_named_file_that_is_absent_is_an_error() {
        let error = Env::from_file("/nonexistent/govee/.env").expect_err("the file is absent");
        assert_eq!(error.code(), "env");
    }

    #[test]
    fn a_search_that_finds_nothing_is_not_an_error() {
        let mut env = Env::default();
        assert!(
            !env.merge(Path::new("/nonexistent/govee/.env"), false)
                .unwrap()
        );
        assert_eq!(env.source(), None);
    }

    #[test]
    fn the_process_wins_over_the_file() {
        let path = file("env-precedence", "GOVEE_API_KEY=from-the-file\n");
        let mut env = Env::from_pairs([("GOVEE_API_KEY", "from-the-process")]);
        env.merge(&path, true).expect("the file reads");
        assert_eq!(env.var("GOVEE_API_KEY"), Some("from-the-process"));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn the_debug_form_carries_no_value() {
        let env = Env::from_pairs([("GOVEE_API_KEY", "a-key")]);
        let shown = format!("{env:?}");
        assert!(shown.contains("GOVEE_API_KEY"), "{shown}");
        assert!(!shown.contains("a-key"), "{shown}");
    }

    #[test]
    fn the_search_ends_at_the_repository_root() {
        let paths = search_path();
        assert!(
            paths.last().is_some_and(|last| last.ends_with(FILE_NAME)),
            "the configuration directory is searched last: {paths:?}"
        );
        assert!(paths.iter().all(|path| path.ends_with(FILE_NAME)));
    }
}
