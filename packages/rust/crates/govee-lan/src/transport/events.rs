//! What the transport reports: per-device health, and the event stream.

use std::net::{IpAddr, SocketAddr};
use std::time::Instant;

use govee_core::Mode;

use crate::DeviceId;
use crate::breaker::{Breaker, State, Transition};
use crate::cache::Change;
use crate::discovery::DiscoveredDevice;
use crate::status::DeviceStatus;

/// A command that was written to the socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    /// Which device it went to.
    pub id: DeviceId,
    /// Which mode served it. Always `lan` here; carried so that a caller
    /// reporting "which mode served this command" reads the same whatever the
    /// transport was.
    pub mode: Mode,
    /// The `msg.cmd` that went out.
    pub cmd: String,
    /// Where it went.
    pub addr: SocketAddr,
}

/// A device's health in this mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// Breaker state.
    pub state: State,
    /// Consecutive unanswered verifications.
    pub failures: u32,
    /// Whether a command would be sent right now.
    pub available: bool,
}

/// A device the transport can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownDevice {
    /// Its identity.
    pub id: DeviceId,
    /// Its last known address.
    pub ip: IpAddr,
    /// The SKU it reports.
    pub sku: String,
    /// Its health.
    pub health: Health,
}

/// Something worth telling the application about.
///
/// `docs/modes.md` requires every mode transition to be subscribable; the rest
/// is here because an application that shows devices needs it and polling for
/// it would be worse.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// A device answered a scan.
    Discovered {
        /// What it reported.
        device: DiscoveredDevice,
        /// Whether it is new, has moved, or has been updated.
        change: Change,
    },
    /// A cached device has not answered a scan for
    /// [`Options::forget_after`](super::Options::forget_after).
    Forgotten {
        /// The device that was dropped.
        id: DeviceId,
    },
    /// A command was written to the socket.
    Sent(Sent),
    /// A device reported its state.
    Status(DeviceStatus),
    /// A device's health in this mode changed.
    HealthChanged {
        /// The device.
        id: DeviceId,
        /// The mode, always `lan` here.
        mode: Mode,
        /// What it moved from and to.
        transition: Transition,
    },
}

pub(super) fn health_of(breaker: &Breaker, now: Instant) -> Health {
    Health {
        state: breaker.state(),
        failures: breaker.failures(),
        available: breaker.allows(now),
    }
}
