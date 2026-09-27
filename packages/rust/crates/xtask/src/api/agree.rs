//! Each surface writes its own defaults: the check that they state one value.
//!
//! A parameter is matched by its name in `snake_case`, with the object
//! prefix of a Node option and the `--` of a flag removed. A flag that ends
//! in `-ms` counts milliseconds where the bindings count seconds.

use std::collections::BTreeMap;

use super::params::Params;
use super::text::camel;

#[derive(Debug, PartialEq)]
enum Value {
    Number(f64),
    Bool(bool),
    Text(String),
    Color([u8; 3]),
}

/// One error for each parameter that two surfaces give two defaults.
pub(super) fn defaults(
    rust: &Params,
    surfaces: &[(&str, &Params)],
    verbs: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for key in rust.keys() {
        let Some((class, name)) = key.split_once("::") else {
            continue;
        };
        let mut seen: BTreeMap<String, (String, Value)> = BTreeMap::new();
        for (surface, params) in surfaces {
            let keys: Vec<String> = match *surface {
                "cli" => verbs
                    .get(name)
                    .into_iter()
                    .flatten()
                    .map(|verb| format!("govee {verb}"))
                    .collect(),
                "node" => vec![format!("{class}.{}", camel(name))],
                _ => vec![format!("{class}.{name}")],
            };
            for rows in keys.iter().filter_map(|k| params.get(k)) {
                for (param, _, _, default) in rows {
                    let Some((id, value)) = default.as_deref().and_then(|d| normal(param, d))
                    else {
                        continue;
                    };
                    match seen.get(&id) {
                        Some((first, known)) if *known != value => errors.push(format!(
                            "`{key}`: `{id}` defaults to {known:?} on {first} and to {value:?} on {surface}"
                        )),
                        Some(_) => {}
                        None => {
                            seen.insert(id, ((*surface).to_owned(), value));
                        }
                    }
                }
            }
        }
    }
    errors
}

/// The shared name and the value, or `None` where the default is "absent".
fn normal(param: &str, default: &str) -> Option<(String, Value)> {
    let last = param.rsplit('.').next().unwrap_or(param);
    let flag = last.strip_prefix("--");
    let mut id = snake(flag.unwrap_or(last));
    let mut scale = 1.0;
    if flag.is_some()
        && let Some(stem) = id.strip_suffix("_ms")
    {
        id = stem.to_owned();
        scale = 1000.0;
    }
    let text = default.trim().trim_matches(['\'', '"']);
    let value = match text {
        "None" | "null" | "undefined" => return None,
        "True" | "true" => Value::Bool(true),
        "False" | "false" => Value::Bool(false),
        _ => {
            if let Ok(number) = text.parse::<f64>() {
                Value::Number(number / scale)
            } else if let Some(rgb) = color(text) {
                Value::Color(rgb)
            } else {
                Value::Text(text.to_owned())
            }
        }
    };
    Some((id, value))
}

/// `#RRGGBB`, `(r, g, b)` or `[r, g, b]`.
fn color(text: &str) -> Option<[u8; 3]> {
    if let Some(hex) = text.strip_prefix('#') {
        let byte = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
        return Some([byte(0)?, byte(2)?, byte(4)?]);
    }
    let inner = text.strip_prefix(['(', '['])?.strip_suffix([')', ']'])?;
    let channels: Vec<u8> = inner
        .split(',')
        .map(|c| c.trim().parse().ok())
        .collect::<Option<_>>()?;
    channels.try_into().ok()
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c == '-' {
            out.push('_');
        } else if c.is_uppercase() {
            out.push('_');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
