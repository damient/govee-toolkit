//! Structural checks on a device file.
//!
//! These are the conventions in `devices/README.md` and `CLAUDE.md`, made
//! machine-checkable: a frame that does not parse, a placeholder with no
//! argument behind it, an undocumented command with no note pointing at the
//! protocol documentation. They run in CI and in this crate's tests.
//!
//! They check the *shape* of a file, never whether a device really behaves that
//! way — that stays a matter of capture and verification.

use crate::catalog::{Command, Device, Mode};
use crate::frame::{Frame, Token};

/// One thing wrong with a device file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The SKU the file declares.
    pub sku: String,
    /// Where it is, as `mode.command` or a bare field name.
    pub at: String,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}: {}", self.sku, self.at, self.message)
    }
}

/// Check one device file. An empty result means it is well-formed.
#[must_use]
pub fn device(device: &Device) -> Vec<Problem> {
    let mut problems = Vec::new();
    let at = |at: &str, message: String| Problem {
        sku: device.sku.clone(),
        at: at.to_owned(),
        message,
    };

    if device.sku.is_empty() {
        problems.push(at("sku", "empty".to_owned()));
    } else if device.sku != device.sku.to_uppercase() {
        problems.push(at("sku", "must be uppercase".to_owned()));
    }

    for alias in &device.candidate_aliases {
        if device.aliases.contains(alias) {
            problems.push(at(
                "candidate_aliases",
                format!("`{alias}` is also listed under `aliases`; it is either verified or not"),
            ));
        }
    }

    for mode in [Mode::Lan, Mode::Ble, Mode::Cloud] {
        for (name, command) in device.commands.get(mode) {
            let at_command = format!("{mode}.{name}");
            problems.extend(
                check_command(name, command)
                    .into_iter()
                    .map(|message| at(&at_command, message)),
            );
        }
    }

    problems
}

fn check_command(name: &str, command: &Command) -> Vec<String> {
    let mut problems = Vec::new();

    if !command.documented {
        // An undocumented command is only reproducible if the protocol section
        // behind it is written down. See CLAUDE.md, "Device files".
        if command.notes.trim().is_empty() {
            problems.push("undocumented, but carries no `notes:`".to_owned());
        } else if !command.notes.contains("docs/protocol/") {
            problems.push("undocumented `notes:` does not point at docs/protocol/".to_owned());
        }
    }

    let frame = match command.frame.as_deref() {
        None => None,
        Some(source) => match Frame::parse(name, source) {
            Ok(frame) => Some(frame),
            Err(e) => {
                problems.push(e.to_string());
                None
            }
        },
    };

    if let Some(frame) = &frame {
        for token in frame.tokens() {
            let referenced: &[&str] = match token {
                Token::Arg { name, .. } => &[name.as_str()],
                Token::Repeat { list, count, .. } => &[list.as_str(), count.as_str()],
                _ => &[],
            };
            for arg in referenced {
                if !command.args.contains_key(*arg) {
                    problems.push(format!(
                        "frame refers to `{arg}`, which `args:` does not declare"
                    ));
                }
            }
        }
    }

    let mut placeholders = Vec::new();
    collect_placeholders(&command.payload, &mut placeholders);
    for name in &placeholders {
        if name == "frame" {
            if frame.is_none() {
                problems.push("payload uses `${frame}` but no `frame:` is declared".to_owned());
            }
        } else if !command.args.contains_key(name) {
            problems.push(format!(
                "payload refers to `{name}`, which `args:` does not declare"
            ));
        }
    }
    if frame.is_some() && !placeholders.iter().any(|p| p == "frame") {
        problems.push("declares a `frame:` the payload never carries with `${frame}`".to_owned());
    }

    problems
}

fn collect_placeholders(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(inner) = s.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
                out.push(inner.to_owned());
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_placeholders(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_placeholders(v, out);
            }
        }
        _ => {}
    }
}
