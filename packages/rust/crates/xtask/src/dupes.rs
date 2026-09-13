//! `xtask dupes` — command entries two device files declare identically, and
//! no shared table carries.
//!
//! The layout of a command belongs in one place. Two files that declare one
//! layout are a family waiting to be written, and this task is what stops the
//! duplication from coming back with the next model.
//!
//! `notes:` is excluded from the comparison: what one unit does is a property
//! of that unit, and an `overrides:` entry is where it goes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process;

use serde_json::Value;

/// Report every duplicated layout, and fail when there is one.
pub(crate) fn dupes(devices: &[(PathBuf, Value)], families: &BTreeMap<String, Value>) {
    let shared = family_commands(families);
    let mut seen: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();

    for (path, device) in devices {
        let sku = device
            .get("sku")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{}: no `sku:`", path.display()));
        for (mode, table) in device
            .get("commands")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            for (command, spec) in table.as_object().into_iter().flatten() {
                if shared.contains(&(mode.clone(), command.clone())) {
                    continue;
                }
                let key = (mode.clone(), command.clone(), layout(spec));
                seen.entry(key).or_default().push(sku.to_owned());
            }
        }
    }

    let mut duplicated: Vec<((String, String), Vec<String>)> = seen
        .into_iter()
        .filter(|(_, skus)| skus.len() > 1)
        .map(|((mode, command, _), skus)| ((mode, command), skus))
        .collect();
    duplicated.sort();

    if duplicated.is_empty() {
        println!("no duplicated command layout");
        return;
    }
    eprintln!("these command layouts are declared more than once, and no family carries them:");
    for ((mode, command), skus) in &duplicated {
        eprintln!("  {mode}.{command}: {}", skus.join(", "));
    }
    eprintln!(
        "move each into devices/families/, name it in every `include:`, \
         and put what one unit does in `overrides:`"
    );
    process::exit(1);
}

/// Every `(mode, command)` a shared table already declares.
fn family_commands(families: &BTreeMap<String, Value>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for family in families.values() {
        for (mode, table) in family
            .get("commands")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            for command in table
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, _)| name)
            {
                out.push((mode.clone(), command.clone()));
            }
        }
    }
    out
}

/// The command without its `notes:`, as a comparable string.
fn layout(spec: &Value) -> String {
    let mut spec = spec.clone();
    if let Some(object) = spec.as_object_mut() {
        object.remove("notes");
    }
    spec.to_string()
}
