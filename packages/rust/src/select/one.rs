//! A target for a command that drives one device: an identity, or a name the
//! configuration gives. It reads the configuration and no scan, so it resolves
//! before any transport starts.

use super::{Error, ID, Kind, NAME, SKU, Selector, is_identity, kind};
use crate::config::Config;
use crate::transport::DeviceId;

impl Selector {
    /// Read a target that names one device, against the names in `config`.
    ///
    /// `id:…` and `name:…` state the kind. A bare target is a name where the
    /// configuration gives one, and an identity otherwise.
    ///
    /// # Errors
    ///
    /// - [`Error::Empty`] and [`Error::EmptyValue`] as for [`Selector::parse`].
    /// - [`Error::NotOne`] for `sku:…`, which names a model.
    /// - [`Error::NoMatch`] for a `name:…` the configuration does not give.
    /// - [`Error::Several`] for a name that two devices carry.
    /// - [`Error::Ambiguous`] for a bare identity that is also a name.
    pub fn one(target: &str, config: &Config) -> Result<DeviceId, Error> {
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
            return match kind {
                Kind::Id => Ok(DeviceId::new(value)),
                Kind::Sku => Err(Error::NotOne {
                    target: format!("{SKU}:{value}"),
                }),
                Kind::Name => named(value, config)?.ok_or_else(|| Error::NoMatch {
                    target: format!("{NAME}:{value}"),
                }),
            };
        }
        match named(target, config)? {
            Some(_) if is_identity(target) => Err(Error::Ambiguous {
                target: target.to_owned(),
                kind: ID.to_owned(),
            }),
            Some(id) => Ok(id),
            None => Ok(DeviceId::new(target)),
        }
    }
}

fn named(name: &str, config: &Config) -> Result<Option<DeviceId>, Error> {
    let mut found = config.devices.iter().filter(|(_, device)| {
        device
            .name
            .as_deref()
            .is_some_and(|given| given.eq_ignore_ascii_case(name))
    });
    let first = found.next().map(|(id, _)| id.clone());
    if found.next().is_some() {
        return Err(Error::Several {
            target: format!("{NAME}:{name}"),
        });
    }
    Ok(first)
}
