//! What the facade reports: its event stream, and the two shapes an
//! application reads devices through.

use govee_core::Mode;
use govee_lan::{DeviceId, Health};

/// Something worth telling the application about.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// Something the `lan` transport reported: a discovery, a status, a health
    /// transition. Health transitions carry the mode they are about, so an
    /// application subscribes once and does not care how many transports exist.
    Lan(govee_lan::Event),
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
    /// The mode that served it. The whole point of returning this: with several
    /// modes enabled, which one ran is not something a caller should guess.
    pub mode: Mode,
    /// The device file entry that was sent.
    pub command: String,
    /// The `msg.cmd` that went on the wire.
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
    /// Its health in `lan`, if `lan` is one of them.
    pub lan_health: Option<Health>,
}
