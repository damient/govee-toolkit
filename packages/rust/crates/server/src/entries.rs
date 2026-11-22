//! `PUT /api/devices`: the device entries of `config.yaml`, in one write. A
//! device that the request removes leaves its rooms too.
//!
//! The edit goes through a YAML tree, so the other keys of the file stay as
//! they are. The comments of the file do not survive a write.

use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
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

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Entries {
    keep: Vec<Keep>,
    remove: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Keep {
    id: String,
    /// Written in an entry that has no SKU. A SKU in the file stays.
    #[serde(default)]
    sku: Option<String>,
    /// `null` or a blank string removes the name.
    #[serde(default)]
    name: Option<String>,
}

impl Keep {
    fn name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }
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

/// The key of `id` in `devices`. A key matches whatever its case, as in the
/// SDK.
fn key_of(devices: &Mapping, id: &str) -> Value {
    devices
        .keys()
        .find(|key| key.as_str().is_some_and(|key| key.eq_ignore_ascii_case(id)))
        .cloned()
        .unwrap_or_else(|| id.into())
}

/// The text of `config` with `entries` applied. An entry with no SKU gets
/// the SKU of the request, so that a read with no scan knows the device file.
/// A new entry also gets `modes: [lan]`, because the device answered a `lan`
/// scan.
fn edited(config: &str, entries: &Entries) -> Result<String, String> {
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
    for keep in &entries.keep {
        let key = key_of(devices, &keep.id);
        let made = !devices.contains_key(&key);
        let entry = child(devices, key)?;
        if let Some(sku) = &keep.sku
            && !entry.contains_key("sku")
        {
            entry.insert("sku".into(), sku.as_str().into());
        }
        if made {
            entry.insert("modes".into(), Value::Sequence(vec!["lan".into()]));
        }
        match keep.name() {
            Some(name) => entry.insert("name".into(), name.into()),
            None => entry.remove("name"),
        };
    }
    for id in &entries.remove {
        let key = key_of(devices, id);
        devices.remove(&key);
    }
    let text = serde_norway::to_string(&root).map_err(invalid)?;
    serde_norway::from_str::<Config>(&text).map_err(invalid)?;
    Ok(text)
}

fn write(path: &Path, entries: &Entries) -> Result<(), (StatusCode, String)> {
    let internal = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    let config = storage::read(path).map_err(internal)?.unwrap_or_default();
    let text = edited(&config, entries).map_err(|e| (StatusCode::CONFLICT, e))?;
    storage::write(path, text.as_bytes(), false).map_err(internal)
}

/// Write the entries, then close the SDK, so that the next request reads them.
pub(crate) async fn put(State(sdk): State<Arc<Sdk>>, Json(entries): Json<Entries>) -> Response {
    let Some(storage) = sdk.storage() else {
        return refuse(
            StatusCode::NOT_FOUND,
            "this server keeps no configuration file",
        );
    };
    let long = entries
        .keep
        .iter()
        .filter_map(Keep::name)
        .any(|name| name.chars().count() > NAME_MAX);
    if long {
        return refuse(
            StatusCode::BAD_REQUEST,
            "a name is longer than 64 characters",
        );
    }
    if let Err((status, error)) = write(&storage.config, &entries) {
        return refuse(status, &error);
    }
    sdk.reset().await;
    if let Err(error) = sdk.rooms().forget(&entries.remove) {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
    }
    Json(json!({})).into_response()
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn keep(id: &str, sku: Option<&str>, name: Option<&str>) -> Keep {
        Keep {
            id: id.to_owned(),
            sku: sku.map(str::to_owned),
            name: name.map(str::to_owned),
        }
    }

    fn apply(config: &str, keep_list: Vec<Keep>, remove: &[&str]) -> Value {
        let entries = Entries {
            keep: keep_list,
            remove: remove.iter().map(|id| (*id).to_owned()).collect(),
        };
        let text = edited(config, &entries).unwrap_or_default();
        serde_norway::from_str(&text).unwrap_or_default()
    }

    #[test]
    fn a_name_goes_under_the_existing_key_and_keeps_the_rest() {
        let config = "defaults:\n  modes: [lan]\ndevices:\n  aa:bb:\n    modes: [cloud]\n";
        let value = apply(
            config,
            vec![keep("AA:BB", Some("H6022"), Some("desk"))],
            &[],
        );
        assert_eq!(value["devices"]["aa:bb"]["name"], "desk");
        assert_eq!(value["devices"]["aa:bb"]["modes"][0], "cloud");
        assert_eq!(value["devices"]["aa:bb"]["sku"], "H6022");
        assert_eq!(value["defaults"]["modes"][0], "lan");
    }

    #[test]
    fn a_sku_in_the_file_stays() {
        let config = "devices:\n  AA:BB:\n    sku: H6008\n";
        let value = apply(config, vec![keep("AA:BB", Some("H6022"), None)], &[]);
        assert_eq!(value["devices"]["AA:BB"]["sku"], "H6008");
    }

    #[test]
    fn a_new_entry_gets_the_sku_and_lan() {
        let value = apply("", vec![keep("AA:BB", Some("H6022"), None)], &[]);
        assert_eq!(value["devices"]["AA:BB"]["sku"], "H6022");
        assert_eq!(value["devices"]["AA:BB"]["modes"][0], "lan");
        assert!(value["devices"]["AA:BB"].get("name").is_none());
    }

    #[test]
    fn a_blank_name_removes_the_name_and_keeps_the_entry() {
        let config = "devices:\n  AA:BB:\n    name: desk\n    modes: [lan]\n";
        let value = apply(config, vec![keep("AA:BB", None, Some("  "))], &[]);
        assert!(value["devices"]["AA:BB"].get("name").is_none());
        assert_eq!(value["devices"]["AA:BB"]["modes"][0], "lan");
    }

    #[test]
    fn remove_drops_the_entry_whatever_its_case() {
        let config = "devices:\n  aa:bb:\n    name: desk\n  CC:DD:\n    name: lamp\n";
        let value = apply(config, vec![], &["AA:BB"]);
        assert!(value["devices"].get("aa:bb").is_none());
        assert_eq!(value["devices"]["CC:DD"]["name"], "lamp");
    }

    #[test]
    fn a_file_that_does_not_parse_is_refused() {
        assert!(edited("devices: [", &Entries::default()).is_err());
    }
}
