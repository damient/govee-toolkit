//! A target that names one device, which the configuration alone resolves.

use super::{Error, GROUP, NAME, Names, SKU, Selector, resolve};
use crate::codec::Catalog;
use crate::config::Config;
use crate::transport::DeviceId;

enum Found {
    Device(DeviceId),
    Group(String),
    Model(String),
}

impl Selector {
    /// Read a target that names one device, against the SKUs in `catalog`
    /// and the names in `config`.
    ///
    /// # Errors
    ///
    /// - [`Error::Empty`] and [`Error::EmptyValue`] as for [`Selector::parse`].
    /// - [`Error::NotOne`] for `sku:…` and for a group.
    /// - [`Error::NoMatch`] for a name that the configuration does not give.
    /// - [`Error::Several`] for a name that two devices carry.
    /// - [`Error::Ambiguous`] for a bare target that reads as two kinds.
    pub(crate) fn one(target: &str, catalog: &Catalog, config: &Config) -> Result<DeviceId, Error> {
        match find(target, catalog, config)? {
            Found::Device(id) => Ok(id),
            Found::Group(name) => Err(Error::NotOne {
                target: format!("{GROUP}:{name}"),
            }),
            Found::Model(sku) => Err(Error::NotOne {
                target: format!("{SKU}:{sku}"),
            }),
        }
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

fn find(target: &str, catalog: &Catalog, config: &Config) -> Result<Found, Error> {
    let selector = resolve(target, Some(catalog), config)?;
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
