//! Why a patch does not load.
//!
//! An operator reads these next to a desk, so every message names the entry
//! and, where the fault is a channel range, the range itself.

use govee_toolkit::DeviceId;
use thiserror::Error;

use super::address::PortAddress;
use crate::profile::{self, Personality, UNIVERSE};

/// The channels one fixture answers to, on one port-address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The port-address the fixture sits on.
    pub universe: PortAddress,
    /// The start address, 1 to 512.
    pub first: u16,
    /// The last channel the fixture answers to.
    pub last: u16,
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "universe {} channels {} to {}",
            self.universe, self.first, self.last
        )
    }
}

/// One thing wrong with a patch.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// The file cannot be read.
    #[error("cannot read the patch `{path}`: {reason}")]
    Read {
        /// The file named on the command line.
        path: String,
        /// What the operating system reported.
        reason: String,
    },
    /// The file is not the patch format. An unknown key lands here: a
    /// misspelled option that was ignored would read as a setting that did
    /// not work.
    #[error("cannot parse the patch `{path}`: {reason}")]
    Parse {
        /// The file named on the command line.
        path: String,
        /// What the parser reported, with the line it stopped on.
        reason: String,
    },
    /// The port-address does not fit the 15 bits Art-Net holds.
    #[error("{device}: {field} is {value}, over the {max} it holds")]
    Address {
        /// The entry it is about.
        device: DeviceId,
        /// Which of `net`, `subnet` and `universe` is too large.
        field: &'static str,
        /// What the patch says.
        value: u16,
        /// The largest value the field holds.
        max: u16,
    },
    /// The start address is outside a universe.
    #[error(
        "{device}: address {address} is outside the 1 to {} a universe holds",
        UNIVERSE
    )]
    StartAddress {
        /// The entry it is about.
        device: DeviceId,
        /// What the patch says.
        address: u16,
    },
    /// The fixture runs past the end of its universe. The patch is never
    /// truncated to fit, and it never spills into the next universe.
    #[error(
        "{device}: `{personality}` from address {} takes {span}, past the {} a universe holds",
        span.first,
        UNIVERSE
    )]
    PastUniverse {
        /// The entry it is about.
        device: DeviceId,
        /// The personality it asks for.
        personality: Personality,
        /// The channels it would take.
        span: Span,
    },
    /// Two driven entries answer to one channel under different channel
    /// tables. One value then means two things, and neither operator can tell
    /// which fixture took it. Two entries on one span under one personality
    /// are a clone, and no fault.
    #[error("{first} and {second} overlap: {first_span} and {second_span}")]
    Overlap {
        /// The entry that sits lower.
        first: DeviceId,
        /// The channels it takes.
        first_span: Span,
        /// The entry that sits higher.
        second: DeviceId,
        /// The channels it takes.
        second_span: Span,
    },
    /// One device is patched twice. A second entry would send two looks to
    /// one device.
    #[error("{device} is patched twice")]
    Twice {
        /// The entry it is about.
        device: DeviceId,
    },
    /// Nothing states how many channels the entry holds, so the next scan
    /// could hand them to a second fixture.
    #[error(
        "{device}: the device did not answer and the entry carries no `sku:`, so nothing states the channels it holds"
    )]
    Unsized {
        /// The entry it is about.
        device: DeviceId,
    },
    /// Nothing says what the device is, so nothing says what it serves.
    #[error("{device}: no device of that identity answered; the bridge drives what it found")]
    Unknown {
        /// The entry it is about.
        device: DeviceId,
    },
    /// The device serves the personality through nothing.
    #[error("{device}: {source}")]
    Unserved {
        /// The entry it is about.
        device: DeviceId,
        /// What the channel table refused.
        source: profile::Error,
    },
}
