//! Structural checks on a device file.
//!
//! These are the conventions in `devices/README.md` and `CLAUDE.md`, made
//! machine-checkable: a frame that does not parse, a placeholder with no
//! argument behind it, an undocumented command with no note pointing at the
//! protocol documentation. They run in CI and in this crate's tests.
//!
//! They check the *shape* of a file, never whether a device really behaves that
//! way — that stays a matter of capture and verification.

use self::command::{check_command, check_role_args};
use self::modes::check_mode_capabilities;
use crate::codec::catalog::{Command, Device, Mode, Role};

mod command;
mod modes;

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
                check_command(mode, name, command)
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

    for mode in [Mode::Lan, Mode::Ble, Mode::Cloud] {
        problems.extend(
            check_mode_capabilities(device, mode)
                .into_iter()
                .map(|(field, message)| at(&format!("modes.{mode}.{field}"), message)),
        );
    }

    if let (Some(measured), Some(declared)) = (
        device.measurements.native_pixels,
        device.capabilities.native_pixels(),
    ) && declared != measured
    {
        problems.push(at(
            "capabilities.segments.native_pixels",
            format!(
                "is {declared}, but `measurements.native_pixels` records {measured} on the unit measured"
            ),
        ));
    }

    problems
}

#[cfg(test)]
mod tests;
