//! Errors.
//!
//! Every failure is typed and explicit. The firmware clamps out-of-range values
//! in silence (`docs/protocol/lan.md` 2.1); this crate does not — an argument
//! outside its declared range is [`Error::OutOfRange`], never a clamped value.

use crate::catalog::Mode;

/// Anything that can go wrong turning a device file and arguments into bytes.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No device file declares this SKU, as `sku` or as a verified alias.
    #[error("unknown SKU `{sku}`")]
    UnknownSku {
        /// The SKU that was looked up.
        sku: String,
    },

    /// The hardware does not support the mode at all.
    #[error("{sku}: mode `{mode}` is not supported by this device")]
    ModeUnsupported {
        /// The device.
        sku: String,
        /// The mode that was asked for.
        mode: Mode,
    },

    /// The mode is supported but does not carry this command. Never approximate
    /// it with another mode — see `docs/modes.md`.
    #[error("{sku}: command `{command}` is not reachable over `{mode}`")]
    UnknownCommand {
        /// The device.
        sku: String,
        /// The mode that was asked for.
        mode: Mode,
        /// The command that was asked for.
        command: String,
    },

    /// A declared argument was not supplied.
    #[error("{command}: missing argument `{arg}`")]
    MissingArg {
        /// The command being encoded.
        command: String,
        /// The declared argument that has no value.
        arg: String,
    },

    /// An argument was supplied that the command does not declare.
    #[error("{command}: unknown argument `{arg}`")]
    UnknownArg {
        /// The command being encoded.
        command: String,
        /// The supplied argument.
        arg: String,
    },

    /// An argument was supplied with the wrong shape.
    #[error("{command}: argument `{arg}` expects {expected}, got {got}")]
    ArgType {
        /// The command being encoded.
        command: String,
        /// The argument.
        arg: String,
        /// What the device file declares.
        expected: &'static str,
        /// What was supplied.
        got: &'static str,
    },

    /// An integer argument fell outside the range the device file declares.
    #[error("{command}: argument `{arg}` = {value} is outside {min}..={max}")]
    OutOfRange {
        /// The command being encoded.
        command: String,
        /// The argument.
        arg: String,
        /// The value supplied.
        value: i64,
        /// Lower bound, inclusive.
        min: i64,
        /// Upper bound, inclusive.
        max: i64,
    },

    /// A repeat count was supplied and disagrees with the length of the list it
    /// counts. Supplying neither is fine — the count is derived from the list.
    #[error("{command}: `{count_arg}` = {declared} but `{list_arg}` holds {actual} items")]
    RepeatCountMismatch {
        /// The command being encoded.
        command: String,
        /// The argument used as the repeat count.
        count_arg: String,
        /// The list argument the group draws from.
        list_arg: String,
        /// The count that was supplied.
        declared: usize,
        /// The number of items actually supplied.
        actual: usize,
    },

    /// The `frame:` string in the device file is not valid.
    #[error("{command}: invalid frame `{frame}`: {reason}")]
    FrameSyntax {
        /// The command being encoded.
        command: String,
        /// The offending frame string.
        frame: String,
        /// What is wrong with it.
        reason: String,
    },

    /// A value does not fit the width the frame declares for it.
    #[error("{command}: `{arg}` = {value} does not fit in {bits} bits")]
    FrameWidth {
        /// The command being encoded.
        command: String,
        /// The argument.
        arg: String,
        /// The value supplied.
        value: i64,
        /// The declared width.
        bits: u32,
    },

    /// The `payload:` template holds a placeholder that resolves to nothing.
    #[error("{command}: payload placeholder `${{{name}}}` has no value")]
    UnresolvedPlaceholder {
        /// The command being encoded.
        command: String,
        /// The placeholder name.
        name: String,
    },

    /// A device file did not parse.
    #[error("device file `{file}`: {source}")]
    DeviceFile {
        /// The file name.
        file: String,
        /// The underlying parse error.
        #[source]
        source: Box<serde_norway::Error>,
    },

    /// Two device files claim the same SKU or alias.
    #[error("`{sku}` is declared by both `{first}` and `{second}`")]
    DuplicateSku {
        /// The contested SKU.
        sku: String,
        /// First file to declare it.
        first: String,
        /// Second file to declare it.
        second: String,
    },
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// A stable, language-neutral identifier for this failure.
    ///
    /// Ports and bindings surface the same code for the same condition, so the
    /// conformance vectors in `tests/fixtures/golden/` can assert on failures
    /// as well as on bytes.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownSku { .. } => "unknown_sku",
            Self::ModeUnsupported { .. } => "mode_unsupported",
            Self::UnknownCommand { .. } => "unknown_command",
            Self::MissingArg { .. } => "missing_arg",
            Self::UnknownArg { .. } => "unknown_arg",
            Self::ArgType { .. } => "arg_type",
            Self::OutOfRange { .. } => "out_of_range",
            Self::RepeatCountMismatch { .. } => "repeat_count_mismatch",
            Self::FrameSyntax { .. } => "frame_syntax",
            Self::FrameWidth { .. } => "frame_width",
            Self::UnresolvedPlaceholder { .. } => "unresolved_placeholder",
            Self::DeviceFile { .. } => "device_file",
            Self::DuplicateSku { .. } => "duplicate_sku",
        }
    }
}
