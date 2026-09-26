//! `dist/api.json`: the method that serves each role on each surface.
//!
//! A Rust method serves the roles that the `Serves` line of its doc comment
//! links. A CLI verb serves the roles of the methods that its dispatch arm
//! reaches.

use std::collections::BTreeMap;
use std::path::Path;
use std::process;

use govee_toolkit::codec::Role;
use serde_json::{Value, json};

mod dispatch;
mod rust;
mod text;

/// `(surface, role, reason)`: a role that a surface leaves out on purpose.
const OMITTED: &[(&str, &str, &str)] = &[];

const SURFACES: [&str; 4] = ["cli", "rust", "node", "python"];

/// Exits 1 when a role has no method on a surface.
pub(crate) fn api(root: &Path, check: bool) {
    let rust_root = root.join("packages/rust");
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

    let serves = rust::serves(&rust_root.join("src"));
    let verbs = dispatch::verbs(&rust_root.join("crates/cli/src/run"));
    let (joined, errors) = join(&serves, &verbs, &methods);
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("api: {error}");
        }
        eprintln!(
            "Link the role in the `Serves` line of its method, or name it in `OMITTED` in crates/xtask/src/api."
        );
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
    println!("{} roles -> {}", Role::CLAIMABLE.len(), out.display());
}

/// `serves` is `(role variant, Type::method)`.
fn join(
    serves: &[(String, String)],
    verbs: &BTreeMap<String, Vec<String>>,
    methods: &BTreeMap<&str, Vec<String>>,
) -> (Value, Vec<String>) {
    let mut errors = Vec::new();
    for (variant, method) in serves {
        if !Role::CLAIMABLE
            .iter()
            .any(|role| format!("{role:?}") == *variant)
        {
            errors.push(format!(
                "`{method}` serves `Role::{variant}`, which is no role"
            ));
        }
    }
    for (surface, role, _) in OMITTED {
        if !Role::CLAIMABLE
            .iter()
            .any(|known| known.to_string() == *role)
        {
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
    for role in Role::CLAIMABLE {
        let (variant, name) = (format!("{role:?}"), role.to_string());
        let serving: Vec<&str> = serves
            .iter()
            .filter(|(v, _)| *v == variant)
            .map(|(_, method)| method.as_str())
            .collect();
        let mut row = serde_json::Map::new();
        for surface in SURFACES {
            let omitted = OMITTED.iter().any(|(s, r, _)| *s == surface && *r == name);
            let found = if omitted {
                Vec::new()
            } else {
                let mut found: Vec<String> = serving
                    .iter()
                    .flat_map(|rust| on_surface(rust, verbs, surface, &methods[surface]))
                    .collect();
                found.sort();
                found.dedup();
                found
            };
            if found.is_empty() && !omitted {
                errors.push(format!("`{name}` has no method on `{surface}`"));
            }
            row.insert(surface.to_owned(), json!(found));
        }
        joined.insert(name, Value::Object(row));
    }
    (Value::Object(joined), errors)
}

fn on_surface(
    rust: &str,
    verbs: &BTreeMap<String, Vec<String>>,
    surface: &str,
    methods: &[String],
) -> Vec<String> {
    let Some((class, name)) = rust.split_once("::") else {
        return Vec::new();
    };
    let names: Vec<String> = match surface {
        "cli" => verbs
            .get(name)
            .into_iter()
            .flatten()
            .map(|verb| format!("govee {verb}"))
            .collect(),
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

/// `Type.name`, or `govee verb`.
fn head(method: &str) -> &str {
    let end = match method.strip_prefix("govee ") {
        Some(rest) => rest.find(' ').map_or(method.len(), |i| i + "govee ".len()),
        None => method.find('(').unwrap_or(method.len()),
    };
    &method[..end]
}
