//! A target that the configuration alone resolves: an identity, a name the
//! configuration gives, or a group. It reads no scan, so it resolves before
//! any transport starts.

use super::{Error, GROUP, NAME, Names, SKU, Selector, resolve};
use crate::config::Config;
use crate::govee::Govee;
use crate::transport::DeviceId;

/// What one target names in the configuration.
enum Found {
    Device(DeviceId),
    Group(String),
    Model(String),
}

impl Selector {
    /// Read a target that names one device, against the names in `config`.
    ///
    /// `id:…` and `name:…` state the kind. A bare target is a name where the
    /// configuration gives one, and an identity where it reads as one.
    ///
    /// # Errors
    ///
    /// - [`Error::Empty`] and [`Error::EmptyValue`] as for [`Selector::parse`].
    /// - [`Error::NotOne`] for `sku:…` and for a group.
    /// - [`Error::NoMatch`] for a `name:…` the configuration does not give, and
    ///   for a bare target that is no name and does not read as an identity.
    /// - [`Error::Several`] for a name that two devices carry.
    /// - [`Error::Ambiguous`] for a bare target that reads as two kinds.
    pub fn one(target: &str, config: &Config) -> Result<DeviceId, Error> {
        match find(target, config)? {
            Found::Device(id) => Ok(id),
            Found::Group(name) => Err(Error::NotOne {
                target: format!("{GROUP}:{name}"),
            }),
            Found::Model(sku) => Err(Error::NotOne {
                target: format!("{SKU}:{sku}"),
            }),
        }
    }

    /// Read a target that names one device or one group, against `config`.
    /// A group answers its members in identity order.
    ///
    /// `group:…` states the kind. A bare target is a group where no device
    /// carries it as a name and a device carries it under `groups:`.
    ///
    /// # Errors
    ///
    /// As for [`Selector::one`], except for a group, plus [`Error::Model`]
    /// for `sku:…` and [`Error::NoMatch`] for a `group:…` that no device
    /// carries.
    pub fn many(target: &str, config: &Config) -> Result<Vec<DeviceId>, Error> {
        match find(target, config)? {
            Found::Device(id) => Ok(vec![id]),
            Found::Group(name) => Ok(config.members(&name)),
            Found::Model(sku) => Err(Error::Model {
                target: format!("{SKU}:{sku}"),
            }),
        }
    }
}

impl Govee {
    /// The identity of the one device that `target` names, against the
    /// configuration in force. Pass the result to [`Govee::device`].
    ///
    /// It reads the configuration and no scan, so it costs nothing on the send
    /// path.
    ///
    /// # Errors
    ///
    /// Every [`Error`] that [`Selector::one`] reports.
    pub fn target(&self, target: &str) -> Result<DeviceId, Error> {
        Selector::one(target, self.config())
    }

    /// The identities that `target` names, one device or every member of a
    /// group, against the configuration in force. Pass the result to
    /// [`Govee::group`].
    ///
    /// It reads the configuration and no scan.
    ///
    /// # Errors
    ///
    /// Every [`Error`] that [`Selector::many`] reports.
    pub fn targets(&self, target: &str) -> Result<Vec<DeviceId>, Error> {
        Selector::many(target, self.config())
    }
}

/// The devices the configuration names, found by a scan or not.
impl Names for Config {
    fn names(&self, name: &str) -> bool {
        self.named(name).next().is_some()
    }

    fn groups(&self, group: &str) -> bool {
        self.is_group(group)
    }
}

/// A bare target never reads as a SKU here: which SKUs exist is the catalog's
/// to say, and this reads the configuration alone.
fn find(target: &str, config: &Config) -> Result<Found, Error> {
    let selector = resolve(target, None, config)?;
    let found = match &selector {
        Selector::Id(id) => Some(Found::Device(id.clone())),
        Selector::Sku(sku) => Some(Found::Model(sku.clone())),
        Selector::Name(name) => named(name, config)?.map(Found::Device),
        Selector::Group(group) => config.is_group(group).then(|| Found::Group(group.clone())),
    };
    found.ok_or_else(|| Error::NoMatch {
        target: selector.to_string(),
    })
}

fn named(name: &str, config: &Config) -> Result<Option<DeviceId>, Error> {
    let mut found = config.named(name);
    let first = found.next().cloned();
    if found.next().is_some() {
        return Err(Error::Several {
            target: format!("{NAME}:{name}"),
        });
    }
    Ok(first)
}
