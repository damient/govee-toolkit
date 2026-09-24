//! A target that the configuration alone resolves, before any transport
//! starts.

use super::{Error, GROUP, NAME, Names, SKU, Selector, resolve};
use crate::config::Config;
use crate::govee::Govee;
use crate::transport::DeviceId;

enum Found {
    Device(DeviceId),
    Group(String),
    Model(String),
}

impl Selector {
    /// Read a target that names one device, against the names in `config`.
    ///
    /// # Errors
    ///
    /// - [`Error::Empty`] and [`Error::EmptyValue`] as for [`Selector::parse`].
    /// - [`Error::NotOne`] for `sku:…` and for a group.
    /// - [`Error::NoMatch`] for a name that the configuration does not give.
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
    /// The identity of the one device that `target` names, from the
    /// configuration and with no scan. Pass it to [`Govee::device`].
    ///
    /// # Errors
    ///
    /// Every [`Error`] that [`Selector::one`] reports.
    pub fn target(&self, target: &str) -> Result<DeviceId, Error> {
        Selector::one(target, self.config())
    }

    /// The identities that `target` names, one device or every member of a
    /// group, from the configuration and with no scan. Pass them to
    /// [`Govee::group`].
    ///
    /// # Errors
    ///
    /// Every [`Error`] that [`Selector::many`] reports.
    pub fn targets(&self, target: &str) -> Result<Vec<DeviceId>, Error> {
        Selector::many(target, self.config())
    }
}

impl Names for Config {
    fn names(&self, name: &str) -> bool {
        self.named(name).next().is_some()
    }

    fn groups(&self, group: &str) -> bool {
        self.is_group(group)
    }
}

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
