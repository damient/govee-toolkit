//! The tables in `docs/lan-supported-devices.md`, and the `models:` list of
//! `devices/listed/govee-lan.yaml`.

use std::collections::BTreeSet;
use std::path::Path;

use govee_toolkit::codec::Catalog;
use serde_json::Value;

/// The line after which `devices/listed/govee-lan.yaml` is generated.
const MODELS: &str = "\nmodels:\n";

pub(crate) fn source_block(list: &Value) -> String {
    format!(
        "**Source:** <{}>\n**Retrieved:** {}\n**Entries:** {}\n",
        text(list, "source"),
        text(list, "retrieved"),
        models(list).len(),
    )
}

/// One row per model. `Definition` names the device file that claims the
/// SKU, or says that the SKU resolves to the listing.
pub(crate) fn model_table(list: &Value, catalog: &Catalog) -> String {
    let mut out = String::from(
        "| SKU | Model | Category | Definition |\n| --- | ----- | -------- | ---------- |\n",
    );
    for model in sorted(list) {
        let sku = text(model, "sku");
        let definition = match catalog.device(sku) {
            Ok(device) if device.listing.is_some() => "listed, not verified".to_owned(),
            Ok(device) => format!("[`{0}`](../devices/{0}.yaml)", device.sku),
            Err(_) => "none".to_owned(),
        };
        out.push_str(&format!(
            "| `{sku}` | {} | {} | {definition} |\n",
            text(model, "name"),
            text(model, "category"),
        ));
    }
    out
}

/// `file` with its `models:` list rebuilt from `list`, less the SKUs in
/// `elsewhere`.
pub(crate) fn listing(
    path: &Path,
    file: &str,
    list: &Value,
    elsewhere: &BTreeSet<String>,
) -> String {
    let start = file
        .find(MODELS)
        .unwrap_or_else(|| panic!("{} has no `models:` line", path.display()))
        + MODELS.len();
    let mut updated = file[..start].to_owned();
    for model in sorted(list)
        .into_iter()
        .filter(|model| !elsewhere.contains(&text(model, "sku").to_uppercase()))
    {
        updated.push_str(&format!(
            "  - {{ sku: {}, name: {} }}\n",
            quoted(text(model, "sku")),
            quoted(&model_name(text(model, "name"))),
        ));
    }
    updated
}

/// Every model is a Govee device, so the list drops the word from the name.
fn model_name(name: &str) -> String {
    name.replace("Govee ", "")
}

fn sorted(list: &Value) -> Vec<&Value> {
    let mut rows: Vec<&Value> = models(list).iter().collect();
    rows.sort_by_key(|model| text(model, "sku"));
    rows
}

/// A YAML double-quoted scalar: a JSON string is one.
fn quoted(value: &str) -> String {
    Value::from(value).to_string()
}

fn models(list: &Value) -> &[Value] {
    list.get("models")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("docs/lan-supported-devices.json: an entry has no `{key}`"))
}
