//! The patch: which device answers which channels, on which universe.
//!
//! The patch is a file of its own, named on the command line. The format is
//! `docs/dmx.md` 2. An unknown key is refused: a misspelled option that was
//! ignored would read as a setting that did not work.
//!
//! [`Patch::load`] reads the file and checks what the file alone states.
//! [`Patch::resolve`] joins it to the devices the bridge found, and that is
//! where the width, the overlaps and the personalities are checked.

mod address;
mod entry;
mod error;
mod label;
mod plan;
mod resolve;
pub mod stamp;
#[cfg(test)]
mod tests;
mod write;

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use self::address::{MAX as MAX_PORT_ADDRESS, PortAddress};
pub use self::entry::{Entry, SignalLoss};
pub use self::error::{Error, Span};
pub use self::label::Label;
pub use self::plan::{Candidate, Layout, Placement, Plan, Skip};
pub use self::resolve::{Fixture, Rig};
pub use self::write::update;

/// The file `govee-dmx` reads where the command line names none:
/// `$XDG_CONFIG_HOME/govee-toolkit/patch.yaml`, beside `config.yaml`.
#[must_use]
pub fn default_path() -> PathBuf {
    govee_toolkit::paths::config_dir().join("patch.yaml")
}

/// The name a desk lists the node under, where the patch names none.
pub const NODE_NAME: &str = "govee-toolkit";
/// How long a device goes without a write before it receives the current
/// values once. Nothing acknowledges a LAN frame.
pub const REFRESH_SECS: u64 = 10;
/// How long a fixture waits for a frame before `on_signal_loss` decides what
/// it shows. Art-Net calls a sender lost after 4 seconds.
pub const SIGNAL_LOSS_SECS: u64 = 4;
/// How long a fixture stays on and black before the dimmer at 0 powers it
/// off. Long enough that a cue that dips through 0 costs no power command,
/// short enough that a blackout leaves no device powered.
pub const OFF_DELAY_SECS: u64 = 5;

/// A patch file.
///
/// [`Patch::plan`] answers what a scan adds to it, and [`update`] writes those
/// entries back without moving a line of what is already there.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Patch {
    /// The instant the last scan ran, as RFC 3339 in UTC. `govee-dmx patch`
    /// writes it, and nothing reads it: it states how old the rig below is.
    pub scanned: Option<String>,
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
    /// Seconds. The default is [`REFRESH_SECS`].
    pub refresh_secs: u64,
    /// Seconds. The default is [`SIGNAL_LOSS_SECS`].
    pub signal_loss_secs: u64,
    /// Seconds, and `0` powers a fixture off at once. The default is
    /// [`OFF_DELAY_SECS`].
    pub off_delay_secs: u64,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            name: NODE_NAME.to_owned(),
            refresh_secs: REFRESH_SECS,
            signal_loss_secs: SIGNAL_LOSS_SECS,
            off_delay_secs: OFF_DELAY_SECS,
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
