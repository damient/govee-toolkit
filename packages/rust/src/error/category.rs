//! The family of a failure, which a binding maps to an error class.

/// What kind of failure an [`Error`](super::Error) is. A binding maps this to
/// the exception or the error class it raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Category {
    /// The command could not be built. Nothing was sent.
    Codec,
    /// A mode failed to carry the command, or the device refused it.
    Transport,
    /// The configuration is unreadable, or it enables what cannot work.
    Config,
}

impl Category {
    /// The name of the class every failure belongs to, whatever its category.
    pub const BASE_CLASS: &'static str = "GoveeError";

    /// A stable, language-neutral name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codec => "codec",
            Self::Transport => "transport",
            Self::Config => "config",
        }
    }

    /// The name a binding gives the error class of this category. Every
    /// binding raises the same name for the same failure.
    #[must_use]
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Codec => "CodecError",
            Self::Transport => "TransportError",
            Self::Config => "ConfigError",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
