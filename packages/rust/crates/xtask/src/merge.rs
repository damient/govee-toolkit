//! Flattening a device file: the `include:`, then the `overrides:`, then the
//! bounds an argument takes from `capabilities:`.
//!
//! The generated catalog is flat, so a reader of `catalog.json` resolves
//! nothing. What happens here must match `codec::Catalog::parse_device`: a
//! catalog that disagreed with what the SDK loaded would send one set of
//! bytes and document another.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value};

/// Which capability parameter an argument role takes its bounds from.
const CAPABILITY_BOUNDS: [(&str, &str, &str); 2] = [
    ("brightness", "brightness", "range"),
    ("color_temp", "colortemp", "range_kelvin"),
];

/// Flatten one device file, in the order the crate applies the three steps.
pub(crate) fn flatten(path: &Path, device: &mut Value, families: &BTreeMap<String, Value>) {
    let local = local_commands(device);
    resolve_includes(path, device, families);
    apply_overrides(path, device, &local);
    resolve_bounds(path, device);
}

/// The command names the file declares itself, by mode.
fn local_commands(device: &Value) -> BTreeMap<String, Vec<String>> {
    let mut local = BTreeMap::new();
    let Some(commands) = device.get("commands").and_then(Value::as_object) else {
        return local;
    };
    for (mode, table) in commands {
        if let Some(table) = table.as_object() {
            local.insert(mode.clone(), table.keys().cloned().collect());
        }
    }
    local
}

/// Merge in every table a device file includes, and drop the `include:` key.
///
/// An unknown family and a command declared twice are errors here as they are
/// in the crate.
fn resolve_includes(path: &Path, device: &mut Value, families: &BTreeMap<String, Value>) {
    let Some(object) = device.as_object_mut() else {
        return;
    };
    let Some(include) = object.remove("include") else {
        return;
    };
    let names: Vec<String> = include
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default();

    for name in names {
        let family = families
            .get(&name)
            .unwrap_or_else(|| panic!("{}: no family `{name}`", path.display()));
        let Some(from) = family.get("commands").and_then(Value::as_object) else {
            continue;
        };
        let commands = object
            .entry("commands")
            .or_insert_with(|| Value::Object(Map::new()));
        let Some(commands) = commands.as_object_mut() else {
            continue;
        };
        for (mode, table) in from {
            let Some(table) = table.as_object() else {
                continue;
            };
            let into = commands
                .entry(mode.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            let Some(into) = into.as_object_mut() else {
                continue;
            };
            for (command, spec) in table {
                assert!(
                    !into.contains_key(command),
                    "{}: `{mode}.{command}` is declared here and in `{name}`",
                    path.display()
                );
                into.insert(command.clone(), spec.clone());
            }
        }
    }
}

/// Apply the `overrides:` block, and drop the key.
fn apply_overrides(path: &Path, device: &mut Value, local: &BTreeMap<String, Vec<String>>) {
    let Some(object) = device.as_object_mut() else {
        return;
    };
    let Some(overrides) = object.remove("overrides") else {
        return;
    };
    let Some(overrides) = overrides.as_object() else {
        return;
    };
    let file = path.display();
    for (mode, patches) in overrides {
        let Some(patches) = patches.as_object() else {
            continue;
        };
        let table = object
            .get_mut("commands")
            .and_then(|c| c.get_mut(mode))
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| panic!("{file}: `overrides.{mode}` names no command table"));
        for (command, patch) in patches {
            assert!(
                !local
                    .get(mode)
                    .is_some_and(|names| names.iter().any(|n| n == command)),
                "{file}: `overrides.{mode}.{command}`: this file declares the command itself"
            );
            let Some(spec) = table.get_mut(command).and_then(Value::as_object_mut) else {
                panic!(
                    "{file}: `overrides.{mode}.{command}`: no table this file includes declares it"
                )
            };
            patch_command(
                &format!("{file}: `overrides.{mode}.{command}`"),
                spec,
                patch,
            );
            if patch.get("drop").and_then(Value::as_bool) == Some(true) {
                table.remove(command);
            }
        }
    }
}

fn patch_command(where_: &str, spec: &mut Map<String, Value>, patch: &Value) {
    let Some(patch) = patch.as_object() else {
        return;
    };
    if patch.get("drop").and_then(Value::as_bool) == Some(true) {
        assert!(
            patch.len() == 1,
            "{where_}: `drop` removes the command, so it takes no other field"
        );
        return;
    }
    if let Some(notes) = patch.get("notes") {
        assert!(
            spec.get("notes") != Some(notes),
            "{where_}: `notes` repeats what the table already says"
        );
        spec.insert("notes".to_owned(), notes.clone());
    }
    let Some(args) = patch.get("args").and_then(Value::as_object) else {
        return;
    };
    for (arg, bound) in args {
        let Some(declared) = spec
            .get_mut("args")
            .and_then(|a| a.get_mut(arg))
            .and_then(Value::as_object_mut)
        else {
            panic!("{where_}: the command declares no argument `{arg}`")
        };
        for (field, value) in bound.as_object().into_iter().flatten() {
            assert!(
                declared.get(field) != Some(value),
                "{where_}: argument `{arg}`: `{field}` repeats what the table already gives"
            );
            declared.insert(field.clone(), value.clone());
        }
    }
}

/// Replace every `range: capability` with the pair `capabilities:` declares.
fn resolve_bounds(path: &Path, device: &mut Value) {
    let capabilities = device.get("capabilities").cloned().unwrap_or(Value::Null);
    let file = path.display();
    let Some(commands) = device.get_mut("commands").and_then(Value::as_object_mut) else {
        return;
    };
    for (mode, table) in commands {
        for (command, spec) in table.as_object_mut().into_iter().flatten() {
            for (arg, declared) in spec
                .get_mut("args")
                .and_then(Value::as_object_mut)
                .into_iter()
                .flatten()
            {
                let Some(declared) = declared.as_object_mut() else {
                    continue;
                };
                if declared.get("range") != Some(&Value::String("capability".to_owned())) {
                    continue;
                }
                let role = declared.get("role").and_then(Value::as_str);
                let pair = role.and_then(|role| bounds_for(&capabilities, role));
                let Some(pair) = pair else {
                    panic!(
                        "{file}: `{mode}.{command}` argument `{arg}`: \
                         `range: capability` needs a capability this device declares"
                    )
                };
                declared.insert("range".to_owned(), pair);
            }
        }
    }
}

fn bounds_for(capabilities: &Value, role: &str) -> Option<Value> {
    let (_, capability, parameter) = CAPABILITY_BOUNDS.iter().find(|(r, _, _)| *r == role)?;
    capabilities.get(capability)?.get(parameter).cloned()
}
