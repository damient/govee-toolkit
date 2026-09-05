//! Errors the facade can return.

use crate::codec::Mode;
use crate::config::Problem;
use crate::lan::DeviceId;

/// Anything that can go wrong between a call and the bytes on the wire.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Encoding failed: an unknown SKU, an unknown command, an argument out of
    /// range. Nothing was sent.
    #[error(transparent)]
    Codec(#[from] crate::codec::Error),

    /// The transport failed.
    #[error(transparent)]
    Transport(#[from] crate::lan::Error),

    /// The configuration file could not be read or parsed.
    #[error("configuration `{path}`: {reason}")]
    Config {
        /// The file.
        path: String,
        /// What is wrong with it.
        reason: String,
    },

    /// The configuration enables something that cannot work.
    ///
    /// Reported at startup, as `docs/modes.md` requires: a mode the hardware
    /// does not support is a mistake to fix, not a command to fail later.
    #[error("the configuration cannot be applied:\n  {}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n  "))]
    Configuration(Vec<Problem>),

    /// Every enabled mode is refused right now.
    ///
    /// Not a reason to try something else: the modes listed here are the ones
    /// the user enabled, and there is nothing beyond them.
    #[error("{id}: none of the enabled modes is available ({})", .modes.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))]
    NoModeAvailable {
        /// The device.
        id: DeviceId,
        /// The modes that were tried, in preference order.
        modes: Vec<Mode>,
    },

    /// A mode the configuration enables has no transport yet.
    ///
    /// Explicit rather than skipped: silently moving on to the next mode would
    /// be substituting one for another, which this SDK does not do.
    #[error("{id}: mode `{mode}` is enabled but not implemented yet")]
    ModeNotImplemented {
        /// The device.
        id: DeviceId,
        /// The mode.
        mode: Mode,
    },

    /// A device file could not be read from the user's own directory.
    #[error("local device file `{path}`: {reason}")]
    LocalDevices {
        /// The file or directory.
        path: String,
        /// What went wrong.
        reason: String,
    },
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// A stable, language-neutral identifier for this failure.
    ///
    /// Shares the namespace of [`crate::codec::Error::code`] and
    /// [`crate::lan::Error::code`], so a binding surfaces one flat set of codes
    /// whatever layer failed.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Codec(e) => e.code(),
            Self::Transport(e) => e.code(),
            Self::Config { .. } => "config",
            Self::Configuration(_) => "configuration",
            Self::NoModeAvailable { .. } => "no_mode_available",
            Self::ModeNotImplemented { .. } => "mode_not_implemented",
            Self::LocalDevices { .. } => "local_devices",
        }
    }
}
