//! Errors the transport can return.
//!
//! Nothing here is recoverable by substituting something else. A device that
//! cannot be reached over `lan` produces [`Error::Unreachable`] or
//! [`Error::Unavailable`], and that is the answer the caller gets — choosing
//! another mode is the facade's decision to make, from the user's
//! configuration, never this crate's.

use std::net::SocketAddr;

use crate::DeviceId;
use crate::breaker::State;

/// Anything that can go wrong reaching a device over the local network.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Encoding failed before anything was sent.
    #[error(transparent)]
    Codec(#[from] govee_core::Error),

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
    #[error("{id}: `lan` is {state} and in cooldown; the command was not sent")]
    Unavailable {
        /// The device.
        id: DeviceId,
        /// Why it is refused.
        state: State,
    },

    /// A command was sent and the device did not answer within the deadline.
    #[error("{id}: no answer from {addr} within {timeout_ms} ms")]
    Unreachable {
        /// The device.
        id: DeviceId,
        /// Where the command went.
        addr: SocketAddr,
        /// How long it was given.
        timeout_ms: u64,
    },

    /// An encoded command could not be serialized into a datagram. It cannot
    /// happen for a value [`govee_core`] built; it is here so that no code path
    /// has to unwrap.
    #[error("{cmd}: the encoded command is not serializable: {reason}")]
    Serialize {
        /// The command.
        cmd: String,
        /// What serde reported.
        reason: String,
    },

    /// The transport's receive loop is gone, so nothing can be sent or awaited.
    #[error("the transport has been shut down")]
    ShutDown,

    /// A socket operation failed.
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
    /// Shares the namespace of [`govee_core::Error::code`], so a binding
    /// surfaces one flat set of codes whatever layer failed.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Codec(e) => e.code(),
            Self::UnknownDevice { .. } => "unknown_device",
            Self::Unavailable { .. } => "mode_unavailable",
            Self::Unreachable { .. } => "unreachable",
            Self::Serialize { .. } => "serialize",
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
