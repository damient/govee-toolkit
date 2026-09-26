//! `dist/api.json`: every role, joined to the method that serves it on each
//! surface, and the public surface of each language.
//!
//! Each surface is read from what generates or checks it, so the file cannot
//! drift from the code. The one table written by hand is [`JOIN`], and
//! `--check` fails when it names a method that a surface does not carry.

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, process};

use serde_json::{Value, json};

mod rust;
mod text;

/// The Rust method that serves a role, and the CLI verbs that reach it.
///
/// The Node name is the Rust name in camel case, and the Python name is the
/// Rust name. A role must appear here or in [`OMITTED`] for each surface.
struct Join {
    role: &'static str,
    rust: &'static str,
    cli: &'static [&'static str],
}

const JOIN: &[Join] = &[
    Join {
        role: "status",
        rust: "DeviceHandle::status",
        cli: &["status"],
    },
    Join {
        role: "power",
        rust: "DeviceHandle::power",
        cli: &["on", "off"],
    },
    Join {
        role: "brightness",
        rust: "DeviceHandle::brightness",
        cli: &["brightness"],
    },
    Join {
        role: "color",
        rust: "DeviceHandle::color",
        cli: &["color"],
    },
    Join {
        role: "color_temp",
        rust: "DeviceHandle::color_temp",
        cli: &["colortemp"],
    },
    // The channel is armed by the paint that needs it, never on its own.
    Join {
        role: "segment_enable",
        rust: "DeviceHandle::segment",
        cli: &["segment"],
    },
    Join {
        role: "segment_color",
        rust: "DeviceHandle::segment",
        cli: &["segment"],
    },
    Join {
        role: "segment_color_masked",
        rust: "DeviceHandle::segment",
        cli: &["segment"],
    },
    Join {
        role: "segment_gradient",
        rust: "DeviceHandle::gradient",
        cli: &["gradient"],
    },
    // The four provisioning roles are the steps of one exchange.
    Join {
        role: "wifi_link",
        rust: "DeviceHandle::provision_wifi",
        cli: &["provision"],
    },
    Join {
        role: "wifi_api_type",
        rust: "DeviceHandle::provision_wifi",
        cli: &["provision"],
    },
    Join {
        role: "wifi_provision",
        rust: "DeviceHandle::provision_wifi",
        cli: &["provision"],
    },
    Join {
        role: "wifi_provision_with_api",
        rust: "DeviceHandle::provision_wifi",
        cli: &["provision"],
    },
    Join {
        role: "music",
        rust: "DeviceHandle::music",
        cli: &["music"],
    },
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
    let roles = rust::roles(&rust_root.join("src/codec/catalog/spec.rs"));
    let methods: BTreeMap<&str, Vec<String>> = BTreeMap::from([
        ("cli", rust::cli(&rust_root.join("crates/cli/src/cli"))),
        ("rust", rust::methods(&rust_root.join("src"))),
        (
            "node",
            text::node(&read(&root.join("packages/node/binding.d.cts"))),
        ),
        (
            "python",
            text::python(&read(
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
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{}: {e}", parent.display()));
    }
    let mut body = serde_json::to_string_pretty(&document).expect("serialize the api");
    body.push('\n');
    fs::write(&out, body).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
    println!("{} roles -> {}", roles.len(), out.display());
}

fn join(roles: &[String], methods: &BTreeMap<&str, Vec<String>>) -> (Value, Vec<String>) {
    let mut errors = Vec::new();
    for entry in JOIN {
        if !roles.iter().any(|role| role == entry.role) {
            errors.push(format!(
                "`{}` is joined, and the `Role` enum has no such role",
                entry.role
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
        let Some(entry) = JOIN.iter().find(|entry| entry.role == role) else {
            errors.push(format!("`{role}` is in no row of `JOIN`"));
            continue;
        };
        let mut row = serde_json::Map::new();
        for surface in SURFACES {
            let omitted = OMITTED.iter().any(|(s, r, _)| *s == surface && r == role);
            let found = if omitted {
                Vec::new()
            } else {
                serving(entry, surface, &methods[surface])
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

/// The entries of `methods` that serve `entry` on `surface`.
fn serving(entry: &Join, surface: &str, methods: &[String]) -> Vec<String> {
    let (class, name) = entry
        .rust
        .split_once("::")
        .expect("`Type::method` in `JOIN`");
    let names: Vec<String> = match surface {
        "cli" => entry
            .cli
            .iter()
            .map(|verb| format!("govee {verb}"))
            .collect(),
        "rust" => vec![entry.rust.to_owned()],
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

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}
