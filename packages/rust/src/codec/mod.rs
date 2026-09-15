//! Device catalog and protocol codec.
//!
//! The single place where protocol logic lives, and it does no I/O:
//! `devices/*.yaml` plus arguments in, the exact bytes a transport sends out.
//! `tools/check-no-io.sh` fails the build if anything network-shaped is
//! imported here.
//!
//! Two rules shape the API. No SKU and no command name appears in this code —
//! a device file describes its own commands, so adding a device is adding
//! YAML. And nothing is approximated: an argument outside its declared range,
//! or a command a mode does not carry, is a typed error. The firmware clamps
//! in silence; this crate does not.
//!
//! ```
//! use govee_toolkit::codec::{self, Args, Catalog, Mode};
//!
//! let catalog = Catalog::embedded()?;
//! let device = catalog.device("H61A0")?;
//! let encoded = codec::encode(
//!     device,
//!     Mode::Lan,
//!     "brightness",
//!     &Args::new().int("level", 50),
//! )?;
//!
//! assert_eq!(encoded.cmd, "brightness");
//! # Ok::<_, codec::Error>(())
//! ```

pub mod args;
pub mod capabilities;
pub mod catalog;
pub mod chunk;
pub mod cloud;
pub mod coerce;
pub mod command;
pub mod error;
pub mod exchange;
pub mod frame;
pub mod measurements;
pub mod reply;
pub mod validate;
pub mod white;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub use args::{ArgValue, Args};
pub use capabilities::{Capabilities, CapabilityParams, ModeCapabilities, PAIR_PARAMS, Reason};
pub use catalog::{
    ArgRole, ArgSpec, Bounds, CapabilityRef, Command, Device, Family, Mode, ModeSupport, Modes,
    Role, Support, UnknownMode,
};
pub use chunk::Chunk;
pub use coerce::Supplied;
pub use command::{Encoded, encode};
pub use error::{Error, Result};
pub use exchange::{Exchange, Exchanges, Step};
pub use frame::Frame;
pub use measurements::{Ble as BleMeasurements, FrameRate, FrameRates, Measurements};
pub use reply::Captured;

include!(concat!(env!("OUT_DIR"), "/devices.rs"));

/// The device-file schema revision this build implements.
///
/// A file declaring anything else is refused rather than read as this one: a
/// later revision can change what an existing field means. See
/// `devices/schema.yaml` and `docs/versioning.md`.
pub const SCHEMA_VERSION: u32 = 1;

/// A device file that replaced one already in the catalog.
///
/// An override shadows what the build shipped, so it must be visible. Log
/// every one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overridden {
    /// The SKU that was replaced.
    pub sku: String,
    /// The file that shipped with the build.
    pub was: String,
    /// The file that replaced it.
    pub now: String,
}

/// Every known device.
///
/// A clone shares the entries rather than copying them, so a binding can hand
/// one out per call. [`Catalog::overlay`] copies once, and a clone taken
/// before it keeps the entries it was built with.
#[derive(Debug, Clone)]
pub struct Catalog(Arc<Inner>);

#[derive(Debug, Clone, Default)]
struct Inner {
    devices: Vec<Device>,
    /// Uppercased SKU or verified alias, to an index into `devices`.
    index: BTreeMap<String, usize>,
    /// Where each device came from, parallel to `devices`.
    origin: Vec<String>,
    /// The shared command tables an `include:` resolves against, by name. An
    /// overlay resolves against these too.
    families: BTreeMap<String, Family>,
}

impl Catalog {
    /// The catalog compiled into this build.
    ///
    /// Parsing is cheap but not free — build one and keep it.
    ///
    /// # Errors
    ///
    /// [`Error::DeviceFile`] if an embedded file does not parse, or
    /// [`Error::DuplicateSku`] if two of them claim the same SKU.
    pub fn embedded() -> Result<Self> {
        Self::from_sources_with(EMBEDDED.iter().copied(), EMBEDDED_FAMILIES.iter().copied())
    }

    /// Build a catalog from `(file name, YAML)` pairs.
    ///
    /// # Errors
    ///
    /// See [`Catalog::embedded`].
    pub fn from_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Result<Self> {
        Self::from_sources_with(sources, [])
    }

    /// Build a catalog from device files and the shared tables they include.
    ///
    /// Both are `(file name, YAML)` pairs. A device file naming a fragment
    /// that is not here fails to load, rather than losing the commands it
    /// expected to gain.
    ///
    /// # Errors
    ///
    /// See [`Catalog::embedded`], plus [`Error::UnknownFamily`] if an
    /// `include:` names no fragment and [`Error::DuplicateCommand`] if a
    /// fragment and the file both declare one command.
    pub fn from_sources_with<'a, 'b>(
        sources: impl IntoIterator<Item = (&'a str, &'a str)>,
        families: impl IntoIterator<Item = (&'b str, &'b str)>,
    ) -> Result<Self> {
        let mut inner = Inner::default();
        for (file, yaml) in families {
            let family = parse_family(file, yaml)?;
            inner.families.insert(family.family.clone(), family);
        }
        for (file, yaml) in sources {
            let device = inner.parse_device(file, yaml)?;
            inner.push(device, file)?;
        }
        Ok(Self(Arc::new(inner)))
    }

    /// Replace catalog entries with locally supplied files.
    ///
    /// A file here replaces the one the build shipped for that SKU, wholesale.
    /// Not the default: a new SKU normally arrives with a release, so that one
    /// person's device does not define the model for everyone.
    ///
    /// Returns what was replaced. Two files in one overlay claiming the same
    /// SKU is a mistake, not an override, and stays an error.
    ///
    /// # Errors
    ///
    /// [`Error::DeviceFile`] if a file does not parse, or
    /// [`Error::DuplicateSku`] if the overlay is self-contradictory or an alias
    /// it declares belongs to a device it does not replace.
    pub fn overlay<'a>(
        &mut self,
        sources: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Vec<Overridden>> {
        let mut replaced = Vec::new();
        let mut claimed: BTreeMap<String, String> = BTreeMap::new();
        let inner = Arc::make_mut(&mut self.0);

        for (file, yaml) in sources {
            let device = inner.parse_device(file, yaml)?;
            let key = device.sku.to_uppercase();
            if let Some(first) = claimed.insert(key.clone(), file.to_owned()) {
                return Err(Error::DuplicateSku {
                    sku: key,
                    first,
                    second: file.to_owned(),
                });
            }

            if let Some(position) = inner.index.get(&key).copied() {
                // Drop every key the old entry answered to, including aliases
                // the replacement does not declare.
                inner.index.retain(|_, i| *i != position);
                let was = inner.origin.get(position).cloned().unwrap_or_default();
                inner.claim_keys(&device, position, file)?;
                if let Some(slot) = inner.devices.get_mut(position) {
                    *slot = device;
                }
                if let Some(slot) = inner.origin.get_mut(position) {
                    file.clone_into(slot);
                }
                replaced.push(Overridden {
                    sku: key,
                    was,
                    now: file.to_owned(),
                });
            } else {
                inner.push(device, file)?;
            }
        }

        Ok(replaced)
    }

    /// Look up a device by SKU or by verified alias. Case-insensitive.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownSku`] if nothing declares it.
    pub fn device(&self, sku: &str) -> Result<&Device> {
        self.0.device(sku)
    }

    /// Every device file in the catalog.
    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.0.devices.iter()
    }

    /// Every SKU that resolves, aliases included.
    pub fn skus(&self) -> impl Iterator<Item = &str> {
        self.0.index.keys().map(String::as_str)
    }
}

impl Inner {
    /// Add a device at the end, under every key it answers to.
    fn push(&mut self, device: Device, file: &str) -> Result<()> {
        let position = self.devices.len();
        self.claim_keys(&device, position, file)?;
        self.devices.push(device);
        self.origin.push(file.to_owned());
        Ok(())
    }

    /// Parse a device file, merge in every table it includes, then apply the
    /// patches the file declares against them.
    fn parse_device(&self, file: &str, yaml: &str) -> Result<Device> {
        let mut device = parse(file, yaml)?;
        // Read before the merge, so an `overrides:` entry that names a local
        // command is refused rather than applied.
        let local = Mode::ALL.map(|mode| {
            device
                .commands
                .get(mode)
                .keys()
                .cloned()
                .collect::<BTreeSet<String>>()
        });
        for name in device.include.clone() {
            let family = self
                .families
                .get(&name)
                .ok_or_else(|| Error::UnknownFamily {
                    file: file.to_owned(),
                    family: name.clone(),
                })?;
            for mode in Mode::ALL {
                for (command, spec) in family.commands.get(mode) {
                    let table = device.commands.get_mut(mode);
                    // A silent override would let a fragment decide what bytes
                    // reach a device that meant to declare its own.
                    if table.contains_key(command) {
                        return Err(Error::DuplicateCommand {
                            file: file.to_owned(),
                            family: name.clone(),
                            mode,
                            command: command.clone(),
                        });
                    }
                    table.insert(command.clone(), spec.clone());
                }
            }
        }
        let overrides = std::mem::take(&mut device.overrides);
        for (mode, local) in Mode::ALL.into_iter().zip(&local) {
            catalog::apply_overrides(
                file,
                mode,
                device.commands.get_mut(mode),
                local,
                overrides.get(mode),
            )?;
        }
        // After the merge and the patches: a shared table resolves against the
        // capabilities of the device that included it, and an override can
        // carry a reference of its own.
        catalog::resolve_bounds(file, &mut device)?;
        Ok(device)
    }

    /// Point every key a device answers to at `position`.
    ///
    /// `aliases` are SKUs verified to behave identically, so they resolve.
    /// `candidate_aliases` deliberately do not: an unverified lookalike must
    /// read as an unknown SKU, not as a supported device.
    fn claim_keys(&mut self, device: &Device, position: usize, file: &str) -> Result<()> {
        let keys = std::iter::once(device.sku.clone()).chain(device.aliases.iter().cloned());
        for key in keys {
            let key = key.to_uppercase();
            if let Some(previous) = self.index.get(&key)
                && *previous != position
            {
                return Err(Error::DuplicateSku {
                    sku: key,
                    first: self.origin.get(*previous).cloned().unwrap_or_default(),
                    second: file.to_owned(),
                });
            }
            self.index.insert(key, position);
        }
        Ok(())
    }

    fn device(&self, sku: &str) -> Result<&Device> {
        // The index is keyed uppercase, so a caller that already holds an
        // uppercase SKU costs no allocation on the send path.
        self.index
            .get(sku)
            .or_else(|| self.index.get(&sku.to_uppercase()))
            .and_then(|i| self.devices.get(*i))
            .ok_or_else(|| Error::UnknownSku {
                sku: sku.to_owned(),
            })
    }
}

fn parse(file: &str, yaml: &str) -> Result<Device> {
    let device: Device = serde_norway::from_str(yaml).map_err(|e| Error::DeviceFile {
        file: file.to_owned(),
        source: Box::new(e),
    })?;
    check_version(file, device.schema_version)?;
    Ok(device)
}

fn parse_family(file: &str, yaml: &str) -> Result<Family> {
    let family: Family = serde_norway::from_str(yaml).map_err(|e| Error::DeviceFile {
        file: file.to_owned(),
        source: Box::new(e),
    })?;
    check_version(file, family.schema_version)?;
    Ok(family)
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

/// A frame as lowercase hex, for a test that compares against a capture.
#[cfg(test)]
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}
