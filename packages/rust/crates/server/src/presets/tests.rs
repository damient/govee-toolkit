#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use serde_json::{Value, json};

use super::*;

fn list(values: Value) -> Result<Vec<Preset>, String> {
    valid(serde_json::from_value(values).map_err(|e| e.to_string())?)
}

#[test]
fn a_preset_holds_a_color_or_cells() {
    assert!(list(json!([{ "name": "Preset #1", "sku": "H61A0", "color": "#ff0000" }])).is_ok());
    assert!(
        list(json!([{ "name": "A", "sku": "H61A0", "cells": ["#ff0000", "#000000"] }])).is_ok()
    );
    assert!(list(json!([{ "name": "A", "sku": "H61A0" }])).is_err());
    let both = json!([{ "name": "A", "sku": "H61A0", "color": "#ff0000", "cells": ["#ff0000"] }]);
    assert!(list(both).is_err());
    assert!(list(json!([{ "name": "A", "sku": "H61A0", "cells": [] }])).is_err());
    assert!(list(json!([{ "name": "A", "sku": "H61A0", "color": "red" }])).is_err());
    assert!(list(json!([{ "name": "A", "sku": "H61A0", "color": "#ff0000", "x": 1 }])).is_err());
}

#[test]
fn an_edit_changes_one_preset_of_the_server() {
    let store = Store::default();
    let one = |name: &str| -> Preset {
        serde_json::from_value(json!({ "name": name, "sku": "H61A0", "color": "#ff0000" })).unwrap()
    };
    let add = |name: &str| {
        let preset = one(name);
        store.edit(move |list| {
            list.push(preset);
            Ok(())
        })
    };
    add("A").unwrap();
    add("B").unwrap();
    add("C").unwrap();
    assert_eq!(add(" ").unwrap_err().0, StatusCode::BAD_REQUEST);
    let presets = store
        .edit(|list| {
            let at = place(list, "preset-2")?;
            list.remove(at);
            Ok(())
        })
        .unwrap();
    let names: Vec<&str> = presets.presets.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["A", "C"]);
    assert_eq!(add("D").unwrap().presets[2].id, "preset-4");
    let missing = store.edit(|list| place(list, "preset-2").map(|_| ()));
    assert_eq!(missing.unwrap_err().0, StatusCode::NOT_FOUND);
}

#[test]
fn a_white_is_a_temperature() {
    assert!(list(json!([{ "name": "A", "sku": "H6022", "kelvin": 2700 }])).is_ok());
    assert!(list(json!([{ "name": "A", "sku": "H6022", "kelvin": 20 }])).is_err());
    let both = json!([{ "name": "A", "sku": "H6022", "kelvin": 2700, "color": "#ff0000" }]);
    assert!(list(both).is_err());
}

#[test]
fn the_brightness_is_a_percent() {
    let level = |level: u8| {
        list(json!([{ "name": "A", "sku": "H61A0", "color": "#ff0000", "brightness": level }]))
    };
    assert!(level(100).is_ok());
    assert!(level(101).is_err());
    let saved = list(json!([{ "name": "A", "sku": "H61A0", "color": "#ff0000" }])).unwrap();
    assert_eq!(
        serde_json::to_value(&saved).unwrap(),
        json!([{ "id": "preset-1", "name": "A", "sku": "H61A0", "color": "#ff0000" }])
    );
}

#[test]
fn a_new_preset_gets_a_number_above_the_others() {
    let presets = list(json!([
        { "id": "preset-4", "name": "A", "sku": "H61A0", "color": "#ff0000" },
        { "name": " B ", "sku": "H6022", "color": "#00ff00" },
    ]))
    .unwrap();
    assert_eq!(presets[1].id, "preset-5");
    assert_eq!(presets[1].name, "B");
    let twice = json!([
        { "id": "p", "name": "A", "sku": "H61A0", "color": "#ff0000" },
        { "id": "p", "name": "B", "sku": "H61A0", "color": "#ff0000" },
    ]);
    assert!(list(twice).is_err());
}

#[test]
fn a_name_and_a_model_are_short_and_plain() {
    let named = |name: &str| list(json!([{ "name": name, "sku": "H61A0", "color": "#ff0000" }]));
    assert!(named("  ").is_err());
    assert!(named("a\nb").is_err());
    assert!(named(&"a".repeat(65)).is_err());
    assert!(list(json!([{ "name": "A", "sku": "../x", "color": "#ff0000" }])).is_err());
}

#[test]
fn the_list_is_bounded_and_versioned() {
    let one = json!({ "name": "A", "sku": "H61A0", "color": "#ff0000" });
    assert!(list(json!(vec![one.clone(); PRESETS])).is_ok());
    assert!(list(json!(vec![one; PRESETS + 1])).is_err());
    let other = Presets {
        version: "9.0.0".to_owned(),
        presets: Vec::new(),
    };
    assert!(other.checked().is_err());
}
