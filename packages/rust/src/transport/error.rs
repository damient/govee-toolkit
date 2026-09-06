//! Errors a transport can return, whichever mode it serves.
//!
//! Nothing here is recoverable by substituting something else. A device that
//! cannot be reached produces [`Error::Unreachable`] or [`Error::Unavailable`],
//! and that is the answer the caller gets — choosing another mode is the
//! facade's decision to make, from the user's configuration, never a
//! transport's.

use crate::codec::Mode;
use crate::transport::DeviceId;
use crate::transport::breaker::State;

/// Anything that can go wrong reaching a device.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No device with this identity has been discovered, and none is cached.
    ///
    /// Note what this is *not*: a reason to scan. Scanning on the send path
    /// costs a multicast round-trip — `docs/protocol/lan.md` §1, latency notes.
    #[error("no known device `{id}`; it has not been discovered and is not in the cache")]
    UnknownDevice {
        /// The identity that was asked for.
        id: DeviceId,
    },

    /// The breaker refuses this mode for now, from state already known.
    #[error("{id}: `{mode}` is {state} and in cooldown; the command was not sent")]
    Unavailable {
        /// The device.
        id: DeviceId,
        /// The mode that is refused.
        mode: Mode,
        /// Why it is refused.
        state: State,
    },

    /// A command was sent and the device did not answer within the deadline.
    #[error("{id}: no answer from {endpoint} within {timeout_ms} ms")]
    Unreachable {
        /// The device.
        id: DeviceId,
        /// Where the command went, in whatever form the mode addresses a
        /// device: a socket address over `lan`, a Bluetooth address over `ble`.
        endpoint: String,
        /// How long it was given.
        timeout_ms: u64,
    },

    /// An encoded command could not be serialized into a datagram. It cannot
    /// happen for a value [`crate::codec`] built; it is here so that no code
    /// path has to unwrap.
    #[error("{cmd}: the encoded command is not serializable: {reason}")]
    Serialize {
        /// The command.
        cmd: String,
        /// What serde reported.
        reason: String,
    },

    /// A transport option is outside the range the transport can honour. It is
    /// refused rather than moved to the nearest value it could serve: an option
    /// quietly replaced is an option the caller never set.
    #[error("`{field}` is out of range: {reason}")]
    Option {
        /// The field, as it is named on the mode's options type.
        field: String,
        /// What the range is, and what was given.
        reason: String,
    },

    /// The transport's receive loop is gone, so nothing can be sent or awaited.
    #[error("the transport has been shut down")]
    ShutDown,

    /// An adapter or socket operation failed.
    #[error("{context}: {source}")]
    Io {
        /// What was being attempted.
        context: String,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// The on-disk device cache could not be read or written.
    #[error("device cache `{path}`: {reason}")]
    Cache {
        /// The file.
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
    /// Shares the namespace of [`crate::codec::Error::code`], so a binding
    /// surfaces one flat set of codes whatever layer failed.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownDevice { .. } => "unknown_device",
            Self::Unavailable { .. } => "mode_unavailable",
            Self::Unreachable { .. } => "unreachable",
            Self::Serialize { .. } => "serialize",
            Self::Option { .. } => "out_of_range",
            Self::ShutDown => "shut_down",
            Self::Io { .. } => "io",
            Self::Cache { .. } => "cache",
        }
    }

    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}
