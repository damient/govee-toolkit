//! Change the `devices:` entries of a configuration file. The other keys stay,
//! and the comments of the file are lost.

use std::path::Path;

use serde_norway::{Mapping, Value};

use crate::codec::Mode;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::transport::DeviceId;

/// One change to the `devices:` entries. A key matches in any case.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceEdit {
    /// Make the entry of `id` with `modes`. An entry keeps its modes.
    Add {
        /// The device.
        id: DeviceId,
        /// The modes of the new entry.
        modes: Vec<Mode>,
    },
    /// Write `sku` in the entry of `id` when the entry gives none.
    Sku {
        /// The device.
        id: DeviceId,
        /// The SKU to read the device file under.
        sku: String,
    },
    /// Set the trimmed name of `id`. `None` or a blank name removes it.
    Name {
        /// The device.
        id: DeviceId,
        /// The new name.
        name: Option<String>,
    },
    /// Remove the entry of `id`. A missing entry is not an error.
    Remove(DeviceId),
    /// Remove the `devices:` key.
    RemoveAll,
}

/// The text of the file after `edits`, in order, or `None` when the edits
/// change nothing. `path` names the file in the error.
///
/// # Errors
///
/// [`Error::Config`] when `text` does not parse, when its root or a device
/// entry is not a mapping, or when the result is not a valid configuration.
pub fn edit_devices(path: &Path, text: &str, edits: &[DeviceEdit]) -> Result<Option<String>> {
    let invalid = |reason: String| Error::Config {
        path: path.display().to_string(),
        reason,
    };
    let mut root = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_norway::from_str(text).map_err(|e| invalid(e.to_string()))?
    };
    if root.is_null() {
        root = Value::Mapping(Mapping::new());
    }
    let Value::Mapping(map) = &mut root else {
        return Err(invalid("the root is not a mapping".to_owned()));
    };
    let before = map.clone();
    for edit in edits {
        apply(map, edit).map_err(|reason| invalid(reason.to_owned()))?;
    }
    if *map == before {
        return Ok(None);
    }
    let text = serde_norway::to_string(&root).map_err(|e| invalid(e.to_string()))?;
    serde_norway::from_str::<Config>(&text).map_err(|e| invalid(e.to_string()))?;
    Ok(Some(text))
}

const NOT_A_MAPPING: &str = "a device entry is not a mapping";

fn apply(map: &mut Mapping, edit: &DeviceEdit) -> std::result::Result<(), &'static str> {
    if *edit == DeviceEdit::RemoveAll {
        map.remove("devices");
        return Ok(());
    }
    let devices = child(map, "devices".into())?;
    match edit {
        DeviceEdit::Add { id, modes } => {
            let key = key_of(devices, id);
            if !devices.contains_key(&key) {
                let modes = modes.iter().map(|mode| mode.to_string().into()).collect();
                let mut entry = Mapping::new();
                entry.insert("modes".into(), Value::Sequence(modes));
                devices.insert(key, entry.into());
            }
        }
        DeviceEdit::Sku { id, sku } => {
            let entry = child(devices, key_of(devices, id))?;
            if !entry.contains_key("sku") {
                entry.insert("sku".into(), sku.as_str().into());
            }
        }
        DeviceEdit::Name { id, name } => {
            let entry = child(devices, key_of(devices, id))?;
            match name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                Some(name) => entry.insert("name".into(), name.into()),
                None => entry.remove("name"),
            };
        }
        DeviceEdit::Remove(id) => {
            devices.remove(key_of(devices, id));
        }
        DeviceEdit::RemoveAll => {}
    }
    Ok(())
}

fn child(map: &mut Mapping, key: Value) -> std::result::Result<&mut Mapping, &'static str> {
    match map.entry(key).or_insert_with(|| Mapping::new().into()) {
        Value::Mapping(child) => Ok(child),
        _ => Err(NOT_A_MAPPING),
    }
}

/// The key of a new entry is the normalized form of `id`.
fn key_of(devices: &Mapping, id: &DeviceId) -> Value {
    devices
        .keys()
        .find(|key| key.as_str().is_some_and(|key| DeviceId::new(key) == *id))
        .cloned()
        .unwrap_or_else(|| id.as_str().into())
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn id(raw: &str) -> DeviceId {
        DeviceId::new(raw)
    }

    fn edited(text: &str, edits: &[DeviceEdit]) -> Value {
        let text = edit_devices(Path::new("config.yaml"), text, edits)
            .unwrap()
            .unwrap_or_else(|| text.to_owned());
        serde_norway::from_str(&text).unwrap()
    }

    fn keep(raw: &str, sku: &str, name: Option<&str>) -> Vec<DeviceEdit> {
        vec![
            DeviceEdit::Add {
                id: id(raw),
                modes: vec![Mode::Lan],
            },
            DeviceEdit::Sku {
                id: id(raw),
                sku: sku.to_owned(),
            },
            DeviceEdit::Name {
                id: id(raw),
                name: name.map(str::to_owned),
            },
        ]
    }

    #[test]
    fn a_name_goes_under_the_existing_key_and_keeps_the_rest() {
        let text = "defaults:\n  modes: [lan]\ndevices:\n  aa:bb:\n    modes: [cloud]\n";
        let value = edited(text, &keep("AA:BB", "H6022", Some(" desk ")));
        assert_eq!(value["devices"]["aa:bb"]["name"], "desk");
        assert_eq!(value["devices"]["aa:bb"]["modes"][0], "cloud");
        assert_eq!(value["devices"]["aa:bb"]["sku"], "H6022");
        assert_eq!(value["defaults"]["modes"][0], "lan");
    }

    #[test]
    fn a_sku_in_the_file_stays() {
        let value = edited(
            "devices:\n  AA:BB:\n    sku: H6008\n",
            &keep("AA:BB", "H6022", None),
        );
        assert_eq!(value["devices"]["AA:BB"]["sku"], "H6008");
    }

    #[test]
    fn a_new_entry_gets_the_sku_and_the_modes() {
        let value = edited("", &keep("aa:bb", "H6022", None));
        assert_eq!(value["devices"]["AA:BB"]["sku"], "H6022");
        assert_eq!(value["devices"]["AA:BB"]["modes"][0], "lan");
        assert!(value["devices"]["AA:BB"].get("name").is_none());
    }

    #[test]
    fn a_blank_name_removes_the_name_and_keeps_the_entry() {
        let text = "devices:\n  AA:BB:\n    name: desk\n    modes: [lan]\n";
        let value = edited(text, &keep("AA:BB", "H6022", Some("  ")));
        assert!(value["devices"]["AA:BB"].get("name").is_none());
        assert_eq!(value["devices"]["AA:BB"]["modes"][0], "lan");
    }

    #[test]
    fn remove_drops_the_entry_whatever_its_case() {
        let text = "devices:\n  aa:bb:\n    name: desk\n  CC:DD:\n    name: lamp\n";
        let value = edited(text, &[DeviceEdit::Remove(id("AA:BB"))]);
        assert!(value["devices"].get("aa:bb").is_none());
        assert_eq!(value["devices"]["CC:DD"]["name"], "lamp");
    }

    #[test]
    fn remove_all_drops_the_key_and_keeps_the_others() {
        let text = "lan:\n  status_timeout_ms: 500\ndevices:\n  AA:BB:\n    name: Desk\n";
        let value = edited(text, &[DeviceEdit::RemoveAll]);
        assert!(value.get("devices").is_none());
        assert_eq!(value["lan"]["status_timeout_ms"], 500);
    }

    #[test]
    fn an_edit_that_changes_nothing_gives_none() {
        let path = Path::new("config.yaml");
        let edits = [DeviceEdit::RemoveAll];
        assert_eq!(edit_devices(path, "# a comment\n", &edits).unwrap(), None);
        assert_eq!(edit_devices(path, "lan: {}\n", &edits).unwrap(), None);
        let named = "devices:\n  AA:BB:\n    sku: H6022\n    name: desk\n";
        assert_eq!(
            edit_devices(path, named, &keep("AA:BB", "H6022", Some("desk"))).unwrap(),
            None
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_refused() {
        assert!(edit_devices(Path::new("c"), "devices: [", &[DeviceEdit::RemoveAll]).is_err());
        assert!(edit_devices(Path::new("c"), "- a\n", &[DeviceEdit::RemoveAll]).is_err());
    }
}
