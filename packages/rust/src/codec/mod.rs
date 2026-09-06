//! Device catalog and protocol codec.
//!
//! The single place where protocol logic lives. It does **no I/O**: it turns
//! `devices/*.yaml` plus arguments into the exact bytes a transport sends, and
//! nothing more. Transports, mode selection and the circuit breaker live in the
//! modules above it, and `tools/check-no-io.sh` fails the build if anything
//! network-shaped is imported here.
//!
//! Two rules shape the API:
//!
//! - **No SKU, no command name appears in this code.** A device file describes
//!   its own commands; the codec interprets them. Adding a device is adding
//!   YAML.
//! - **Nothing is approximated.** An argument outside its declared range, a
//!   command a mode does not carry, an unsupported mode — each is a typed
//!   error. The firmware clamps in silence; this crate does not.
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
pub mod command;
pub mod error;
pub mod exchange;
pub mod frame;
pub mod measurements;
pub mod reply;
pub mod validate;

use std::collections::BTreeMap;

pub use args::{ArgValue, Args};
pub use capabilities::{Capabilities, CapabilityParams, ModeCapabilities, Reason};
pub use catalog::{ArgRole, ArgSpec, Command, Device, Mode, ModeSupport, Modes, Role, Support};
pub use chunk::Chunk;
pub use command::{Encoded, encode};
pub use error::{Error, Result};
pub use exchange::{Exchange, Exchanges, Step};
pub use frame::Frame;
pub use measurements::{FrameRate, FrameRates, Measurements};
pub use reply::Captured;

include!(concat!(env!("OUT_DIR"), "/devices.rs"));

/// The device-file schema revision this build implements.
///
/// A file declaring anything else is refused rather than read as this one: the
/// fields a later revision adds change what the fields it already had mean, and
/// guessing which is which is how a device gets sent bytes nobody verified. See
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
#[derive(Debug, Clone)]
pub struct Catalog {
    devices: Vec<Device>,
    /// Uppercased SKU or verified alias, to an index into `devices`.
    index: BTreeMap<String, usize>,
    /// Where each device came from, parallel to `devices`.
    origin: Vec<String>,
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
        Self::from_sources(EMBEDDED.iter().copied())
    }

    /// Build a catalog from `(file name, YAML)` pairs.
    ///
    /// # Errors
    ///
    /// See [`Catalog::embedded`].
    pub fn from_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Result<Self> {
        let mut catalog = Self {
            devices: Vec::new(),
            index: BTreeMap::new(),
            origin: Vec::new(),
        };
        for (file, yaml) in sources {
            let device = parse(file, yaml)?;
            let position = catalog.devices.len();
            catalog.claim_keys(&device, position, file)?;
            catalog.devices.push(device);
            catalog.origin.push(file.to_owned());
        }
        Ok(catalog)
    }

    /// Replace catalog entries with locally supplied files.
    ///
    /// A file here replaces the one the build shipped for that SKU, wholesale.
    /// This is not the default: a new SKU normally arrives with a release, so
    /// that one person's device does not silently define the model for
    /// everyone.
    ///
    /// Returns what was replaced, so a caller can report it. Two files in one
    /// overlay that claim the same SKU is still an error: that is a mistake,
    /// not an override.
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

        for (file, yaml) in sources {
            let device = parse(file, yaml)?;
            let key = device.sku.to_uppercase();
            if let Some(first) = claimed.insert(key.clone(), file.to_owned()) {
                return Err(Error::DuplicateSku {
                    sku: key,
                    first,
                    second: file.to_owned(),
                });
            }

            if let Some(position) = self.index.get(&key).copied() {
                // Drop every key the old entry answered to, including aliases
                // the replacement does not declare.
                self.index.retain(|_, i| *i != position);
                let was = self.origin.get(position).cloned().unwrap_or_default();
                self.claim_keys(&device, position, file)?;
                if let Some(slot) = self.devices.get_mut(position) {
                    *slot = device;
                }
                if let Some(slot) = self.origin.get_mut(position) {
                    file.clone_into(slot);
                }
                replaced.push(Overridden {
                    sku: key,
                    was,
                    now: file.to_owned(),
                });
            } else {
                let position = self.devices.len();
                self.claim_keys(&device, position, file)?;
                self.devices.push(device);
                self.origin.push(file.to_owned());
            }
        }

        Ok(replaced)
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

    /// Look up a device by SKU or by verified alias. Case-insensitive.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownSku`] if nothing declares it.
    pub fn device(&self, sku: &str) -> Result<&Device> {
        self.index
            .get(&sku.to_uppercase())
            .and_then(|i| self.devices.get(*i))
            .ok_or_else(|| Error::UnknownSku {
                sku: sku.to_owned(),
            })
    }

    /// Every device file in the catalog.
    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.iter()
    }

    /// Every SKU that resolves, aliases included.
    pub fn skus(&self) -> impl Iterator<Item = &str> {
        self.index.keys().map(String::as_str)
    }
}

fn parse(file: &str, yaml: &str) -> Result<Device> {
    let device: Device = serde_norway::from_str(yaml).map_err(|e| Error::DeviceFile {
        file: file.to_owned(),
        source: Box::new(e),
    })?;
    if device.schema_version != SCHEMA_VERSION {
        return Err(Error::SchemaVersion {
            file: file.to_owned(),
            found: device.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    Ok(device)
}
