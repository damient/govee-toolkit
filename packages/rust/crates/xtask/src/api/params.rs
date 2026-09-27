//! The parameters of each method on each surface: the name, the type, whether
//! the caller must pass it, and the default that the surface declares.
//!
//! A Python default is the one in the stub, which `mypy.stubtest` checks
//! against the module. A Node default is an `@param [name=value]` line of the
//! doc comment. A CLI default is the one clap reports, in `args.json`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::text::{self, split_any, split_top};

/// `(name, type, required, default)`.
pub(super) type Param = (String, String, bool, Option<String>);

pub(super) type Params = BTreeMap<String, Vec<Param>>;

pub(super) fn to_json(params: &Params) -> Value {
    params
        .iter()
        .map(|(key, rows)| {
            let rows: Vec<Value> = rows
                .iter()
                .map(|(name, ty, required, default)| {
                    json!({ "name": name, "type": ty, "required": required, "default": default })
                })
                .collect();
            (key.clone(), Value::Array(rows))
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The part before `(` and the arguments inside it. A getter has none.
fn split(method: &str) -> Option<(&str, &str)> {
    let open = method.find('(')?;
    let inner = method.get(open + 1..method.len().checked_sub(1)?)?;
    Some((&method[..open], inner))
}

/// `name: Type`. `Option<T>` is the one form the caller can leave out.
pub(super) fn rust(methods: &[String]) -> Params {
    let mut out = Params::new();
    for method in methods {
        let Some((key, inner)) = split(method) else {
            continue;
        };
        let rows = split_top(inner)
            .into_iter()
            .filter_map(|arg| {
                let (name, ty) = arg.trim().split_once(": ")?;
                let required = !ty.starts_with("Option<");
                Some((name.to_owned(), ty.to_owned(), required, None))
            })
            .collect();
        out.insert(key.to_owned(), rows);
    }
    out
}

/// `name: type = default`. A bare `*` only marks the keywords that follow.
pub(super) fn python(methods: &[String]) -> Params {
    let mut out = Params::new();
    for method in methods {
        let Some((key, inner)) = split(method) else {
            continue;
        };
        let rows = split_top(inner)
            .into_iter()
            .map(str::trim)
            .filter(|arg| !arg.is_empty() && *arg != "*" && *arg != "/")
            .map(|arg| {
                let (head, default) = match top_level(arg, " = ") {
                    Some(at) => (&arg[..at], Some(arg[at + 3..].to_owned())),
                    None => (arg, None),
                };
                let (name, ty) = head.split_once(": ").unwrap_or((head, ""));
                let required = default.is_none() && !name.starts_with('*');
                (name.to_owned(), ty.to_owned(), required, default)
            })
            .collect();
        out.insert(key.to_owned(), rows);
    }
    out
}

/// `name?: type`. An object type gives one row for each key, as
/// `name.key`, since the caller writes the keys and not the object.
pub(super) fn node(
    methods: &[String],
    defaults: &BTreeMap<String, BTreeMap<String, String>>,
) -> Params {
    let mut out = Params::new();
    for method in methods {
        let Some((key, inner)) = split(method) else {
            continue;
        };
        let known = defaults.get(key);
        let mut rows = Vec::new();
        for arg in split_top(inner)
            .into_iter()
            .map(str::trim)
            .filter(|a| !a.is_empty())
        {
            let (name, ty, required) = node_arg(arg);
            let members = ty
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
                .map(|body| split_any(body.trim(), &[',', ';']));
            let Some(members) = members else {
                let default = known.and_then(|d| d.get(&name)).cloned();
                rows.push((name, ty, required, default));
                continue;
            };
            for member in members.into_iter().map(str::trim).filter(|m| !m.is_empty()) {
                let (inner_name, inner_ty, inner_required) = node_arg(member);
                let full = format!("{name}.{inner_name}");
                let default = known.and_then(|d| d.get(&full)).cloned();
                rows.push((full, inner_ty, required && inner_required, default));
            }
        }
        out.insert(key.to_owned(), rows);
    }
    out
}

fn node_arg(arg: &str) -> (String, String, bool) {
    let (name, ty) = arg.split_once(':').unwrap_or((arg, ""));
    let name = name.trim();
    let ty = ty
        .trim()
        .trim_end_matches(" | undefined | null")
        .trim_end_matches(" | null");
    match name.strip_suffix('?') {
        Some(name) => (name.to_owned(), ty.to_owned(), false),
        None => (name.to_owned(), ty.to_owned(), true),
    }
}

/// The first `needle` outside brackets.
fn top_level(text: &str, needle: &str) -> Option<usize> {
    let mut level = 0i32;
    for (index, c) in text.char_indices() {
        level += text::bracket(text, index, c);
        if level == 0 && text[index..].starts_with(needle) {
            return Some(index);
        }
    }
    None
}

/// `args.json` of one binary: the command path to its arguments.
pub(super) fn cli(args: &Value) -> Params {
    let mut out = Params::new();
    let Some(commands) = args.as_object() else {
        return out;
    };
    for (path, rows) in commands {
        let rows = rows
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| {
                let text = |key: &str| row[key].as_str().map(str::to_owned);
                let choices: Option<Vec<&str>> = row["choices"]
                    .as_array()
                    .map(|all| all.iter().filter_map(Value::as_str).collect());
                let mut ty = match (choices, text("value")) {
                    (Some(choices), _) => choices.join(" | "),
                    (None, Some(value)) => value,
                    (None, None) => "flag".to_owned(),
                };
                if row["multiple"].as_bool() == Some(true) {
                    ty.push('…');
                }
                let name = text("name").unwrap_or_default();
                let required = row["required"].as_bool() == Some(true);
                (name, ty, required, text("default"))
            })
            .collect();
        out.insert(path.clone(), rows);
    }
    out
}
