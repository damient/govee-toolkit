//! What a transport reports: per-device health, discoveries, and the event
//! stream every mode publishes into.
//!
//! None of it names a mode in its shape. A device is addressed by a string
//! whose meaning belongs to the transport — a socket address over `lan`, a
//! Bluetooth address over `ble` — because an application that lists devices
//! wants to show where one is, not to parse it.

use std::time::Instant;

use crate::codec::Mode;
use crate::transport::DeviceId;
use crate::transport::breaker::{Breaker, State, Transition};
use crate::transport::status::DeviceStatus;

/// A command that was written out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    /// Which device it went to.
    pub id: DeviceId,
    /// Which mode served it. Carried so that a caller reporting "which mode
    /// served this command" reads the same whatever the transport was.
    pub mode: Mode,
    /// What went out under the protocol's own name for it: the `msg.cmd` over
    /// `lan`, the device file's entry name where the wire carries no name.
    pub cmd: String,
    /// Where it went.
    pub endpoint: String,
}

/// A device's health in one mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// Breaker state.
    pub state: State,
    /// Consecutive unanswered verifications.
    pub failures: u32,
    /// Whether a command would be sent right now.
    pub available: bool,
}

/// A device a transport can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownDevice {
    /// Its identity.
    pub id: DeviceId,
    /// Where it was last reached.
    pub endpoint: String,
    /// The SKU it reports.
    pub sku: String,
    /// Its health.
    pub health: Health,
}

/// A device that answered a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovered {
    /// Its identity.
    pub id: DeviceId,
    /// Where it answered from.
    pub endpoint: String,
    /// The SKU it reports.
    pub sku: String,
    /// The firmware versions it reports, when it reports any.
    pub firmware: Option<String>,
}

/// What a discovery changed about what was already known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// An identity the transport had never seen.
    New,
    /// A known device, reachable where it was before.
    Refreshed,
    /// A known device that moved — a new DHCP lease, usually.
    Moved,
    /// A known device whose reported firmware changed. Worth surfacing:
    /// `docs/protocol/lan.md` §2.8, behavior can open or close with an update.
    FirmwareChanged,
}

/// Something worth telling the application about.
///
/// `docs/modes.md` requires every mode transition to be subscribable; the rest
/// is here because an application that shows devices needs it and polling for
/// it would be worse. Every variant carries the mode it is about, so an
/// application subscribes once and does not care how many transports exist.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// A device answered a scan.
    Discovered {
        /// The mode that found it.
        mode: Mode,
        /// What it reported.
        device: Discovered,
        /// Whether it is new, has moved, or has been updated.
        change: Change,
    },
    /// A cached device has not answered a scan for long enough to be dropped.
    Forgotten {
        /// The mode that forgot it.
        mode: Mode,
        /// The device that was dropped.
        id: DeviceId,
    },
    /// A command was written out.
    Sent(Sent),
    /// A device reported its state.
    Status {
        /// The mode that heard it.
        mode: Mode,
        /// What it reported.
        status: DeviceStatus,
    },
    /// A device's health in one mode changed.
    HealthChanged {
        /// The device.
        id: DeviceId,
        /// The mode.
        mode: Mode,
        /// What it moved from and to.
        transition: Transition,
    },
}

pub(crate) fn health_of(breaker: &Breaker, now: Instant) -> Health {
    Health {
        state: breaker.state(),
        failures: breaker.failures(),
        available: breaker.allows(now),
    }
}
