//! Bounds for an integer argument, and how `capability` resolves.
//!
//! A device file writes `range: [min, max]`, or the keyword `capability` to
//! take the pair from the device's own `capabilities:`. The keyword keeps one
//! number in one place: a shared table declares the layout, and the device
//! file declares the bounds once, where a reader looks for them. The catalog
//! resolves the keyword when it loads the file, so nothing on the send path
//! reads `capabilities:`.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::spec::{ArgRole, ArgSpec};
use super::{Command, Device, Mode};
use crate::codec::capabilities::Capabilities;
use crate::codec::error::{Error, Result};

/// Inclusive bounds for an integer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKeyword {
    /// `range: capability`
    Capability,
}

/// Which capability parameter a role takes its bounds from.
///
/// A role absent here carries no bounds anywhere in `capabilities:`, so
/// `range: capability` on it is a mistake in the file rather than a gap.
fn source(role: ArgRole) -> Option<(&'static str, Parameter)> {
    match role {
        ArgRole::Brightness => Some(("brightness", Parameter::Range)),
        ArgRole::ColorTemp => Some(("colortemp", Parameter::RangeKelvin)),
        _ => None,
    }
}

/// The parameter of one capability that carries a pair.
#[derive(Clone, Copy)]
enum Parameter {
    /// `range:`
    Range,
    /// `range_kelvin:`
    RangeKelvin,
}

impl Parameter {
    fn name(self) -> &'static str {
        match self {
            Self::Range => "range",
            Self::RangeKelvin => "range_kelvin",
        }
    }

    fn read(self, capabilities: &Capabilities, capability: &str) -> Option<[i64; 2]> {
        let params = capabilities.get(capability)?;
        match self {
            Self::Range => params.range,
            Self::RangeKelvin => params.range_kelvin,
        }
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
    let capabilities = device.capabilities.clone();
    for mode in Mode::ALL {
        resolve_table(file, mode, device.commands.get_mut(mode), &capabilities)?;
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
            let pair = role
                .and_then(source)
                .and_then(|(capability, parameter)| parameter.read(capabilities, capability))
                .ok_or_else(|| Error::CapabilityBounds {
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
        Some((capability, parameter)) => {
            format!("`capabilities.{capability}.{}`", parameter.name())
        }
        None => "a role that names a capability parameter, such as `brightness` \
                 or `color_temp`"
            .to_owned(),
    }
}
