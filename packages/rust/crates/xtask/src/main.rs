//! Repository tasks, all of them generating something from `devices/*.yaml`.
//!
//! - `xtask catalog [path]` — the distributable catalog.
//! - `xtask compat [--check]` — the tables in `docs/compatibility.md`.
//! - `xtask dupes` — command layouts two device files declare, and no shared
//!   table carries.
//!
//! The catalog is a build output for anyone who wants the device files without
//! a YAML parser: never committed, produced by CI, attached to a release. See
//! `docs/architecture.md`.
//!
//! A repository task fails the build loudly; the library's no-panic rule does
//! not apply here.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::format_push_string
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

use govee_toolkit::codec::{Catalog, SCHEMA_VERSION};

mod dupes;

fn main() {
    let root = repository_root();
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("compat") => compat(&root, args.iter().any(|a| a == "--check")),
        Some("dupes") => {
            let devices = root.join("devices");
            dupes::dupes(&load(&devices), &load_families(&devices));
        }
        Some("catalog") | None => catalog(&root, args.get(1).map(PathBuf::from)),
        Some(other) => {
            eprintln!("unknown task `{other}`; expected `catalog`, `compat` or `dupes`");
            process::exit(2);
        }
    }
}

/// `devices/*.yaml` into one JSON document.
///
/// The crate loads the files, so the catalog says what the SDK reads. It is
/// flat: an `include:` and an `overrides:` block are applied before this, and
/// neither reaches the file.
fn catalog(root: &Path, out: Option<PathBuf>) {
    let devices = root.join("devices");
    let out = out.unwrap_or_else(|| root.join("dist/catalog.json"));

    let sources = read_all(&yaml_files(&devices));
    let families = read_all(&yaml_files(&devices.join("families")));
    let catalog = Catalog::from_sources_with(borrow(&sources), borrow(&families))
        .unwrap_or_else(|e| panic!("{e}"));

    let entries: Vec<_> = catalog.devices().collect();
    let document = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "generator": "packages/rust/crates/xtask",
        "devices": entries,
    });

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{}: {e}", parent.display()));
    }
    let mut text = serde_json::to_string_pretty(&document).expect("serialize the catalog");
    text.push('\n');
    fs::write(&out, text).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
    println!("{} devices -> {}", entries.len(), out.display());
}

/// The two tables in `docs/compatibility.md`, between their generated markers.
///
/// The prose around them is written by hand; only what the device files already
/// state is generated, and it is generated as they state it. A blank
/// `verified.date` renders `?`, because that is what the file says.
fn compat(root: &Path, check: bool) {
    let page = root.join("docs/compatibility.md");
    let text =
        fs::read_to_string(&page).unwrap_or_else(|e| panic!("cannot read {}: {e}", page.display()));

    let devices: Vec<serde_json::Value> = load(&root.join("devices"))
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    let updated = replace_block(&text, "support-by-sku", &support_table(&devices));
    let updated = replace_block(&updated, "capabilities-by-sku", &capability_table(&devices));

    if check {
        if updated != text {
            eprintln!(
                "{} is out of date with devices/*.yaml. Run `cargo run -p xtask -- compat`.",
                page.display()
            );
            process::exit(1);
        }
        println!("{} is up to date", page.display());
        return;
    }
    fs::write(&page, updated).unwrap_or_else(|e| panic!("{}: {e}", page.display()));
    println!(
        "{} regenerated from {} devices",
        page.display(),
        devices.len()
    );
}

/// The shared command tables, by the `family:` each declares.
fn load_families(devices: &Path) -> BTreeMap<String, serde_json::Value> {
    let mut families = BTreeMap::new();
    for path in yaml_files(&devices.join("families")) {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let value: serde_json::Value =
            serde_norway::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = value
            .get("family")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("{}: no `family:`", path.display()))
            .to_owned();
        families.insert(name, value);
    }
    families
}

/// Every device file, parsed, sorted by path — which sorts by SKU.
fn load(dir: &Path) -> Vec<(PathBuf, serde_json::Value)> {
    yaml_files(dir)
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let value =
                serde_norway::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (path, value)
        })
        .collect()
}

/// Every `*.yaml` in `dir`, sorted by path — which sorts by SKU.
fn yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
        // schema.yaml is the reference template, not a device.
        .filter(|p| p.file_name().is_some_and(|n| n != "schema.yaml"))
        .collect();
    paths.sort();
    paths
}

/// Each file as `(name, text)`, which is what the crate loads a catalog from.
fn read_all(paths: &[PathBuf]) -> Vec<(String, String)> {
    paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            (path.display().to_string(), text)
        })
        .collect()
}

fn borrow(sources: &[(String, String)]) -> impl Iterator<Item = (&str, &str)> {
    sources
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
}

fn replace_block(text: &str, name: &str, body: &str) -> String {
    let open = format!("<!-- generated: {name} -->");
    let close = "<!-- /generated -->";
    let start = text
        .find(&open)
        .unwrap_or_else(|| panic!("docs/compatibility.md has no `{open}` marker"));
    let after = start + open.len();
    let end = text[after..]
        .find(close)
        .unwrap_or_else(|| panic!("`{open}` is never closed by `{close}`"))
        + after;
    format!("{}{open}\n{body}{}", &text[..start], &text[end..])
}

fn field<'a>(device: &'a serde_json::Value, path: &[&str]) -> &'a str {
    let mut node = device;
    for key in path {
        match node.get(key) {
            Some(next) => node = next,
            None => return "",
        }
    }
    node.as_str().unwrap_or_default()
}

fn support_table(devices: &[serde_json::Value]) -> String {
    let mut out = String::from(
        "| SKU | Family | Name | `lan` | `ble` | `cloud` | Verified |\n\
         | --- | ------ | ---- | ----- | ----- | ------- | -------- |\n",
    );
    for device in devices {
        let sku = field(device, &["sku"]);
        let support = |mode: &str| match field(device, &["modes", mode, "support"]) {
            "" | "unknown" => "?".to_owned(),
            other => other.to_owned(),
        };
        let date = field(device, &["verified", "date"]);
        let verified = if date.is_empty() {
            "?".to_owned()
        } else {
            format!("✅ {date}")
        };
        out.push_str(&format!(
            "| [{sku}](../devices/{sku}.yaml) | {} | {} | {} | {} | {} | {verified} |\n",
            field(device, &["family"]),
            field(device, &["name"]),
            support("lan"),
            support("ble"),
            support("cloud"),
        ));
    }
    out
}

/// The capability columns, taken from the device files themselves: every name
/// any of them declares, in name order. A device file may declare a capability
/// no SDK has heard of, and it still gets a column.
fn capability_columns(devices: &[serde_json::Value]) -> Vec<String> {
    let mut names: Vec<String> = devices
        .iter()
        .filter_map(|device| device.get("capabilities"))
        .filter_map(serde_json::Value::as_object)
        .flat_map(serde_json::Map::keys)
        .cloned()
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn capability_table(devices: &[serde_json::Value]) -> String {
    let columns = capability_columns(devices);
    let mut out = format!("| SKU | {} |\n", columns.join(" | "));
    out.push_str(&format!(
        "| --- | {} |\n",
        columns
            .iter()
            .map(|c| "-".repeat(c.len()))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for device in devices {
        let cells: Vec<&str> = columns
            .iter()
            .map(|c| {
                if device
                    .get("capabilities")
                    .and_then(|caps| caps.get(c))
                    .is_some()
                {
                    "✅"
                } else {
                    "—"
                }
            })
            .collect();
        out.push_str(&format!(
            "| {} | {} |\n",
            field(device, &["sku"]),
            cells.join(" | ")
        ));
    }
    out
}

/// `packages/rust/crates/xtask` -> the repository root.
fn repository_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(4)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}
