//! Bounds for an integer argument, and how `capability` resolves.
//!
//! A device file writes `range: [min, max]`, or the keyword `capability` to
//! take the pair from the device's own `capabilities:`. The keyword keeps one
//! number in one place: a shared table declares the layout, and the device
//! file declares the bounds once, where a reader looks for them. The catalog
//! resolves the keyword when it loads the file, so nothing on the send path
//! reads `capabilities:`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::spec::{ArgRole, ArgSpec};
use super::{Command, Device, Mode};
use crate::codec::capabilities::{Capabilities, CapabilityParams};
use crate::codec::error::{Error, Result};

/// Inclusive bounds for an integer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bounds {
    /// `[min, max]`, both inclusive.
    Literal([i64; 2]),
    /// `capability`, before the catalog resolves it.
    FromCapability(CapabilityKeyword),
}

impl Bounds {
    /// The pair, or `None` while `capability` is unresolved.
    ///
    /// Every device the catalog hands out carries pairs: the keyword is
    /// resolved on load, and a file it cannot be resolved for fails to load.
    #[must_use]
    pub fn pair(self) -> Option<[i64; 2]> {
        match self {
            Self::Literal(pair) => Some(pair),
            Self::FromCapability(_) => None,
        }
    }
}

/// The `capability` keyword, as it appears in a device file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKeyword {
    /// `range: capability`
    Capability,
}

/// Reads one parameter of [`CapabilityParams`] that carries a pair.
type Read = fn(&CapabilityParams) -> Option<[i64; 2]>;

/// Which capability parameter a role takes its bounds from, as the capability
/// name, the parameter name, and how to read it.
///
/// A role absent here carries no bounds anywhere in `capabilities:`, so
/// `range: capability` on it is a mistake in the file rather than a gap.
fn source(role: ArgRole) -> Option<(&'static str, &'static str, Read)> {
    match role {
        ArgRole::Brightness => Some(("brightness", "range", |params| params.range)),
        ArgRole::ColorTemp => Some(("colortemp", "range_kelvin", |params| params.range_kelvin)),
        _ => None,
    }
}

/// Replace every `range: capability` in `device` with the pair its
/// `capabilities:` declares.
///
/// Run after an `include:` merges, so a shared table reaches this too.
///
/// # Errors
///
/// [`Error::CapabilityBounds`] where the argument carries no role that names
/// a capability parameter, or where the device declares no such pair.
pub fn resolve(file: &str, device: &mut Device) -> Result<()> {
    let Device {
        capabilities,
        commands,
        ..
    } = device;
    for mode in Mode::ALL {
        resolve_table(file, mode, commands.get_mut(mode), capabilities)?;
    }
    Ok(())
}

fn resolve_table(
    file: &str,
    mode: Mode,
    table: &mut BTreeMap<String, Command>,
    capabilities: &Capabilities,
) -> Result<()> {
    for (command, spec) in table.iter_mut() {
        for (arg, declared) in &mut spec.args {
            let ArgSpec::Int { range, role } = declared else {
                continue;
            };
            if range.pair().is_some() {
                continue;
            }
            let found = role
                .and_then(source)
                .and_then(|(capability, _, read)| capabilities.get(capability).and_then(read));
            let pair = found.ok_or_else(|| Error::CapabilityBounds {
                file: file.to_owned(),
                mode,
                command: command.clone(),
                arg: arg.clone(),
                needs: needs(*role),
            })?;
            *range = Bounds::Literal(pair);
        }
    }
    Ok(())
}

/// What the file must supply, for the error message.
fn needs(role: Option<ArgRole>) -> String {
    match role.and_then(source) {
        Some((capability, parameter, _)) => format!("`capabilities.{capability}.{parameter}`"),
        None => "a role that names a capability parameter, such as `brightness` \
                 or `color_temp`"
            .to_owned(),
    }
}
