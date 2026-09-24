//! A target that the configuration alone resolves: an identity, a name the
//! configuration gives, or a group. It reads no scan, so it resolves before
//! any transport starts.

use super::{Error, GROUP, ID, Kind, NAME, SKU, Selector, ambiguous, is_identity, kind};
use crate::config::Config;
use crate::govee::Govee;
use crate::transport::DeviceId;

/// What one target names in the configuration.
enum Found {
    Device(DeviceId),
    Group {
        name: String,
        members: Vec<DeviceId>,
    },
    Model(String),
}

impl Selector {
    /// Read a target that names one device, against the names in `config`.
    ///
    /// `id:…` and `name:…` state the kind. A bare target is a name where the
    /// configuration gives one, and an identity otherwise.
    ///
    /// # Errors
    ///
    /// - [`Error::Empty`] and [`Error::EmptyValue`] as for [`Selector::parse`].
    /// - [`Error::NotOne`] for `sku:…` and for a group.
    /// - [`Error::NoMatch`] for a `name:…` the configuration does not give.
    /// - [`Error::Several`] for a name that two devices carry.
    /// - [`Error::Ambiguous`] for a bare target that reads as two kinds.
    pub fn one(target: &str, config: &Config) -> Result<DeviceId, Error> {
        match find(target, config)? {
            Found::Device(id) => Ok(id),
            Found::Group { name, .. } => Err(Error::NotOne {
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
            Found::Group { members, .. } => Ok(members),
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

fn find(target: &str, config: &Config) -> Result<Found, Error> {
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
            Kind::Id => Ok(Found::Device(DeviceId::new(value))),
            Kind::Sku => Ok(Found::Model(value.to_uppercase())),
            Kind::Name => named(value, config)?
                .map(Found::Device)
                .ok_or_else(|| Error::NoMatch {
                    target: format!("{NAME}:{value}"),
                }),
            Kind::Group => group(value, config).ok_or_else(|| Error::NoMatch {
                target: format!("{GROUP}:{value}"),
            }),
        };
    }
    let identity = is_identity(target);
    match (named(target, config)?, group(target, config)) {
        (Some(_), Some(_)) => Err(ambiguous(target, NAME, GROUP)),
        (Some(_), None) if identity => Err(ambiguous(target, ID, NAME)),
        (None, Some(_)) if identity => Err(ambiguous(target, ID, GROUP)),
        (Some(id), None) => Ok(Found::Device(id)),
        (None, Some(found)) => Ok(found),
        (None, None) => Ok(Found::Device(DeviceId::new(target))),
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

fn group(name: &str, config: &Config) -> Option<Found> {
    let members = config.members(name);
    (!members.is_empty()).then(|| Found::Group {
        name: name.to_owned(),
        members,
    })
}
