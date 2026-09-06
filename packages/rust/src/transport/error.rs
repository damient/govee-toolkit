//! Errors a transport can return, whichever mode it serves.
//!
//! No substitution recovers any of these. A device the transport cannot reach
//! produces [`Error::Unreachable`] or [`Error::Unavailable`], and the caller
//! gets that answer. Only the facade chooses another mode, from the user's
//! configuration.

use crate::codec::Mode;
use crate::transport::DeviceId;
use crate::transport::breaker::State;

/// Anything that can go wrong reaching a device.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No device with this identity has been discovered, and none is cached.
    ///
    /// This is not a reason to scan: a scan on the send path costs a multicast
    /// round-trip (`docs/protocol/lan.md` §1, latency notes).
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
        /// Where the command went, in the form the mode addresses a device: a
        /// socket address over `lan`, a Bluetooth address over `ble`.
        endpoint: String,
        /// How long it was given.
        timeout_ms: u64,
    },

    /// An encoded command does not serialize into a datagram. It cannot happen
    /// for a value [`crate::codec`] built; it exists so that no code path must
    /// unwrap.
    #[error("{cmd}: the encoded command is not serializable: {reason}")]
    Serialize {
        /// The command.
        cmd: String,
        /// What serde reported.
        reason: String,
    },

    /// A transport option is outside the range the transport can honour. The
    /// transport refuses it and never moves it to the nearest value it can
    /// serve: an option quietly replaced is an option the caller never set.
    #[error("`{field}` is out of range: {reason}")]
    Option {
        /// The field, as it is named on the mode's options type.
        field: String,
        /// What the range is, and what was given.
        reason: String,
    },

    /// There is nothing to read: the command declares no `reply:` layout this
    /// mode could match, or the mode does not answer in frames at all.
    #[error("`{mode}`: {reason}")]
    NoReplyLayout {
        /// The mode that was asked.
        mode: Mode,
        /// Why nothing can be read.
        reason: String,
    },

    /// The transport's receive loop is gone, so nothing can be sent or awaited.
    #[error("the transport has been shut down")]
    ShutDown,

    /// An adapter or socket operation failed.
    #[error("{context}: {source}")]
    Io {
        /// The operation that failed.
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
            Self::NoReplyLayout { .. } => "no_reply_layout",
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
