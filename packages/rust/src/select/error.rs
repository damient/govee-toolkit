use crate::codec::Mode;

/// Why a target names no device.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A target with nothing in it.
    #[error("a target names no device")]
    Empty,

    /// A prefix with nothing after it.
    #[error("`{prefix}:` names no {prefix}")]
    EmptyValue {
        /// The prefix that was written.
        prefix: String,
    },

    /// A bare target reads as two kinds. The prefixed forms say which one is
    /// meant.
    #[error(
        "`{target}` reads as a {kind} and as a {other}; write `{kind}:{target}` or `{other}:{target}`"
    )]
    Ambiguous {
        /// What was written.
        target: String,
        /// The kind it reads as first: `id`, `sku` or `name`.
        kind: String,
        /// The other kind it reads as: `name` or `group`.
        other: String,
    },

    /// A target that matches no device the SDK knows. A scan is what makes a
    /// device known; the cache answers for `lan` between runs.
    #[error("`{target}` matches no known device")]
    NoMatch {
        /// The target, in its prefixed form.
        target: String,
    },

    /// A SKU, a name or a group matches known devices, and the configuration
    /// enables the mode the caller will drive for none of them.
    #[error("`{target}` matches no known device that enables `{mode}`")]
    NotOnMode {
        /// The target, in its prefixed form.
        target: String,
        /// The mode the caller will drive.
        mode: Mode,
    },

    /// A command that drives one device got a target that names a model or a
    /// group.
    #[error(
        "`{target}` can name more than one device; this command takes one device, by identity or by name"
    )]
    NotOne {
        /// The target, in its prefixed form.
        target: String,
    },

    /// A command that reads the configuration alone got a SKU, which only a
    /// scan resolves.
    #[error("`{target}` names a model; this command takes an identity, a name or a group")]
    Model {
        /// The target, in its prefixed form.
        target: String,
    },

    /// A command that drives one device got a name that the configuration
    /// gives to more than one device.
    #[error("`{target}` names more than one device in the configuration")]
    Several {
        /// The target, in its prefixed form.
        target: String,
    },
}

impl Error {
    pub(crate) fn ambiguous(target: &str, kind: &str, other: &str) -> Self {
        Self::Ambiguous {
            target: target.trim().to_owned(),
            kind: kind.to_owned(),
            other: other.to_owned(),
        }
    }

    /// A stable, language-neutral identifier for this failure. One namespace
    /// with [`crate::Error::code`].
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty | Self::EmptyValue { .. } | Self::NotOne { .. } | Self::Model { .. } => {
                "target_not_understood"
            }
            Self::Ambiguous { .. } | Self::Several { .. } => "ambiguous_target",
            Self::NoMatch { .. } | Self::NotOnMode { .. } => "no_such_target",
        }
    }
}
