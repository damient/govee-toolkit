//! The tables in `docs/lan-supported-devices.md`, from
//! `docs/lan-supported-devices.json`. The site reads the same file.

use serde_json::Value;

/// The source, the retrieval date and the count, as the page header states
/// them.
pub(crate) fn source_block(list: &Value) -> String {
    format!(
        "**Source:** <{}>\n**Retrieved:** {}\n**Entries:** {}\n",
        text(list, "source"),
        text(list, "retrieved"),
        models(list).len(),
    )
}

/// One row per model, sorted by SKU.
pub(crate) fn model_table(list: &Value) -> String {
    let mut rows: Vec<&Value> = models(list).iter().collect();
    rows.sort_by_key(|model| text(model, "sku"));
    let mut out = String::from("| SKU | Model | Category |\n| --- | ----- | -------- |\n");
    for model in rows {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            text(model, "sku"),
            text(model, "name"),
            text(model, "category"),
        ));
    }
    out
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
