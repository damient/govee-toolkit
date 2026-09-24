//! Naming devices on a command line: by identity, by SKU, by name or by group.
//!
//! One target names one kind of thing. A prefix states the kind, and a bare
//! target takes the kind it reads as:
//!
//! | Target | Kind |
//! | ------ | ---- |
//! | `1C:8B:C4:A2:C0:46:64:6E`, `id:…` | the identity a device reports |
//! | `H6159`, `sku:H6159` | every known device of that model |
//! | `kitchen`, `name:kitchen` | the device the configuration names |
//! | `ambient`, `group:ambient` | every device the configuration puts in that group |
//!
//! A bare target reads as a SKU where a device file is encoded under it, and
//! never from the shape of the text alone. Any other bare target is a name,
//! or a group where no device carries that name. A bare target that reads as
//! two kinds is refused. Nothing is guessed.
//!
//! A SKU matches the SKU a device is encoded under, and no alias of it: an
//! operator who types one model does not mean the other.

mod error;
mod one;
#[cfg(test)]
mod tests;

use std::fmt;

pub use self::error::Error;
use crate::codec::{Catalog, Mode};
use crate::config::{carries, gives};
use crate::event::Device;
use crate::govee::Govee;
use crate::transport::DeviceId;

const ID: &str = "id";
const SKU: &str = "sku";
const NAME: &str = "name";
const GROUP: &str = "group";

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
    /// Every known device in this group, whole and ignoring case.
    Group(String),
}

impl Selector {
    /// Read one target, against the catalog that says which SKUs exist.
    ///
    /// It does not report the ambiguity between a SKU and a device named like
    /// one: that needs the devices to see, so [`Selector::resolve`] and
    /// [`Govee::select`] report it.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] for a target with nothing in it, and
    /// [`Error::EmptyValue`] for a prefix with nothing after it.
    pub fn parse(target: &str, catalog: &Catalog) -> Result<Self, Error> {
        read(target, Some(catalog)).map(|(selector, _)| selector)
    }

    /// Read one target, and settle a bare one against the names and the
    /// groups that `source` gives.
    ///
    /// # Errors
    ///
    /// Every [`Error`] that [`Selector::parse`] reports, and
    /// [`Error::Ambiguous`] for a bare target that reads as two kinds.
    pub fn resolve<N>(target: &str, catalog: &Catalog, source: &N) -> Result<Self, Error>
    where
        N: Names + ?Sized,
    {
        resolve(target, Some(catalog), source)
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Id(_) => ID,
            Self::Sku(_) => SKU,
            Self::Name(_) => NAME,
            Self::Group(_) => GROUP,
        }
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{ID}:{id}"),
            Self::Sku(sku) => write!(f, "{SKU}:{sku}"),
            Self::Name(name) => write!(f, "{NAME}:{name}"),
            Self::Group(group) => write!(f, "{GROUP}:{group}"),
        }
    }
}

/// The names and the groups that a bare target settles against.
pub trait Names {
    /// Whether a device carries `name`, whole and ignoring case.
    fn names(&self, name: &str) -> bool;

    /// Whether a device carries `group`, whole and ignoring case.
    fn groups(&self, group: &str) -> bool;
}

impl Names for [Device] {
    fn names(&self, name: &str) -> bool {
        named(self, name).next().is_some()
    }

    fn groups(&self, group: &str) -> bool {
        grouped(self, group).next().is_some()
    }
}

/// One target, and the text of a bare one. Without a catalog, a bare target
/// never reads as a SKU.
fn read<'a>(
    target: &'a str,
    catalog: Option<&Catalog>,
) -> Result<(Selector, Option<&'a str>), Error> {
    Ok(match split(target)? {
        Written::Prefixed(kind, value) => (kind.read(value), None),
        Written::Bare(target) => (bare(target, catalog), Some(target)),
    })
}

fn resolve<N>(target: &str, catalog: Option<&Catalog>, source: &N) -> Result<Selector, Error>
where
    N: Names + ?Sized,
{
    match read(target, catalog)? {
        (selector, Some(written)) => settle(written, selector, source),
        (selector, None) => Ok(selector),
    }
}

enum Written<'a> {
    Prefixed(Kind, &'a str),
    Bare(&'a str),
}

fn split(target: &str) -> Result<Written<'_>, Error> {
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
        return Ok(Written::Prefixed(kind, value));
    }
    Ok(Written::Bare(target))
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Id,
    Sku,
    Name,
    Group,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Id => ID,
            Self::Sku => SKU,
            Self::Name => NAME,
            Self::Group => GROUP,
        }
    }

    fn read(self, value: &str) -> Selector {
        match self {
            Self::Id => Selector::Id(DeviceId::new(value)),
            Self::Sku => Selector::Sku(value.to_uppercase()),
            Self::Name => Selector::Name(value.to_owned()),
            Self::Group => Selector::Group(value.to_owned()),
        }
    }
}

/// `AA:BB:…` reaches this with `AA` and takes the shape path instead.
fn kind(prefix: &str) -> Option<Kind> {
    match prefix.trim().to_ascii_lowercase().as_str() {
        ID => Some(Kind::Id),
        SKU => Some(Kind::Sku),
        NAME => Some(Kind::Name),
        GROUP => Some(Kind::Group),
        _ => None,
    }
}

fn bare(target: &str, catalog: Option<&Catalog>) -> Selector {
    if is_identity(target) {
        return Selector::Id(DeviceId::new(target));
    }
    if catalog.is_some_and(|catalog| is_sku(target, catalog)) {
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

impl Govee {
    /// The devices the targets name, in the order they were written.
    ///
    /// `mode` is the one mode the caller will drive, and `None` where it
    /// drives none and lists instead. A SKU, a name and a group then match
    /// among the devices that enable `mode`. An identity selects itself, found
    /// or not.
    ///
    /// A device two targets name appears once, at the first place it was
    /// named. A SKU, a name and a group select among the devices the SDK
    /// knows, so scan first where nothing has been discovered yet.
    ///
    /// # Errors
    ///
    /// [`Error::Ambiguous`] where a bare target reads as two kinds,
    /// [`Error::NoMatch`] where a SKU, a name or a group matches no known
    /// device, and
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
            let selector = resolve(target.as_ref(), Some(self.catalog()), known.as_slice())?;
            for id in matches(&selector, &known, mode)? {
                if !chosen.contains(&id) {
                    chosen.push(id);
                }
            }
        }
        Ok(chosen)
    }
}

fn settle<N>(written: &str, selector: Selector, source: &N) -> Result<Selector, Error>
where
    N: Names + ?Sized,
{
    let named = source.names(written);
    let grouped = source.groups(written);
    let other = match (&selector, named, grouped) {
        (Selector::Name(name), false, true) => return Ok(Selector::Group(name.clone())),
        (Selector::Id(_) | Selector::Sku(_), true, _) => NAME,
        (Selector::Name(_) | Selector::Id(_) | Selector::Sku(_), _, true) => GROUP,
        _ => return Ok(selector),
    };
    Err(Error::ambiguous(written, selector.kind(), other))
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
        Selector::Group(group) => grouped(known, group).collect(),
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
    known
        .iter()
        .filter(move |device| gives(device.name.as_deref(), name))
}

fn grouped<'a>(known: &'a [Device], group: &'a str) -> impl Iterator<Item = &'a Device> + 'a {
    known
        .iter()
        .filter(move |device| carries(&device.groups, group))
}
