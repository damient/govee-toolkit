//! The patch: which device answers which channels, on which universe.
//!
//! The patch is a file of its own, named on the command line. It is not
//! `config.yaml`: that file says which modes are enabled for a device, and a
//! rig of 40 fixtures does not belong in it.
//!
//! ```yaml
//! node:
//!   bind: 0.0.0.0
//!   name: govee-toolkit       # what the desk shows in its node list
//!   refresh_secs: 10
//!   signal_loss_secs: 4       # how long a fixture waits for a frame
//! patch:
//!   - device: "AA:BB:CC:DD:EE:FF"
//!     universe: 0             # or: net: 0, subnet: 0, universe: 0
//!     address: 1              # the DMX start address, 1 to 512
//!     personality: segment
//!     max_hz: 20              # optional. The rate the fixture takes writes
//!                             # at. Default: the device file measurement for a
//!                             # zone personality, and 30 for a command
//!     on_signal_loss: hold
//! ```
//!
//! An unknown key is refused: a misspelled option that was ignored would read
//! as a setting that did not work.
//!
//! [`Patch::load`] reads the file and checks what the file alone states.
//! [`Patch::resolve`] joins it to the devices the bridge found, and that is
//! where the width, the overlaps and the personalities are checked. See
//! `docs/dmx.md`.

mod address;
mod entry;
mod error;
mod resolve;
#[cfg(test)]
mod tests;

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

use serde::Deserialize;

pub use self::address::{MAX as MAX_PORT_ADDRESS, PortAddress};
pub use self::entry::{Entry, SignalLoss};
pub use self::error::{Error, Span};
pub use self::resolve::{Fixture, Rig};

/// The name a desk lists the node under, where the patch names none.
pub const NODE_NAME: &str = "govee-toolkit";
/// How long a device goes without a write before it receives the current
/// values once. Nothing acknowledges a LAN frame.
pub const REFRESH_SECS: u64 = 10;
/// How long a fixture waits for a frame before `on_signal_loss` decides what
/// it shows. Art-Net calls a sender lost after 4 seconds.
pub const SIGNAL_LOSS_SECS: u64 = 4;

/// A patch file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Patch {
    /// What the node itself does.
    pub node: Node,
    /// One entry per fixture.
    pub patch: Vec<Entry>,
}

/// The node settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Node {
    /// The address to bind the input socket to. `0.0.0.0` takes every
    /// interface, which is what a broadcast frame needs.
    pub bind: IpAddr,
    /// What the desk shows in its node list.
    pub name: String,
    /// How long a device goes without a write before the bridge sends the
    /// current values once.
    pub refresh_secs: u64,
    /// How long a fixture waits for a frame before `on_signal_loss` decides
    /// what it shows.
    pub signal_loss_secs: u64,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            name: NODE_NAME.to_owned(),
            refresh_secs: REFRESH_SECS,
            signal_loss_secs: SIGNAL_LOSS_SECS,
        }
    }
}

impl Patch {
    /// Read a patch from `path`.
    ///
    /// A missing file is an error here, unlike `config.yaml`: the operator
    /// named this file, so a typo in the name must not run an empty rig.
    ///
    /// # Errors
    ///
    /// [`Error::Read`] where the file cannot be read, and [`Error::Parse`]
    /// where it is not the patch format. A patch that does not parse is never
    /// guessed at.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| Error::Read {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        Self::parse(&text, &path.display().to_string())
    }

    /// Read a patch from `text`. `path` names it in an error.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`], as [`Patch::load`] states.
    pub fn parse(text: &str, path: &str) -> Result<Self, Error> {
        serde_norway::from_str(text).map_err(|e| Error::Parse {
            path: path.to_owned(),
            reason: e.to_string(),
        })
    }
}
