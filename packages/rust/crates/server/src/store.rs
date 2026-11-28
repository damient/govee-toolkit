//! A JSON file that the server holds in memory and writes through, so that
//! every client reads the same content.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::storage;

/// The content of a [`JsonStore`].
pub(crate) trait Checked: Sized + Default + Clone + Serialize + DeserializeOwned {
    /// The content as the server keeps it, or a message that names what is
    /// not valid.
    fn checked(self) -> Result<Self, String>;
}

#[derive(Debug, Default)]
pub(crate) struct JsonStore<T> {
    /// `None` keeps the content in memory alone.
    path: Option<PathBuf>,
    data: Mutex<T>,
}

impl<T: Checked> JsonStore<T> {
    /// A file that does not exist is the default.
    pub(crate) fn open(path: Option<PathBuf>) -> io::Result<Self> {
        let data = match &path {
            Some(path) => read(path)?.unwrap_or_default(),
            None => T::default(),
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
        self.data.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn get(&self) -> T {
        self.lock().clone()
    }

    /// `None` deletes the file. `keep` copies it to `.import.bak` first. A
    /// failed write leaves the content as it was.
    pub(crate) fn set(&self, value: Option<T>, keep: bool) -> io::Result<()> {
        let mut data = self.lock();
        self.write(value.as_ref(), keep)?;
        *data = value.unwrap_or_default();
        Ok(())
    }

    /// Write the file, where the store has one. The caller holds the lock.
    pub(crate) fn write(&self, value: Option<&T>, keep: bool) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let text = value.map(storage::json_text).transpose()?;
        storage::put(path, text.as_deref(), false, keep)
    }
}

/// The content of `path`, checked. `None` where the file does not exist.
pub(crate) fn read<T: Checked>(path: &Path) -> io::Result<Option<T>> {
    let Some(text) = storage::read(path)? else {
        return Ok(None);
    };
    let invalid = |e: String| storage::invalid(format!("{}: {e}", path.display()));
    let value: T = serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
    value.checked().map(Some).map_err(invalid)
}
