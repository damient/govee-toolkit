//! `dist/catalog.json`: the device files and the listings.

use std::path::{Path, PathBuf};

use govee_toolkit::codec::{Catalog, Device, SCHEMA_VERSION};
use govee_toolkit::profile;

use crate::{build_catalog, write_json};

pub(crate) fn catalog(root: &Path, out: Option<PathBuf>) {
    let devices = root.join("devices");
    let out = out.unwrap_or_else(|| root.join("dist/catalog.json"));

    let catalog = build_catalog(&devices);

    let entries: Vec<serde_json::Value> = catalog.devices().map(device_json).collect();
    let document = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "generator": "packages/rust/crates/xtask",
        "devices": entries,
        "listings": listings_json(&catalog),
    });

    write_json(&out, &document);
    println!("{} devices -> {}", entries.len(), out.display());
}

fn device_json(device: &Device) -> serde_json::Value {
    let mut value = serde_json::to_value(device).expect("serialize the device");
    if let Some(object) = value.as_object_mut() {
        object.insert("dmx".to_owned(), profile::report::entry(device));
    }
    value
}

/// A model that a device file claims is left out.
fn listings_json(catalog: &Catalog) -> Vec<serde_json::Value> {
    catalog
        .listings()
        .iter()
        .map(|listing| {
            let mut definition = device_json(&listing.device);
            if let Some(object) = definition.as_object_mut() {
                object.remove("sku");
                object.remove("name");
            }
            let models: Vec<serde_json::Value> = listing
                .models
                .iter()
                .filter(|model| {
                    catalog
                        .device(&model.sku)
                        .is_ok_and(|device| device.listing.as_ref() == Some(&listing.listing))
                })
                .map(|model| serde_json::json!({ "sku": model.sku, "name": model.name }))
                .collect();
            serde_json::json!({
                "listing": listing.listing,
                "source": listing.source,
                "device": definition,
                "models": models,
            })
        })
        .collect()
}
