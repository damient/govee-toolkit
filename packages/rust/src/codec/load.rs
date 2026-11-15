//! Parsing of the three kinds of catalog file: a device file, a shared command
//! table, and a listing.

use serde::{Deserialize, Serialize};

use super::catalog::{Device, Family};
use super::error::{Error, Result};
use super::{Catalog, Inner, SCHEMA_VERSION};

/// One `devices/listed/<name>.yaml` file: models that nobody verified, and the
/// one definition that each of them resolves to.
#[derive(Debug, Clone, Serialize)]
pub struct Listing {
    /// The name every model of the list carries in [`Device::listing`].
    pub listing: String,
    /// Where the list of models comes from, as a repository path.
    pub source: String,
    /// The models, in the order of the file.
    pub models: Vec<ListedModel>,
    /// The shared definition, with an empty `sku` and `name`.
    pub device: Device,
}

/// One model of a [`Listing`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ListedModel {
    /// The SKU.
    pub sku: String,
    /// The model name that the source gives.
    pub name: String,
}

/// The file as written: a [`Listing`] plus the body of a device file, with no
/// `sku` and no `name`.
#[derive(Deserialize)]
struct ListingFile {
    schema_version: u32,
    listing: String,
    source: String,
    models: Vec<ListedModel>,
    device: serde_norway::Mapping,
}

impl Catalog {
    /// Every model of a listing that resolves, as its [`Device`].
    pub fn listed(&self) -> impl Iterator<Item = &Device> {
        self.0.devices.iter().filter(|device| {
            device.listing.is_some()
                && self
                    .device(&device.sku)
                    .is_ok_and(|found| std::ptr::eq(found, *device))
        })
    }

    /// The listings the catalog was built with, every model included.
    #[must_use]
    pub fn listings(&self) -> &[Listing] {
        &self.0.listings
    }
}

impl Inner {
    /// Parse a listing and give each model its own [`Device`], built from the
    /// shared definition. A SKU that a device file claims stays with the file.
    pub(super) fn parse_listing(&self, file: &str, yaml: &str) -> Result<(Listing, Vec<Device>)> {
        let parsed: ListingFile = serde_norway::from_str(yaml).map_err(|e| device_file(file, e))?;
        check_version(file, parsed.schema_version)?;

        let mut body = parsed.device;
        body.insert("schema_version".into(), SCHEMA_VERSION.into());
        body.insert("sku".into(), "".into());
        body.insert("name".into(), "".into());
        let body = serde_norway::to_string(&body).map_err(|e| device_file(file, e))?;
        let template = self.parse_device(file, &body)?;

        let claimed = |sku: &str| {
            self.index
                .get(&sku.to_uppercase())
                .and_then(|at| self.devices.get(*at))
                .is_some_and(|device| device.listing.is_none())
        };
        let devices = parsed
            .models
            .iter()
            .filter(|model| !claimed(&model.sku))
            .map(|model| {
                let mut device = template.clone();
                device.sku.clone_from(&model.sku);
                device.name.clone_from(&model.name);
                device.listing = Some(parsed.listing.clone());
                device
            })
            .collect();
        let listing = Listing {
            listing: parsed.listing,
            source: parsed.source,
            models: parsed.models,
            device: template,
        };
        Ok((listing, devices))
    }
}

pub(super) fn parse(file: &str, yaml: &str) -> Result<Device> {
    let device: Device = serde_norway::from_str(yaml).map_err(|e| device_file(file, e))?;
    check_version(file, device.schema_version)?;
    Ok(device)
}

pub(super) fn parse_family(file: &str, yaml: &str) -> Result<Family> {
    let family: Family = serde_norway::from_str(yaml).map_err(|e| device_file(file, e))?;
    check_version(file, family.schema_version)?;
    Ok(family)
}

fn device_file(file: &str, source: serde_norway::Error) -> Error {
    Error::DeviceFile {
        file: file.to_owned(),
        source: Box::new(source),
    }
}

fn check_version(file: &str, found: u32) -> Result<()> {
    if found == SCHEMA_VERSION {
        return Ok(());
    }
    Err(Error::SchemaVersion {
        file: file.to_owned(),
        found,
        supported: SCHEMA_VERSION,
    })
}
