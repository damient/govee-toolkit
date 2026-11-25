//! The library of each model, as the cache folder keeps it.

use std::path::PathBuf;

use super::{Error, Scene, parse};

/// The cache file of `sku`.
///
/// # Errors
///
/// [`Error::BadSku`] if `sku` is empty or holds a character other than an
/// ASCII letter, a digit or `-`. The name becomes a file name.
pub(super) fn file(sku: &str) -> Result<PathBuf, Error> {
    let fits = !sku.is_empty() && sku.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !fits {
        return Err(Error::BadSku {
            sku: sku.to_owned(),
        });
    }
    Ok(crate::paths::cache_dir()
        .join("scenes")
        .join(format!("{sku}.json")))
}

/// Read the cached library of `sku`.
///
/// # Errors
///
/// [`Error::NotFetched`] if the cache holds none, [`Error::Unreadable`] if the
/// file does not read as a library, and [`Error::BadSku`] as for the file name.
pub fn load(sku: &str) -> Result<Vec<Scene>, Error> {
    let path = file(sku)?;
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::NotFetched {
                sku: sku.to_owned(),
            });
        }
        Err(e) => {
            return Err(Error::Unreadable {
                path,
                reason: e.to_string(),
            });
        }
    };
    parse(sku, &text, &path)
}

/// Check that `text` reads as a library of `sku`, then write it to the cache.
/// Returns the scenes it holds.
///
/// # Errors
///
/// As for [`parse`], plus [`Error::Unreadable`] if the file cannot be
/// written.
pub fn store(sku: &str, text: &str) -> Result<Vec<Scene>, Error> {
    let path = file(sku)?;
    let scenes = parse(sku, text, &path)?;
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, text)
    };
    write().map_err(|e| Error::Unreadable {
        path: path.clone(),
        reason: e.to_string(),
    })?;
    Ok(scenes)
}
