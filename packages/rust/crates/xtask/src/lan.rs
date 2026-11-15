//! The tables in `docs/lan-supported-devices.md`, and the `models:` list of
//! `devices/listed/govee-lan.yaml`.

use std::fs;
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

/// Rewrite the `models:` list of the listing file from `list`, or check it.
/// Returns `false` where `check` finds a difference.
pub(crate) fn listing(path: &Path, list: &Value, check: bool) -> bool {
    let text_now = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let start = text_now
        .find(MODELS)
        .unwrap_or_else(|| panic!("{} has no `models:` line", path.display()))
        + MODELS.len();
    let mut updated = text_now[..start].to_owned();
    for model in sorted(list) {
        updated.push_str(&format!(
            "  - {{ sku: {}, name: {} }}\n",
            quoted(text(model, "sku")),
            quoted(&model_name(text(model, "name"))),
        ));
    }
    if check {
        return updated == text_now;
    }
    fs::write(path, updated).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    println!("{} regenerated", path.display());
    true
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

/// A YAML double-quoted scalar.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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
