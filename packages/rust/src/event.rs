//! What the facade reports: its event stream, and the two shapes an
//! application reads devices through.

use std::collections::BTreeMap;

use crate::codec::Mode;
use crate::transport::{DeviceId, Health};

/// One stream for the whole SDK: what the transports report, plus what only
/// the facade can see.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// Something a transport reported: a discovery, a status, a health
    /// transition. Every one carries the mode it is about, so an application
    /// subscribes once and does not care how many transports exist.
    Transport(crate::transport::Event),
    /// A device answered with a SKU the catalog does not know, so nothing can
    /// be encoded for it. Pin a known SKU in the configuration, or add a device
    /// file — see `devices/README.md`.
    UnknownSku {
        /// The device.
        id: DeviceId,
        /// What it reported.
        sku: String,
    },
}

/// A command that was served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Served {
    /// The device it went to.
    pub id: DeviceId,
    /// The mode that served it. With several modes enabled, a caller must not
    /// guess which one ran.
    pub mode: Mode,
    /// The device file entry that was sent.
    pub command: String,
    /// What went out under the protocol's own name for it: the `msg.cmd` over
    /// `lan`, the device file's entry name where the wire carries no name.
    pub cmd: String,
}

/// A device the facade knows about.
#[derive(Debug, Clone)]
pub struct Device {
    /// Its identity.
    pub id: DeviceId,
    /// The SKU it will be encoded under: the one the configuration pins, or the
    /// one it reports.
    pub sku: String,
    /// The name the configuration gives it, if any.
    pub name: Option<String>,
    /// The enabled modes, in preference order.
    pub modes: Vec<Mode>,
    /// Its health, per enabled mode a transport knows it in. A mode is absent
    /// when this build carries no transport for it, or when that transport has
    /// never heard from the device.
    pub health: BTreeMap<Mode, Health>,
}
