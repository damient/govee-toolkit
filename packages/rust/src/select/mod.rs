//! Naming devices on a command line: by identity, by SKU or by name.
//!
//! One target names one kind of thing. A prefix states the kind, and a bare
//! target takes the kind its shape gives it:
//!
//! | Target | Kind |
//! | ------ | ---- |
//! | `1C:8B:C4:A2:C0:46:64:6E`, `id:…` | the identity a device reports |
//! | `H6159`, `sku:H6159` | every known device of that model |
//! | `name:kitchen` | the device the configuration names |
//!
//! A bare target that its shape reads as an identity or a SKU, and that a
//! known device also carries as a name, is refused: the answer names the two
//! prefixed forms to write instead. Nothing is guessed.
//!
//! A SKU matches the SKU a device is encoded under, and no alias of it. Two
//! SKUs that a device file declares identical are still two models in a room,
//! and an operator who types one does not mean the other.

#[cfg(test)]
mod tests;

use std::fmt;

use crate::event::Device;
use crate::govee::Govee;
use crate::transport::DeviceId;

const ID: &str = "id";
const SKU: &str = "sku";
const NAME: &str = "name";

/// One target, read from what a person typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// One identity. It selects that device whether a scan found it or not:
    /// an identity addresses a device on its own.
    Id(DeviceId),
    /// Every known device encoded under this SKU.
    Sku(String),
    /// Every known device the configuration gives this name. The comparison
    /// ignores case and is exact: a name that matched a part of another one
    /// would widen a rig in silence.
    Name(String),
}

impl Selector {
    /// Read one target.
    ///
    /// The prefixes `id:`, `sku:` and `name:` state the kind. A bare target
    /// takes the kind of its shape, and the module documentation gives the
    /// shapes.
    ///
    /// The ambiguity between a SKU and a device named like one needs the
    /// devices to see, so [`Govee::select`] reports it and this does not.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] for a target with nothing in it, and
    /// [`Error::EmptyValue`] for a prefix with nothing after it.
    pub fn parse(target: &str) -> Result<Self, Error> {
        read(target).map(|(selector, _)| selector)
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
/// never ambiguous: the person already said which kind it is.
fn read(target: &str) -> Result<(Selector, bool), Error> {
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
                prefix: kind.to_owned(),
            });
        }
        let selector = match kind {
            ID => Selector::Id(DeviceId::new(value)),
            SKU => Selector::Sku(value.to_uppercase()),
            _ => Selector::Name(value.to_owned()),
        };
        return Ok((selector, true));
    }
    Ok((bare(target), false))
}

/// The kind a prefix names, where it names one. `AA:BB:…` reaches this with
/// `AA` and takes the shape path instead.
fn kind(prefix: &str) -> Option<&'static str> {
    match prefix.trim().to_ascii_lowercase().as_str() {
        ID => Some(ID),
        SKU => Some(SKU),
        NAME => Some(NAME),
        _ => None,
    }
}

/// The kind a bare target's shape gives it.
fn bare(target: &str) -> Selector {
    if is_identity(target) {
        return Selector::Id(DeviceId::new(target));
    }
    if is_sku(target) {
        return Selector::Sku(target.to_uppercase());
    }
    Selector::Name(target.to_owned())
}

/// Whether the text is shaped like an identity: hexadecimal groups that a
/// colon joins, which is what `lan` reports, or the handle a platform gives a
/// Bluetooth peripheral, which is a UUID and carries dashes.
///
/// The 12 digits are what keeps a hyphenated name out: a device called
/// `bed-a-b-c-d` has the groups and not the digits.
fn is_identity(target: &str) -> bool {
    let groups: Vec<&str> = target.split([':', '-']).collect();
    let hexadecimal = groups
        .iter()
        .all(|group| !group.is_empty() && group.chars().all(|c| c.is_ascii_hexdigit()));
    let digits: usize = groups.iter().map(|group| group.len()).sum();
    groups.len() >= 5 && digits >= 12 && hexadecimal
}

/// Whether the text is shaped like a SKU: `H` and four more characters that
/// are letters or digits, which is how every Govee model is written.
fn is_sku(target: &str) -> bool {
    let mut rest = target.chars();
    rest.next()
        .is_some_and(|first| first.eq_ignore_ascii_case(&'H'))
        && target.len() == 5
        && rest.all(|c| c.is_ascii_alphanumeric())
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
        /// The kind its shape gives it.
        kind: String,
    },

    /// A target that matches no device the SDK knows. A scan is what makes a
    /// device known; the cache answers for `lan` between runs.
    #[error("`{target}` matches no known device")]
    NoMatch {
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
            Self::Empty | Self::EmptyValue { .. } => "target_not_understood",
            Self::Ambiguous { .. } => "ambiguous_target",
            Self::NoMatch { .. } => "no_such_target",
        }
    }
}

impl Govee {
    /// The devices the targets name, in the order they were written.
    ///
    /// A device two targets name appears once, at the first place it was
    /// named. An identity selects itself, found or not. A SKU and a name
    /// select among the devices the SDK knows, so scan first where nothing
    /// has been discovered yet.
    ///
    /// # Errors
    ///
    /// [`Error::Ambiguous`] where a bare target reads as two kinds, and
    /// [`Error::NoMatch`] where a SKU or a name matches no known device.
    /// Every [`Error`] [`Selector::parse`] reports travels out of here too.
    pub fn select<I, T>(&self, targets: I) -> Result<Vec<DeviceId>, Error>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let known = self.devices();
        let mut chosen: Vec<DeviceId> = Vec::new();
        for target in targets {
            let target = target.as_ref();
            let (selector, prefixed) = read(target)?;
            if !prefixed {
                check_ambiguity(target, &selector, &known)?;
            }
            for id in matches(&selector, &known)? {
                if !chosen.contains(&id) {
                    chosen.push(id);
                }
            }
        }
        Ok(chosen)
    }
}

/// Refuse a bare identity or SKU that a known device carries as its name.
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

/// The devices one selector matches.
fn matches(selector: &Selector, known: &[Device]) -> Result<Vec<DeviceId>, Error> {
    let found: Vec<DeviceId> = match selector {
        Selector::Id(id) => return Ok(vec![id.clone()]),
        Selector::Sku(sku) => known
            .iter()
            .filter(|device| device.sku.eq_ignore_ascii_case(sku))
            .map(|device| device.id.clone())
            .collect(),
        Selector::Name(name) => named(known, name).collect(),
    };
    if found.is_empty() {
        return Err(Error::NoMatch {
            target: selector.to_string(),
        });
    }
    Ok(found)
}

/// Every known device the configuration gives this name.
fn named<'a>(known: &'a [Device], name: &'a str) -> impl Iterator<Item = DeviceId> + 'a {
    known
        .iter()
        .filter(move |device| {
            device
                .name
                .as_deref()
                .is_some_and(|given| given.eq_ignore_ascii_case(name))
        })
        .map(|device| device.id.clone())
}
