//! Naming devices on a command line: by identity, by SKU or by name.
//!
//! One target names one kind of thing. A prefix states the kind, and a bare
//! target takes the kind it reads as:
//!
//! | Target | Kind |
//! | ------ | ---- |
//! | `1C:8B:C4:A2:C0:46:64:6E`, `id:…` | the identity a device reports |
//! | `H6159`, `sku:H6159` | every known device of that model |
//! | `name:kitchen` | the device the configuration names |
//!
//! A bare target reads as a SKU where a device file is encoded under it, and
//! never from the shape of the text alone. A bare target that reads as an
//! identity or a SKU, and that a known device also carries as a name, is
//! refused. Nothing is guessed.
//!
//! A SKU matches the SKU a device is encoded under, and no alias of it: an
//! operator who types one model does not mean the other.

mod one;
#[cfg(test)]
mod tests;

use std::fmt;

use crate::codec::{Catalog, Mode};
use crate::event::Device;
use crate::govee::Govee;
use crate::transport::DeviceId;

const ID: &str = "id";
const SKU: &str = "sku";
const NAME: &str = "name";

/// One target, read from what a person typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// One identity. It selects that device whether a scan found it or not.
    Id(DeviceId),
    /// Every known device encoded under this SKU.
    Sku(String),
    /// Every known device the configuration gives this name. The comparison
    /// ignores case and is exact.
    Name(String),
}

impl Selector {
    /// Read one target, against the catalog that says which SKUs exist.
    ///
    /// It does not report the ambiguity between a SKU and a device named like
    /// one: that needs the devices to see, so [`Govee::select`] reports it.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] for a target with nothing in it, and
    /// [`Error::EmptyValue`] for a prefix with nothing after it.
    pub fn parse(target: &str, catalog: &Catalog) -> Result<Self, Error> {
        read(target, catalog).map(|(selector, _)| selector)
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{ID}:{id}"),
            Self::Sku(sku) => write!(f, "{SKU}:{sku}"),
            Self::Name(name) => write!(f, "{NAME}:{name}"),
        }
    }
}

/// One target, and whether a prefix stated its kind. A prefixed target is
/// never ambiguous.
fn read(target: &str, catalog: &Catalog) -> Result<(Selector, bool), Error> {
    let target = target.trim();
    if target.is_empty() {
        return Err(Error::Empty);
    }
    if let Some((prefix, value)) = target.split_once(':')
        && let Some(kind) = kind(prefix)
    {
        let value = value.trim();
        if value.is_empty() {
            return Err(Error::EmptyValue {
                prefix: kind.as_str().to_owned(),
            });
        }
        return Ok((kind.read(value), true));
    }
    Ok((bare(target, catalog), false))
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Id,
    Sku,
    Name,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Id => ID,
            Self::Sku => SKU,
            Self::Name => NAME,
        }
    }

    fn read(self, value: &str) -> Selector {
        match self {
            Self::Id => Selector::Id(DeviceId::new(value)),
            Self::Sku => Selector::Sku(value.to_uppercase()),
            Self::Name => Selector::Name(value.to_owned()),
        }
    }
}

/// `AA:BB:…` reaches this with `AA` and takes the shape path instead.
fn kind(prefix: &str) -> Option<Kind> {
    match prefix.trim().to_ascii_lowercase().as_str() {
        ID => Some(Kind::Id),
        SKU => Some(Kind::Sku),
        NAME => Some(Kind::Name),
        _ => None,
    }
}

fn bare(target: &str, catalog: &Catalog) -> Selector {
    if is_identity(target) {
        return Selector::Id(DeviceId::new(target));
    }
    if is_sku(target, catalog) {
        return Selector::Sku(target.to_uppercase());
    }
    Selector::Name(target.to_owned())
}

/// Hexadecimal groups a colon joins, which `lan` reports, or the dashed UUID
/// a platform gives a Bluetooth peripheral. The 12 digits keep a hyphenated
/// name such as `bed-a-b-c-d` out.
fn is_identity(target: &str) -> bool {
    let groups: Vec<&str> = target.split([':', '-']).collect();
    let hexadecimal = groups
        .iter()
        .all(|group| !group.is_empty() && group.chars().all(|c| c.is_ascii_hexdigit()));
    let digits: usize = groups.iter().map(|group| group.len()).sum();
    groups.len() >= 5 && digits >= 12 && hexadecimal
}

/// An alias answers `false`: [`matches`] selects the SKU a device is encoded
/// under, so an alias would select nothing.
fn is_sku(target: &str, catalog: &Catalog) -> bool {
    catalog
        .devices()
        .any(|device| device.sku.eq_ignore_ascii_case(target))
}

/// Why a target names no device.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A target with nothing in it.
    #[error("a target names no device")]
    Empty,

    /// A prefix with nothing after it.
    #[error("`{prefix}:` names no {prefix}")]
    EmptyValue {
        /// The prefix that was written.
        prefix: String,
    },

    /// A bare target reads as one kind, and a known device carries it as a
    /// name. The prefixed forms say which one is meant.
    #[error(
        "`{target}` is both a {kind} and the name of a device; write `{kind}:{target}` or `name:{target}`"
    )]
    Ambiguous {
        /// What was written.
        target: String,
        /// The kind it reads as.
        kind: String,
    },

    /// A target that matches no device the SDK knows. A scan is what makes a
    /// device known; the cache answers for `lan` between runs.
    #[error("`{target}` matches no known device")]
    NoMatch {
        /// The target, in its prefixed form.
        target: String,
    },

    /// A SKU or a name matches known devices, and the configuration enables
    /// the mode the caller will drive for none of them.
    #[error("`{target}` matches no known device that enables `{mode}`")]
    NotOnMode {
        /// The target, in its prefixed form.
        target: String,
        /// The mode the caller will drive.
        mode: Mode,
    },

    /// A command that drives one device got a target that names a model.
    #[error("`{target}` names a model; this command takes one device, by identity or by name")]
    NotOne {
        /// The target, in its prefixed form.
        target: String,
    },

    /// A command that drives one device got a name that the configuration
    /// gives to more than one device.
    #[error("`{target}` names more than one device in the configuration")]
    Several {
        /// The target, in its prefixed form.
        target: String,
    },
}

impl Error {
    /// A stable, language-neutral identifier for this failure. One namespace
    /// with [`crate::Error::code`].
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty | Self::EmptyValue { .. } | Self::NotOne { .. } => "target_not_understood",
            Self::Ambiguous { .. } | Self::Several { .. } => "ambiguous_target",
            Self::NoMatch { .. } | Self::NotOnMode { .. } => "no_such_target",
        }
    }
}

impl Govee {
    /// The devices the targets name, in the order they were written.
    ///
    /// `mode` is the one mode the caller will drive, and `None` where it
    /// drives none and lists instead. A SKU and a name then match among the
    /// devices that enable `mode`. An identity selects itself, found or not.
    ///
    /// A device two targets name appears once, at the first place it was
    /// named. A SKU and a name select among the devices the SDK knows, so
    /// scan first where nothing has been discovered yet.
    ///
    /// # Errors
    ///
    /// [`Error::Ambiguous`] where a bare target reads as two kinds,
    /// [`Error::NoMatch`] where a SKU or a name matches no known device, and
    /// [`Error::NotOnMode`] where the devices it matches enable no `mode`.
    /// Every [`Error`] [`Selector::parse`] reports travels out of here too.
    pub fn select<I, T>(&self, targets: I, mode: Option<Mode>) -> Result<Vec<DeviceId>, Error>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let known = self.devices();
        let mut chosen: Vec<DeviceId> = Vec::new();
        for target in targets {
            let target = target.as_ref();
            let (selector, prefixed) = read(target, self.catalog())?;
            if !prefixed {
                check_ambiguity(target, &selector, &known)?;
            }
            for id in matches(&selector, &known, mode)? {
                if !chosen.contains(&id) {
                    chosen.push(id);
                }
            }
        }
        Ok(chosen)
    }
}

fn check_ambiguity(written: &str, selector: &Selector, known: &[Device]) -> Result<(), Error> {
    let kind = match selector {
        Selector::Id(_) => ID,
        Selector::Sku(_) => SKU,
        Selector::Name(_) => return Ok(()),
    };
    if named(known, written).next().is_none() {
        return Ok(());
    }
    Err(Error::Ambiguous {
        target: written.to_owned(),
        kind: kind.to_owned(),
    })
}

/// The mode narrows the match and never turns an empty match into a mode
/// fault: nothing matched is [`Error::NoMatch`], matched but on another mode
/// is [`Error::NotOnMode`].
fn matches(
    selector: &Selector,
    known: &[Device],
    mode: Option<Mode>,
) -> Result<Vec<DeviceId>, Error> {
    let found: Vec<&Device> = match selector {
        Selector::Id(id) => return Ok(vec![id.clone()]),
        Selector::Sku(sku) => known
            .iter()
            .filter(|device| device.sku.eq_ignore_ascii_case(sku))
            .collect(),
        Selector::Name(name) => named(known, name).collect(),
    };
    if found.is_empty() {
        return Err(Error::NoMatch {
            target: selector.to_string(),
        });
    }
    let driven: Vec<DeviceId> = found
        .iter()
        .filter(|device| mode.is_none_or(|mode| device.modes.contains(&mode)))
        .map(|device| device.id.clone())
        .collect();
    match (driven.is_empty(), mode) {
        (true, Some(mode)) => Err(Error::NotOnMode {
            target: selector.to_string(),
            mode,
        }),
        _ => Ok(driven),
    }
}

fn named<'a>(known: &'a [Device], name: &'a str) -> impl Iterator<Item = &'a Device> + 'a {
    known.iter().filter(move |device| {
        device
            .name
            .as_deref()
            .is_some_and(|given| given.eq_ignore_ascii_case(name))
    })
}
