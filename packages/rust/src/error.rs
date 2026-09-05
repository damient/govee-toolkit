//! Errors the facade can return.

use crate::codec::{Mode, Role};
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

    /// The device file names no command that reports state in this mode.
    ///
    /// Fire-and-verify does without it and says so;
    /// [`crate::DeviceHandle::status`] fails rather than guessing an entry
    /// name. Mark the right entry `role: status` in `devices/<SKU>.yaml`.
    #[error("{sku}: no command in `commands.{mode}` is marked `role: status`")]
    NoStatusCommand {
        /// The SKU whose file is missing it.
        sku: String,
        /// The mode that was asked for.
        mode: Mode,
    },

    /// The device file names no command for a role the segment stream needs.
    ///
    /// Mark the right entries `role: segment_enable` and `role: segment_color`
    /// in `devices/<SKU>.yaml`; the stream will not guess an entry name.
    #[error("{sku}: no command in `commands.{mode}` is marked `role: {role}`")]
    NoSegmentCommand {
        /// The SKU whose file is missing it.
        sku: String,
        /// The mode that was asked for.
        mode: Mode,
        /// The role nothing claims.
        role: Role,
    },

    /// A stream was asked for native resolution on a unit nobody measured.
    ///
    /// `native_pixels` is a property of the physical unit, so it cannot be
    /// extrapolated from another one and the app's zone count is a different
    /// fact, not a substitute. Measure it — `docs/protocol/lan.md` 2.3 — and
    /// record it in the device file, or ask for a zone count explicitly.
    #[error("{sku}: `capabilities.native_pixels` is not measured on this unit")]
    NativeResolutionUnknown {
        /// The SKU whose file leaves it at zero.
        sku: String,
    },

    /// A frame carried a different number of colors than the stream streams.
    ///
    /// The zone count is fixed when the stream opens: the firmware reads it
    /// from the frame, and changing it mid-stream re-groups the LEDs.
    #[error("this stream carries {expected} zones, not {got}")]
    ZoneCountMismatch {
        /// What the stream was opened with.
        expected: usize,
        /// What the caller supplied.
        got: usize,
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
            Self::NoStatusCommand { .. } => "no_status_command",
            Self::NoSegmentCommand { .. } => "no_segment_command",
            Self::NativeResolutionUnknown { .. } => "native_resolution_unknown",
            Self::ZoneCountMismatch { .. } => "zone_count_mismatch",
            Self::LocalDevices { .. } => "local_devices",
        }
    }
}
