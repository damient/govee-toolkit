//! `args.json`: every argument that clap declares, as `xtask api` reads it.
//! `crates/cli/src/args.rs` holds the same writer for `govee`.

#![allow(clippy::expect_used)]

use std::path::Path;
use std::{env, fs};

use clap::{ArgAction, Command, CommandFactory as _};
use serde_json::{Map, Value, json};

use crate::Cli;

/// `GOVEE_BLESS=1` rewrites the file.
#[test]
fn args_json_matches_the_parser() {
    let mut out = Map::new();
    walk("govee-dmx", &Cli::command(), &mut out);
    let text = serde_json::to_string_pretty(&Value::Object(out)).expect("serialize") + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("args.json");
    if env::var_os("GOVEE_BLESS").is_some() {
        fs::write(&path, &text).expect("write args.json");
    }
    let found = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        found == text,
        "args.json is stale: run `GOVEE_BLESS=1 cargo test -p govee-toolkit-dmx`"
    );
}

fn walk(path: &str, command: &Command, out: &mut Map<String, Value>) {
    let args: Vec<Value> = command
        .get_arguments()
        .filter(|arg| path == "govee-dmx" || !arg.is_global_set())
        .map(|arg| {
            let flag = matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse);
            let value = arg
                .get_value_names()
                .and_then(|names| names.first())
                .map_or_else(|| arg.get_id().as_str().to_uppercase(), ToString::to_string);
            let name = arg
                .get_long()
                .map_or_else(|| value.clone(), |long| format!("--{long}"));
            let default: Vec<String> = arg
                .get_default_values()
                .iter()
                .map(|v| v.to_string_lossy().into_owned())
                .collect();
            let choices: Vec<String> = arg
                .get_possible_values()
                .iter()
                .filter(|choice| !choice.is_hide_set())
                .map(|choice| choice.get_name().to_owned())
                .collect();
            json!({
                "name": name,
                "value": if flag { Value::Null } else { json!(value) },
                "choices": if flag || choices.is_empty() { Value::Null } else { json!(choices) },
                "required": arg.is_required_set(),
                "multiple": matches!(arg.get_action(), ArgAction::Append),
                "default": if flag || default.is_empty() { Value::Null } else { json!(default.join(",")) },
            })
        })
        .collect();
    out.insert(path.to_owned(), Value::Array(args));
    for sub in command.get_subcommands() {
        walk(&format!("{path} {}", sub.get_name()), sub, out);
    }
}
