//! `dist/api.json`: every role, joined to the method that serves it on each
//! surface, and the public surface of each language.
//!
//! Each surface is read from what generates or checks it, so the file cannot
//! drift from the code. The one table written by hand is [`JOIN`], and
//! `--check` fails when it names a method that a surface does not carry.

use std::collections::BTreeMap;
use std::path::Path;
use std::process;

use govee_toolkit::codec::Role;
use serde_json::{Value, json};

mod rust;
mod text;

/// `(role, Rust method, CLI verbs)`: the Rust method that serves a role, and
/// the CLI verbs that reach it.
///
/// The Node name is the Rust name in camel case, and the Python name is the
/// Rust name. A role must appear here or in [`OMITTED`] for each surface.
#[rustfmt::skip]
const JOIN: &[(&str, &str, &[&str])] = &[
    ("status", "DeviceHandle::status", &["status"]),
    ("power", "DeviceHandle::power", &["on", "off"]),
    ("brightness", "DeviceHandle::brightness", &["brightness"]),
    ("color", "DeviceHandle::color", &["color"]),
    ("color_temp", "DeviceHandle::color_temp", &["colortemp"]),
    ("segment_enable", "DeviceHandle::segment", &["segment"]),
    ("segment_color", "DeviceHandle::segment", &["segment"]),
    ("segment_color_masked", "DeviceHandle::segment", &["segment"]),
    ("segment_gradient", "DeviceHandle::gradient", &["gradient"]),
    ("wifi_link", "DeviceHandle::provision_wifi", &["provision"]),
    ("wifi_api_type", "DeviceHandle::provision_wifi", &["provision"]),
    ("wifi_provision", "DeviceHandle::provision_wifi", &["provision"]),
    ("wifi_provision_with_api", "DeviceHandle::provision_wifi", &["provision"]),
    ("music", "DeviceHandle::music", &["music"]),
];

/// `(surface, role, reason)`: a role that a surface leaves out on purpose.
/// Its entry in `roles` is an empty list for that surface.
const OMITTED: &[(&str, &str, &str)] = &[];

const SURFACES: [&str; 4] = ["cli", "rust", "node", "python"];

/// Write `dist/api.json`, or with `check`, only verify the join.
///
/// Both exit 1 when a role has no method on a surface.
pub(crate) fn api(root: &Path, check: bool) {
    let rust_root = root.join("packages/rust");
    let roles: Vec<String> = Role::CLAIMABLE.iter().map(ToString::to_string).collect();
    let methods: BTreeMap<&str, Vec<String>> = BTreeMap::from([
        ("cli", rust::cli(&rust_root.join("crates/cli/src/cli"))),
        ("rust", rust::methods(&rust_root.join("src"))),
        (
            "node",
            text::node(&crate::read(&root.join("packages/node/binding.d.cts"))),
        ),
        (
            "python",
            text::python(&crate::read(
                &root.join("packages/python/govee_toolkit/_govee_toolkit.pyi"),
            )),
        ),
    ]);

    let (joined, errors) = join(&roles, &methods);
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("api: {error}");
        }
        eprintln!("Add the method, or name the role in `OMITTED` in crates/xtask/src/api.");
        process::exit(1);
    }
    if check {
        println!("every role has a method on every surface");
        return;
    }

    let document = json!({
        "schema_version": 1,
        "generator": "packages/rust/crates/xtask",
        "roles": joined,
        "methods": methods,
    });
    let out = root.join("dist/api.json");
    crate::write_json(&out, &document);
    println!("{} roles -> {}", roles.len(), out.display());
}

fn join(roles: &[String], methods: &BTreeMap<&str, Vec<String>>) -> (Value, Vec<String>) {
    let mut errors = Vec::new();
    for (joined, ..) in JOIN {
        if !roles.iter().any(|role| role == joined) {
            errors.push(format!(
                "`{joined}` is joined, and the `Role` enum has no such role"
            ));
        }
    }
    for (surface, role, _) in OMITTED {
        if !roles.iter().any(|known| known == role) {
            errors.push(format!(
                "`{role}` is omitted, and the `Role` enum has no such role"
            ));
        }
        if !SURFACES.contains(surface) {
            errors.push(format!(
                "`{role}` is omitted on `{surface}`, which is no surface"
            ));
        }
    }

    let mut joined = serde_json::Map::new();
    for role in roles {
        let Some(&(_, rust, cli)) = JOIN.iter().find(|(joined, ..)| joined == role) else {
            errors.push(format!("`{role}` is in no row of `JOIN`"));
            continue;
        };
        let mut row = serde_json::Map::new();
        for surface in SURFACES {
            let omitted = OMITTED.iter().any(|(s, r, _)| *s == surface && r == role);
            let found = if omitted {
                Vec::new()
            } else {
                serving(rust, cli, surface, &methods[surface])
            };
            if found.is_empty() && !omitted {
                errors.push(format!("`{role}` has no method on `{surface}`"));
            }
            row.insert(surface.to_owned(), json!(found));
        }
        joined.insert(role.clone(), Value::Object(row));
    }
    (Value::Object(joined), errors)
}

/// The entries of `methods` that serve one row of [`JOIN`] on `surface`.
fn serving(rust: &str, cli: &[&str], surface: &str, methods: &[String]) -> Vec<String> {
    let (class, name) = rust.split_once("::").expect("`Type::method` in `JOIN`");
    let names: Vec<String> = match surface {
        "cli" => cli.iter().map(|verb| format!("govee {verb}")).collect(),
        "rust" => vec![rust.to_owned()],
        "node" => vec![format!("{class}.{}", text::camel(name))],
        _ => vec![format!("{class}.{name}")],
    };
    methods
        .iter()
        .filter(|method| names.iter().any(|name| head(method) == name))
        .cloned()
        .collect()
}

/// A method up to its arguments: `Type.name`, or `govee verb`.
fn head(method: &str) -> &str {
    let end = match method.strip_prefix("govee ") {
        Some(rest) => rest.find(' ').map_or(method.len(), |i| i + "govee ".len()),
        None => method.find('(').unwrap_or(method.len()),
    };
    &method[..end]
}
