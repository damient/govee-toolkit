//! `PUT /api/devices/{id}/name`: the name of a device in `config.yaml`.
//!
//! The edit goes through a YAML tree, so the other keys of the file stay as
//! they are. The comments of the file do not survive a write.

use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as Route, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::Config;
use serde::Deserialize;
use serde_json::json;
use serde_norway::{Mapping, Value};

use crate::devices::Sdk;
use crate::storage;

/// The longest name, in characters.
const NAME_MAX: usize = 64;

#[derive(Debug, Deserialize)]
pub(crate) struct Rename {
    /// `null` or a blank string removes the name.
    name: Option<String>,
}

fn refuse(status: StatusCode, error: &str) -> Response {
    (status, Json(json!({ "error": error }))).into_response()
}

/// The mapping under `key` in `map`, made when it is missing.
fn child(map: &mut Mapping, key: Value) -> Result<&mut Mapping, String> {
    match map.entry(key).or_insert_with(|| Mapping::new().into()) {
        Value::Mapping(child) => Ok(child),
        _ => Err("the configuration file is not valid: a device entry is not a mapping".to_owned()),
    }
}

/// The text of `config` with the name of `id` set to `name`, or removed.
fn renamed(config: &str, id: &str, name: Option<&str>) -> Result<String, String> {
    let invalid = |e: serde_norway::Error| format!("the configuration file is not valid: {e}");
    let mut root = if config.trim().is_empty() {
        Value::Mapping(Mapping::new())
    } else {
        serde_norway::from_str(config).map_err(invalid)?
    };
    let Value::Mapping(map) = &mut root else {
        return Err("the configuration file is not valid: it is not a mapping".to_owned());
    };
    let devices = child(map, "devices".into())?;
    // A key matches whatever its case, as in the SDK.
    let key = devices
        .keys()
        .find(|key| key.as_str().is_some_and(|key| key.eq_ignore_ascii_case(id)))
        .cloned()
        .unwrap_or_else(|| id.into());
    let entry = child(devices, key.clone())?;
    if let Some(name) = name {
        entry.insert("name".into(), name.into());
    } else {
        entry.remove("name");
        if entry.is_empty() {
            devices.remove(&key);
        }
    }
    let text = serde_norway::to_string(&root).map_err(invalid)?;
    serde_norway::from_str::<Config>(&text).map_err(invalid)?;
    Ok(text)
}

fn write(path: &Path, id: &str, name: Option<&str>) -> Result<(), (StatusCode, String)> {
    let internal = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    let config = storage::read(path).map_err(internal)?.unwrap_or_default();
    let text = renamed(&config, id, name).map_err(|e| (StatusCode::CONFLICT, e))?;
    storage::write(path, text.as_bytes(), false).map_err(internal)
}

pub(crate) async fn put(
    State(sdk): State<Arc<Sdk>>,
    Route(id): Route<String>,
    Json(body): Json<Rename>,
) -> Response {
    let Some(storage) = sdk.storage() else {
        return refuse(
            StatusCode::NOT_FOUND,
            "this server keeps no configuration file",
        );
    };
    let name = body
        .name
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    if name
        .as_ref()
        .is_some_and(|name| name.chars().count() > NAME_MAX)
    {
        return refuse(
            StatusCode::BAD_REQUEST,
            "the name is longer than 64 characters",
        );
    }
    match write(&storage.config, &id, name.as_deref()) {
        Ok(()) => Json(json!({ "id": id, "name": name })).into_response(),
        Err((status, error)) => refuse(status, &error),
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn a_name_goes_under_the_existing_key_and_keeps_the_rest() {
        let config = "defaults:\n  modes: [lan]\ndevices:\n  aa:bb:\n    modes: [lan]\n";
        let text = renamed(config, "AA:BB", Some("desk")).unwrap_or_default();
        let value: Value = serde_norway::from_str(&text).unwrap_or_default();
        assert_eq!(value["devices"]["aa:bb"]["name"], "desk");
        assert_eq!(value["devices"]["aa:bb"]["modes"][0], "lan");
        assert_eq!(value["defaults"]["modes"][0], "lan");
    }

    #[test]
    fn an_empty_file_gets_a_device_entry() {
        let text = renamed("", "AA:BB", Some("desk")).unwrap_or_default();
        assert!(text.contains("AA:BB"), "{text}");
        assert!(text.contains("desk"), "{text}");
    }

    #[test]
    fn removing_the_last_key_removes_the_entry() {
        let text =
            renamed("devices:\n  AA:BB:\n    name: desk\n", "AA:BB", None).unwrap_or_default();
        let value: Value = serde_norway::from_str(&text).unwrap_or_default();
        assert_eq!(value["devices"].as_mapping().map(Mapping::len), Some(0));
    }

    #[test]
    fn a_file_that_does_not_parse_is_refused() {
        assert!(renamed("devices: [", "AA:BB", Some("desk")).is_err());
    }
}
