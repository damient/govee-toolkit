//! `POST /api/reset`: put the Studio settings back to the defaults, remove
//! the device entries of `config.yaml`, or both. The other keys of the file
//! stay, and so do the API key and the sharing switch.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use govee_toolkit::Config;
use serde::Deserialize;
use serde_json::json;
use serde_norway::Value;

use crate::backup::Backup;
use crate::settings::refuse;
use crate::share::Local;
use crate::storage::{self, Storage};
use crate::studio::StudioSettings;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reset {
    /// Theme, language and the `advanced` switch.
    #[serde(default)]
    prefs: bool,
    /// The `devices:` key of `config.yaml`: names, modes and groups.
    #[serde(default)]
    devices: bool,
}

/// The text of `config` with no `devices:` key. `None` when the file has no
/// such key, and so stays as it is. The comments do not survive.
fn without_devices(config: &str) -> Result<Option<String>, String> {
    let invalid = |e: serde_norway::Error| format!("the configuration file is not valid: {e}");
    if config.trim().is_empty() {
        return Ok(None);
    }
    let mut root: Value = serde_norway::from_str(config).map_err(invalid)?;
    if root.is_null() {
        return Ok(None);
    }
    let Value::Mapping(map) = &mut root else {
        return Err("the configuration file is not valid: it is not a mapping".to_owned());
    };
    if map.remove("devices").is_none() {
        return Ok(None);
    }
    let text = serde_norway::to_string(&root).map_err(invalid)?;
    serde_norway::from_str::<Config>(&text).map_err(invalid)?;
    Ok(Some(text))
}

fn forget_devices(storage: &Storage) -> Result<(), (StatusCode, String)> {
    let internal = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    let Some(config) = storage::read(&storage.config).map_err(internal)? else {
        return Ok(());
    };
    match without_devices(&config).map_err(|e| (StatusCode::CONFLICT, e))? {
        Some(text) => storage::write(&storage.config, text.as_bytes(), false).map_err(internal),
        None => Ok(()),
    }
}

/// Only a client on this machine can reset, as for an import.
pub(crate) async fn reset(
    State(backup): State<Arc<Backup>>,
    Extension(local): Extension<Local>,
    Json(body): Json<Reset>,
) -> Response {
    let storage = match backup.admit(local) {
        Ok(storage) => storage,
        Err((status, error)) => return refuse(status, error),
    };
    if !body.prefs && !body.devices {
        return refuse(StatusCode::BAD_REQUEST, "choose what to reset");
    }
    if body.devices {
        if let Err((status, error)) = forget_devices(storage) {
            return refuse(status, &error);
        }
        backup.sdk.reset().await;
    }
    if body.prefs
        && let Err(error) = backup
            .store
            .update(|studio| studio.studio.settings = StudioSettings::default())
    {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    Json(json!({ "prefs": body.prefs, "devices": body.devices })).into_response()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn the_device_entries_go_and_the_other_keys_stay() {
        let config = "lan:\n  status_timeout_ms: 500\ndevices:\n  AA:BB:CC:DD:EE:FF:00:11:\n    name: Desk\n";
        let text = without_devices(config).unwrap().unwrap();
        assert!(!text.contains("devices"), "{text}");
        assert!(text.contains("status_timeout_ms: 500"), "{text}");
    }

    #[test]
    fn a_file_with_no_device_entry_stays_as_it_is() {
        assert_eq!(without_devices("# only a comment\n").unwrap(), None);
        assert_eq!(without_devices("lan: {}\n").unwrap(), None);
        assert_eq!(without_devices("").unwrap(), None);
    }
}
