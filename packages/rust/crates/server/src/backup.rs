//! `/api/backup`: `studio.json`, the configuration of the SDK and the API key
//! in one JSON document, and back. The document can carry the API key, so
//! only a client on this machine can export or import it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::path::Path;
use std::sync::Arc;

use axum::Extension;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use govee_toolkit::Config;
use govee_toolkit::config::{CloudConfig, KEY_ENV};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::compat;
use crate::devices::Sdk;
use crate::settings::refuse;
use crate::share::{Local, Share};
use crate::storage::{self, Storage};
use crate::studio::{Store, Studio};

/// The value of `format` in a backup.
const FORMAT: &str = "govee-toolkit-backup";
/// The answer to a file that is not a backup. The parse error names fields
/// that a user does not know.
const NOT_A_BACKUP: &str = "this file is not a backup of govee-toolkit";
/// The content of `studio.json`, with the configuration file and the
/// credentials beside it. An import reads the two parts as JSON values first:
/// [`Studio::from_value`] checks their versions.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document<Server = serde_json::Value, Part = serde_json::Value> {
    format: String,
    #[serde(default)]
    server: Server,
    #[serde(default)]
    studio: Part,
    /// The text of the configuration file, comments included. `null` when
    /// there is no file.
    config: Option<String>,
    /// `GOVEE_*` variables, such as the API key. `null` when none is set.
    credentials: Option<BTreeMap<String, String>>,
}

/// What an import replaces, and what it starts again.
#[derive(Debug)]
pub(crate) struct Backup {
    pub(crate) storage: Option<Storage>,
    pub(crate) store: Arc<Store>,
    pub(crate) share: Arc<Share>,
    pub(crate) sdk: Arc<Sdk>,
}

impl Backup {
    pub(crate) fn admit(&self, local: Local) -> Result<&Storage, (StatusCode, &'static str)> {
        if !local.0 {
            return Err((
                StatusCode::FORBIDDEN,
                "only this computer can export or import",
            ));
        }
        self.storage
            .as_ref()
            .ok_or((StatusCode::NOT_FOUND, "this server keeps no files"))
    }
}

fn parse_config(text: &str) -> Result<Config, String> {
    serde_norway::from_str(text).map_err(|e| format!("the configuration file is not valid: {e}"))
}

fn key(storage: &Storage, config: Option<&str>) -> Result<Option<String>, String> {
    let env = storage.env().map_err(|e| e.to_string())?;
    let cloud = match config {
        Some(text) => parse_config(text)?.cloud,
        None => CloudConfig::default(),
    };
    cloud.key(&env).map_err(|e| e.to_string())
}

/// `GET /api/backup`: the document, as a file to download.
pub(crate) async fn export(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    let document = (|| {
        let config = storage::read(&storage.config).map_err(|e| e.to_string())?;
        let key = key(storage, config.as_deref())?;
        let settings = backup.store.get();
        let document = Document {
            format: FORMAT.to_owned(),
            server: settings.server,
            studio: settings.studio,
            config,
            credentials: key.map(|key| BTreeMap::from([(KEY_ENV.to_owned(), key)])),
        };
        serde_json::to_string_pretty(&document).map_err(|e| e.to_string())
    })();
    match document {
        Ok(text) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                // Studio names the file with the local date and time: a name
                // here would win over the one that Studio gives.
                (header::CONTENT_DISPOSITION, "attachment"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            text,
        )
            .into_response(),
        Err(error) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &error),
    }
}

/// The files of a valid document.
#[derive(Debug)]
struct Plan {
    config: Option<String>,
    studio: Studio,
    credentials: Option<String>,
}

fn validate(body: &[u8]) -> Result<Plan, String> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| NOT_A_BACKUP.to_owned())?;
    if value.get("format").and_then(serde_json::Value::as_str) != Some(FORMAT) {
        return Err(NOT_A_BACKUP.to_owned());
    }
    // Before the strict parse: a newer server can write keys that this one
    // does not know, and the user must read "update", not "not a backup".
    let mut parts = serde_json::Map::new();
    for key in ["server", "studio"] {
        if let Some(part) = value.get(key) {
            parts.insert(key.to_owned(), part.clone());
        }
    }
    let studio = Studio::from_value(serde_json::Value::Object(parts))?;
    let server = value
        .pointer("/server/version")
        .and_then(serde_json::Value::as_str)
        .filter(|server| compat::newer_server(server))
        .map(str::to_owned);
    let document: Document = serde_json::from_value(value).map_err(|_| match server {
        Some(server) => compat::refusal(crate::studio::VERSION, Some(&server)),
        None => NOT_A_BACKUP.to_owned(),
    })?;
    if let Some(text) = &document.config {
        parse_config(text)?;
    }
    let credentials = match document.credentials {
        Some(vars) if !vars.is_empty() => Some(credentials_text(&vars)?),
        _ => None,
    };
    Ok(Plan {
        config: document.config,
        studio,
        credentials,
    })
}

/// The credentials file in the `.env` format. A value goes between single
/// quotes, which `.env` reads verbatim, so a value cannot hold one.
fn credentials_text(vars: &BTreeMap<String, String>) -> Result<String, String> {
    let mut text =
        String::from("# GOVEE_* variables, written by govee-server. Keep this file private.\n");
    for (name, value) in vars {
        let named = name.starts_with("GOVEE_")
            && name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
        if !named {
            return Err(format!("credentials: `{name}` is not a GOVEE_* name"));
        }
        if value.trim().is_empty() || value.contains('\'') || value.chars().any(char::is_control) {
            return Err(format!("credentials: the value of {name} cannot be stored"));
        }
        let _ = writeln!(text, "{name}='{value}'");
    }
    Ok(text)
}

fn replace(path: &Path, text: Option<&str>, private: bool) -> io::Result<()> {
    storage::keep(path)?;
    match text {
        Some(text) => storage::write(path, text.as_bytes(), private),
        None => storage::remove(path),
    }
}

/// `POST /api/backup` with a document: replace every file, each after a
/// `.import.bak` copy. Nothing is written unless the whole document is valid.
pub(crate) async fn import(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
    body: Bytes,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    let plan = match validate(&body) {
        Ok(plan) => plan,
        Err(error) => return refuse(StatusCode::BAD_REQUEST, &error),
    };
    let share = plan.studio.server.settings.share;
    let written = replace(&storage.credentials(), plan.credentials.as_deref(), true)
        .and_then(|()| replace(&storage.config, plan.config.as_deref(), false))
        .and_then(|()| backup.store.replace(plan.studio, true));
    if let Err(error) = written {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    // As at start: a network address that is not available leaves sharing off.
    let _ = backup.share.apply(share);
    backup.sdk.reset().await;
    axum::Json(json!({ "imported": true })).into_response()
}

fn copies(storage: &Storage) -> [std::path::PathBuf; 3] {
    [
        storage.credentials(),
        storage.config.clone(),
        storage.studio(),
    ]
}

/// `POST /api/backup/restore`: put back the files as they were before the
/// last import, from the `.import.bak` copies, then delete the copies. A file
/// with no copy did not exist, and goes.
pub(crate) async fn restore(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    let files = copies(storage);
    if !files.iter().any(|path| storage::bak(path).exists()) {
        return refuse(StatusCode::CONFLICT, "there is no import to undo");
    }
    let read = |path: &Path| storage::read(&storage::bak(path)).map_err(|e| e.to_string());
    let plan = (|| {
        let config = read(&storage.config)?;
        if let Some(text) = &config {
            parse_config(text)?;
        }
        let studio = match read(&storage.studio())? {
            Some(text) => {
                Studio::from_value(serde_json::from_str(&text).map_err(|e| e.to_string())?)?
            }
            None => Studio::default(),
        };
        Ok::<_, String>(Plan {
            config,
            studio,
            credentials: read(&storage.credentials())?,
        })
    })();
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error),
    };
    let share = plan.studio.server.settings.share;
    let put = |path: &Path, text: Option<&str>, private: bool| match text {
        Some(text) => storage::write(path, text.as_bytes(), private),
        None => storage::remove(path),
    };
    let written = put(&storage.credentials(), plan.credentials.as_deref(), true)
        .and_then(|()| put(&storage.config, plan.config.as_deref(), false))
        .and_then(|()| backup.store.replace(plan.studio, false))
        .and_then(|()| {
            files
                .iter()
                .try_for_each(|path| storage::remove(&storage::bak(path)))
        });
    if let Err(error) = written {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    let _ = backup.share.apply(share);
    backup.sdk.reset().await;
    axum::Json(json!({ "restored": true })).into_response()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn document(credentials: &serde_json::Value) -> Vec<u8> {
        json!({
            "format": FORMAT,
            "server": { "version": "0.1.0" },
            "studio": { "version": "0.1.0" },
            "config": null,
            "credentials": credentials,
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn only_a_client_on_this_machine_can_export_or_import() {
        let store = Arc::new(Store::default());
        let backup = Backup {
            storage: Some(Storage::in_dir(std::env::temp_dir())),
            share: Arc::new(Share::new(None, Arc::clone(&store))),
            store,
            sdk: Arc::new(Sdk::default()),
        };
        assert_eq!(
            backup.admit(Local(false)).unwrap_err().0,
            StatusCode::FORBIDDEN
        );
        assert!(backup.admit(Local(true)).is_ok());
    }

    #[test]
    fn a_credential_is_written_between_single_quotes() {
        let plan = validate(&document(&json!({ "GOVEE_API_KEY": "a-key" }))).unwrap();
        let text = plan.credentials.unwrap();
        assert!(text.ends_with("GOVEE_API_KEY='a-key'\n"), "{text}");
    }

    #[test]
    fn a_credential_that_the_file_cannot_hold_is_refused() {
        assert!(validate(&document(&json!({ "GOVEE_API_KEY": "a'b" }))).is_err());
        assert!(validate(&document(&json!({ "GOVEE_API_KEY": "a\nb" }))).is_err());
        assert!(validate(&document(&json!({ "PATH": "x" }))).is_err());
    }
}
