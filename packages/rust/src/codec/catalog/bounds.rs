//! Bounds for an integer argument, and how a capability reference resolves.
//!
//! The catalog resolves a reference when it loads the device file, so nothing
//! on the send path reads `capabilities:`.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::spec::ArgSpec;
use super::{Command, Device, Mode};
use crate::codec::capabilities::{Capabilities, PAIR_PARAMS};
use crate::codec::error::{Error, Result};

/// Inclusive bounds for an integer argument.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bounds {
    /// `[min, max]`, both inclusive.
    Literal([i64; 2]),
    /// A capability parameter, before the catalog resolves it.
    FromCapability(CapabilityRef),
}

impl Bounds {
    /// The pair, or `None` while a reference is unresolved. Every device the
    /// catalog hands out carries pairs.
    #[must_use]
    pub fn pair(&self) -> Option<[i64; 2]> {
        match self {
            Self::Literal(pair) => Some(*pair),
            Self::FromCapability(_) => None,
        }
    }
}

/// What `devices/*.yaml` bounds one argument by. Every surface that reports a
/// bound reads this, so a new [`ArgSpec`] states its bound once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgBound {
    /// The inclusive pair an `int` takes.
    Range([i64; 2]),
    /// How many zones a mask counts.
    Zones(usize),
    /// The length a list, a text or a run of bytes caps at.
    MaxLen(usize),
    /// The file declares none.
    None,
}

impl ArgSpec {
    /// What the file bounds this argument by.
    ///
    /// [`ArgBound::None`] where the file declares no bound, and for an `int`
    /// whose capability reference the catalog has not resolved yet.
    #[must_use]
    pub fn bound(&self) -> ArgBound {
        match self {
            Self::Int { range, .. } => range
                .as_ref()
                .and_then(Bounds::pair)
                .map_or(ArgBound::None, ArgBound::Range),
            Self::Zones { count, .. } => count.map_or(ArgBound::None, ArgBound::Zones),
            Self::RgbList { max_len, .. }
            | Self::String { max_len, .. }
            | Self::Bytes { max_len, .. } => max_len.map_or(ArgBound::None, ArgBound::MaxLen),
        }
    }
}

/// Where a device file says the pair lives, in its own `capabilities:`.
/// Written and read as `<capability>.<parameter>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRef {
    /// The capability name, as `capabilities:` writes it.
    pub capability: String,
    /// The parameter of that capability. It must carry a pair — see
    /// [`PAIR_PARAMS`].
    pub parameter: String,
}

impl fmt::Display for CapabilityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.capability, self.parameter)
    }
}

impl<'de> Deserialize<'de> for CapabilityRef {
    fn deserialize<D: Deserializer<'de>>(de: D) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(de)?;
        let (capability, parameter) = text
            .split_once('.')
            .ok_or_else(|| D::Error::custom(format!("`{text}` names no parameter")))?;
        if capability.is_empty() || parameter.contains('.') || parameter.is_empty() {
            return Err(D::Error::custom(format!(
                "`{text}` is not `<capability>.<parameter>`"
            )));
        }
        Ok(Self {
            capability: capability.to_owned(),
            parameter: parameter.to_owned(),
        })
    }
}

impl Serialize for CapabilityRef {
    fn serialize<S: Serializer>(&self, se: S) -> std::result::Result<S::Ok, S::Error> {
        se.collect_str(self)
    }
}

/// Replace every capability reference in `device` with the pair its
/// `capabilities:` declares.
///
/// Run after an `include:` merges, so a shared table reaches this too.
///
/// # Errors
///
/// [`Error::CapabilityBounds`] where the parameter carries no pair, or where
/// the device declares no such pair.
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
            let ArgSpec::Int {
                range: Some(range), ..
            } = declared
            else {
                continue;
            };
            let Bounds::FromCapability(reference) = range else {
                continue;
            };
            let pair =
                read(reference, capabilities).map_err(|problem| Error::CapabilityBounds {
                    file: file.to_owned(),
                    mode,
                    command: command.clone(),
                    arg: arg.clone(),
                    problem,
                })?;
            *range = Bounds::Literal(pair);
        }
    }
    Ok(())
}

fn read(
    reference: &CapabilityRef,
    capabilities: &Capabilities,
) -> std::result::Result<[i64; 2], String> {
    let parameter = reference.parameter.as_str();
    if !PAIR_PARAMS.contains(&parameter) {
        let known = PAIR_PARAMS.join("`, `");
        return Err(format!(
            "`{parameter}` carries no pair; the parameters that do are `{known}`"
        ));
    }
    capabilities
        .get(&reference.capability)
        .and_then(|params| params.pair(parameter))
        .ok_or_else(|| format!("this file declares no `capabilities.{reference}`"))
}
