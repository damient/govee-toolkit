//! Structural checks on a device file.
//!
//! These are the conventions in `devices/README.md` and `CLAUDE.md`, made
//! machine-checkable: a frame that does not parse, a placeholder with no
//! argument behind it, an undocumented command with no note pointing at the
//! protocol documentation. They run in CI and in this crate's tests.
//!
//! They check the *shape* of a file, never whether a device really behaves that
//! way — that stays a matter of capture and verification.

use crate::codec::catalog::{Command, Device, Mode, Role};
use crate::codec::frame::{Frame, Token};

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

        // The SDK picks these commands by role, so two claimants leave it with
        // nothing to pick.
        for role in Role::CLAIMABLE {
            let claimants: Vec<(&str, &Command)> = device
                .commands
                .get(mode)
                .iter()
                .filter(|(_, command)| command.role == Some(role))
                .map(|(name, command)| (name.as_str(), command))
                .collect();
            if claimants.len() > 1 {
                problems.push(at(
                    &mode.to_string(),
                    format!(
                        "`role: {role}` is claimed by {}; at most one command may claim a role",
                        claimants
                            .iter()
                            .map(|(name, _)| *name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
            for (name, command) in claimants {
                problems.extend(
                    check_role_args(role, command)
                        .into_iter()
                        .map(|message| at(&format!("{mode}.{name}"), message)),
                );
            }
        }
    }

    if let Some(measured) = device.measurements.native_pixels
        && device.capabilities.native_pixels != 0
        && device.capabilities.native_pixels != measured
    {
        problems.push(at(
            "capabilities.native_pixels",
            format!(
                "is {}, but `measurements.native_pixels` records {measured} on the unit measured",
                device.capabilities.native_pixels
            ),
        ));
    }

    problems
}

/// A role the SDK invokes on its own has to be callable without a command name
/// in code, so the arguments it fills are named by the role rather than by the
/// file. See `devices/schema.yaml`.
fn check_role_args(role: Role, command: &Command) -> Vec<String> {
    let required: &[(&str, &str)] = match role {
        Role::SegmentEnable => &[("on", crate::codec::args::INT)],
        Role::SegmentColor => &[("colors", crate::codec::args::RGB_LIST)],
        Role::Status => &[],
    };
    required
        .iter()
        .filter_map(|(arg, expected)| match command.args.get(*arg) {
            None => Some(format!("`role: {role}` must declare an argument `{arg}`")),
            Some(spec) if spec.type_name() != *expected => Some(format!(
                "`role: {role}` needs `{arg}` to be {expected}, not {}",
                spec.type_name()
            )),
            Some(_) => None,
        })
        .collect()
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
            if let Some(inner) = crate::codec::command::placeholder(s) {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;
    use crate::codec::Catalog;

    fn device_file(commands: &str) -> String {
        format!(
            "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
             capabilities: {{}}\ncommands:\n  lan:\n{commands}"
        )
    }

    fn parse(commands: &str) -> Catalog {
        Catalog::from_sources([("HTEST.yaml", device_file(commands).as_str())])
            .expect("the device file parses")
    }

    #[test]
    fn one_command_may_report_state() {
        let catalog = parse(
            "    status:\n      cmd: devStatus\n      documented: true\n      role: status\n",
        );
        let device = catalog.device("HTEST").expect("the SKU resolves");
        assert_eq!(device.status_command(Mode::Lan), Some("status"));
        assert!(super::device(device).is_empty());
    }

    #[test]
    fn a_file_may_report_no_state_at_all() {
        let catalog = parse("    power:\n      cmd: turn\n      documented: true\n");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        assert_eq!(device.status_command(Mode::Lan), None);
        assert!(super::device(device).is_empty());
    }

    #[test]
    fn two_claimants_leave_nothing_to_pick() {
        let catalog = parse(
            "    status:\n      cmd: devStatus\n      documented: true\n      role: status\n\
             \n    other:\n      cmd: status\n      documented: true\n      role: status\n",
        );
        let device = catalog.device("HTEST").expect("the SKU resolves");
        let problems = super::device(device);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].message.contains("role: status"));
    }
}
