//! Errors the facade can return.

use crate::codec::{ArgRole, Mode, Role};
use crate::config::Problem;
use crate::transport::DeviceId;

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
    Transport(#[from] crate::transport::Error),

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

    /// The device file names no command for a role the SDK invokes on its own.
    ///
    /// The SDK does not guess an entry name: mark the right entry `role:
    /// status`, `role: segment_enable`, `role: segment_color` or `role:
    /// segment_color_masked` in `devices/<SKU>.yaml`. Fire-and-verify works
    /// without a status command and says so; [`crate::DeviceHandle::status`]
    /// fails instead.
    #[error("{sku}: no command in `commands.{mode}` is marked `role: {role}`")]
    NoRoleCommand {
        /// The SKU whose file is missing it.
        sku: String,
        /// The mode that was asked for.
        mode: Mode,
        /// The role nothing claims.
        role: Role,
    },

    /// A command the SDK invokes on its own does not mark the argument the SDK
    /// has to fill.
    ///
    /// The device file names arguments, so a `role:` command must say which of
    /// its own arguments carries what — `devices/schema.yaml`.
    /// `crate::codec::validate` catches this too; this error covers a file that
    /// reached the send path without it.
    #[error(
        "{sku}: `commands.{mode}.{command}` marks no argument `role: {arg_role}`, so there is nothing to put the value in"
    )]
    NoRoleArg {
        /// The SKU whose file is missing it.
        sku: String,
        /// The mode the command was taken from.
        mode: Mode,
        /// The device file entry.
        command: String,
        /// The argument role nothing claims.
        arg_role: ArgRole,
    },

    /// A stream was asked for a zone count nothing records.
    ///
    /// A zone count is a property of the physical unit: no count substitutes
    /// for another, and none follows from another. Measure it —
    /// `docs/protocol/lan.md` 2.3 — and record it in the device file, or ask
    /// for a zone count explicitly.
    #[error("{sku}: the zone count asked for is not recorded for this unit")]
    ZoneCountUnknown {
        /// The SKU whose file leaves it at zero.
        sku: String,
    },

    /// A frame carried a different number of colors than the stream streams.
    ///
    /// The zone count is fixed when the stream opens: the firmware reads it
    /// from the frame, and a change mid-stream re-groups the LEDs.
    #[error("this stream carries {expected} zones, not {got}")]
    ZoneCountMismatch {
        /// What the stream was opened with.
        expected: usize,
        /// What the caller supplied.
        got: usize,
    },

    /// A zone index past the last zone of the stream.
    #[error("zone {index} is past the last of this stream's {zones} zones")]
    ZoneOutOfRange {
        /// The zero-based index asked for.
        index: usize,
        /// What the stream carries.
        zones: usize,
    },

    /// A stream was asked for native resolution over a mode that paints by
    /// zone mask.
    ///
    /// Such a mode addresses zones, and the device file's zone count is what
    /// its mask can name; there is no per-pixel channel behind it. Ask for
    /// [`Zones::App`](crate::stream::Zones::App) or an explicit count instead.
    #[error("{sku}: mode `{mode}` paints zones by mask and cannot reach native resolution")]
    NativeZonesUnreachable {
        /// The SKU asked for.
        sku: String,
        /// The mode the stream was opened on.
        mode: Mode,
    },

    /// A stream was asked for more zones than the mode's mask can name.
    ///
    /// Refused rather than sent: the firmware drops a mask with bits past the
    /// last zone in silence, so the frame would look sent and paint nothing.
    #[error("{sku}: mode `{mode}` addresses {limit} zones, not {zones}")]
    ZoneCountUnsupported {
        /// The SKU asked for.
        sku: String,
        /// The mode the stream was opened on.
        mode: Mode,
        /// What the caller asked for.
        zones: usize,
        /// What the device file declares the mask can name.
        limit: usize,
    },

    /// A stream opened on a mode whose device file bounds its zone mask by
    /// nothing.
    ///
    /// The bound is the `count:` on the argument marked `role: zones`, or the
    /// width of the mask field the layout writes it into. A file that declares
    /// neither says nothing about how many zones the mask reaches, and a mask
    /// the firmware drops looks exactly like one it applied, so the stream
    /// refuses to arm.
    #[error("{sku}: mode `{mode}`, command `{command}` bounds its zone mask by nothing")]
    ZoneMaskUnbounded {
        /// The SKU asked for.
        sku: String,
        /// The mode the stream was opened on.
        mode: Mode,
        /// The device file entry marked `role: segment_color_masked`.
        command: String,
    },

    /// A stream was asked for a rate at or below zero.
    ///
    /// Out of range, not clamped: a rate is a division on the send path, and
    /// the nearest legal value would flood the channel this stream paces.
    #[error("a stream rate must be above zero, not {hz}")]
    StreamRateOutOfRange {
        /// What the caller asked for, in hertz.
        hz: f64,
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
    /// [`crate::transport::Error::code`], so a binding surfaces one flat set of
    /// codes whatever layer failed.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Codec(e) => e.code(),
            Self::Transport(e) => e.code(),
            Self::Config { .. } => "config",
            Self::Configuration(_) => "configuration",
            Self::NoModeAvailable { .. } => "no_mode_available",
            Self::ModeNotImplemented { .. } => "mode_not_implemented",
            Self::NoRoleCommand { role, .. } => match role {
                Role::Status => "no_status_command",
                Role::SegmentEnable
                | Role::SegmentColor
                | Role::SegmentColorMasked
                | Role::SegmentGradient => "no_segment_command",
            },
            Self::NoRoleArg { .. } => "no_role_arg",
            Self::ZoneCountUnknown { .. } => "zone_count_unknown",
            Self::ZoneCountMismatch { .. } => "zone_count_mismatch",
            Self::ZoneOutOfRange { .. } => "zone_out_of_range",
            Self::NativeZonesUnreachable { .. } => "native_zones_unreachable",
            Self::ZoneCountUnsupported { .. } => "zone_count_unsupported",
            Self::ZoneMaskUnbounded { .. } => "zone_mask_unbounded",
            Self::StreamRateOutOfRange { .. } => "stream_rate_out_of_range",
            Self::LocalDevices { .. } => "local_devices",
        }
    }
}
