//! `/api/backup`. The document can carry the API key, so only a client on
//! this machine can export or import it.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

use axum::Extension;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use govee_toolkit::config::{CloudConfig, KEY_ENV};
use govee_toolkit::{Config, Env};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::devices::Sdk;
use crate::rooms::{self, Rooms};
use crate::share::{Local, Share};
use crate::storage::{self, Storage};
use crate::studio::{Store, Studio};
use crate::{compat, refuse};

const FORMAT: &str = "govee-toolkit-backup";
/// The parse error names fields that a user does not know.
const NOT_A_BACKUP: &str = "this file is not a backup of govee-toolkit";
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document<Server = serde_json::Value, Part = serde_json::Value> {
    format: String,
    #[serde(default)]
    server: Server,
    #[serde(default)]
    studio: Part,
    /// The text of the configuration file, comments included.
    config: Option<String>,
    credentials: Option<BTreeMap<String, String>>,
    /// The content of `rooms.json`. A backup with no `rooms` holds no room.
    #[serde(default)]
    rooms: Option<Rooms>,
}

#[derive(Debug)]
pub(crate) struct Backup {
    pub(crate) storage: Option<Storage>,
    pub(crate) store: Arc<Store>,
    pub(crate) share: Arc<Share>,
    pub(crate) sdk: Arc<Sdk>,
    pub(crate) rooms: Arc<rooms::Store>,
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

pub(crate) async fn export(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    let storage = storage.clone();
    let shared = Arc::clone(&backup);
    let document = storage::blocking(move || {
        let config = storage::read(&storage.config)?;
        let key = key(&storage, config.as_deref()).map_err(storage::invalid)?;
        let backup = shared;
        let settings = backup.store.get();
        let document = Document {
            format: FORMAT.to_owned(),
            server: settings.server,
            studio: settings.studio,
            config,
            credentials: key.map(|key| BTreeMap::from([(KEY_ENV.to_owned(), key)])),
            rooms: Some(backup.rooms.get()),
        };
        serde_json::to_string_pretty(&document).map_err(storage::invalid)
    })
    .await;
    match document {
        Ok(text) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                // A name here wins over the local date that Studio gives.
                (header::CONTENT_DISPOSITION, "attachment"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            text,
        )
            .into_response(),
        Err(error) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
    }
}

#[derive(Debug)]
struct Plan {
    config: Option<String>,
    studio: Studio,
    credentials: Option<String>,
    rooms: Option<Rooms>,
}

fn validate(body: &[u8], credentials: &Path) -> Result<Plan, String> {
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
        Some(vars) if !vars.is_empty() => {
            let vars = vars
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()));
            Some(Env::file_text(credentials, vars).map_err(|e| e.to_string())?)
        }
        _ => None,
    };
    let rooms = document.rooms.map(Rooms::checked).transpose()?;
    Ok(Plan {
        config: document.config,
        studio,
        credentials,
        rooms,
    })
}

fn write(backup: &Backup, storage: &Storage, plan: Plan, keep: bool) -> io::Result<()> {
    storage::put(
        &storage.credentials(),
        plan.credentials.as_deref(),
        true,
        keep,
    )?;
    storage::put(&storage.config, plan.config.as_deref(), false, keep)?;
    backup.rooms.set(plan.rooms, keep)?;
    backup.store.replace(plan.studio, keep)
}

async fn settle(backup: &Backup, share: bool) {
    // As at start: a network address that is not available leaves sharing off.
    let _ = backup.share.apply(share);
    backup.sdk.reload().await;
}

/// Nothing is written unless the whole document is valid.
pub(crate) async fn import(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
    body: Bytes,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    let plan = match validate(&body, &storage.credentials()) {
        Ok(plan) => plan,
        Err(error) => return refuse(StatusCode::BAD_REQUEST, &error),
    };
    let share = plan.studio.server.settings.share;
    let (shared, storage) = (Arc::clone(&backup), storage.clone());
    let written = storage::blocking(move || write(&shared, &storage, plan, true)).await;
    if let Err(error) = written {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    settle(&backup, share).await;
    axum::Json(json!({ "imported": true })).into_response()
}

fn copies(storage: &Storage) -> [std::path::PathBuf; 4] {
    [
        storage.credentials(),
        storage.config.clone(),
        storage.studio(),
        storage.rooms(),
    ]
}

pub(crate) async fn restorable(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    let files = copies(storage);
    let found = storage::blocking(move || Ok(files.iter().any(|path| storage::bak(path).exists())));
    match found.await {
        Ok(restorable) => axum::Json(json!({ "restorable": restorable })).into_response(),
        Err(error) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
    }
}

fn kept(storage: &Storage) -> io::Result<Option<Plan>> {
    if !copies(storage)
        .iter()
        .any(|path| storage::bak(path).exists())
    {
        return Ok(None);
    }
    let read = |path: &Path| storage::read(&storage::bak(path));
    let config = read(&storage.config)?;
    if let Some(text) = &config {
        parse_config(text).map_err(storage::invalid)?;
    }
    let studio = match read(&storage.studio())? {
        Some(text) => {
            let value = serde_json::from_str(&text).map_err(storage::invalid)?;
            Studio::from_value(value).map_err(storage::invalid)?
        }
        None => Studio::default(),
    };
    Ok(Some(Plan {
        config,
        studio,
        credentials: read(&storage.credentials())?,
        rooms: rooms::read(&storage::bak(&storage.rooms()))?,
    }))
}

/// A file with no `.import.bak` copy did not exist, and goes.
pub(crate) async fn restore(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage.clone(),
        Err((status, error)) => return refuse(status, error),
    };
    let shared = Arc::clone(&backup);
    let restored = storage::blocking(move || {
        let Some(plan) = kept(&storage)? else {
            return Ok(None);
        };
        let share = plan.studio.server.settings.share;
        write(&shared, &storage, plan, false)?;
        for path in copies(&storage) {
            storage::remove(&storage::bak(&path))?;
        }
        Ok(Some(share))
    })
    .await;
    match restored {
        Ok(Some(share)) => {
            settle(&backup, share).await;
            axum::Json(json!({ "restored": true })).into_response()
        }
        Ok(None) => refuse(StatusCode::CONFLICT, "there is no import to undo"),
        Err(error) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn validate_doc(body: &[u8]) -> Result<Plan, String> {
        validate(body, Path::new("credentials.env"))
    }

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
            rooms: Arc::new(rooms::Store::default()),
        };
        assert_eq!(
            backup.admit(Local(false)).unwrap_err().0,
            StatusCode::FORBIDDEN
        );
        assert!(backup.admit(Local(true)).is_ok());
    }

    #[test]
    fn a_credential_is_written_between_single_quotes() {
        let plan = validate_doc(&document(&json!({ "GOVEE_API_KEY": "a-key" }))).unwrap();
        let text = plan.credentials.unwrap();
        assert!(text.ends_with("GOVEE_API_KEY='a-key'\n"), "{text}");
    }

    #[test]
    fn a_credential_that_the_file_cannot_hold_is_refused() {
        assert!(validate_doc(&document(&json!({ "GOVEE_API_KEY": "a'b" }))).is_err());
        assert!(validate_doc(&document(&json!({ "GOVEE_API_KEY": "a\nb" }))).is_err());
        assert!(validate_doc(&document(&json!({ "PATH": "x" }))).is_err());
    }
}
