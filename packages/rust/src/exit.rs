//! What a binary writes to the process streams, and the code it exits with.
//!
//! `govee` and `govee-dmx` report through this module, so one exit code and
//! one error record mean the same thing in both. The JSON form is the
//! contract: one object per line on stdout, an error object on stderr, and
//! the exit codes below. The text form is for a person and its layout can
//! change at any release.
//!
//! This is the one module of the crate that writes to the process streams.
//! The library writes through `tracing` — see `docs/architecture.md`.

// The module a binary reports through. The `print` lints protect a host
// application from a library, and nothing here runs inside one.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::process::ExitCode;

use serde_json::{Value, json};

/// Something failed that no other code names.
pub const INTERNAL: u8 = 1;
/// The command line is wrong, or it names something no device file carries.
/// clap reports its own with the same code.
pub const USAGE: u8 = 2;
/// The configuration or the patch cannot be read, or cannot be applied.
pub const CONFIG: u8 = 3;
/// No enabled mode reaches the device. There is no fallback beyond this.
pub const UNREACHABLE: u8 = 4;
/// The command or an argument is refused. Nothing was sent.
pub const REFUSED: u8 = 5;
/// The command line names something this build does not carry.
pub const UNSUPPORTED: u8 = 6;

/// What failed: the name a script reads, the line a person reads, and the
/// exit code.
#[derive(Debug)]
pub struct Failure {
    kind: &'static str,
    message: String,
    code: u8,
}

impl Failure {
    /// A fault the binary itself hit: a runtime that does not build, a socket
    /// it cannot open. [`INTERNAL`].
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message, INTERNAL)
    }

    /// A command line that cannot be read, or that names something no device
    /// file carries. [`USAGE`].
    #[must_use]
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new("usage", message, USAGE)
    }

    /// A configuration or a patch that cannot be read or applied. [`CONFIG`].
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::new("config", message, CONFIG)
    }

    /// A device no enabled mode reaches. [`UNREACHABLE`].
    #[must_use]
    pub fn unreachable(message: impl Into<String>) -> Self {
        Self::new("unreachable", message, UNREACHABLE)
    }

    /// A command the mode or the device file does not carry. Nothing was
    /// sent. [`REFUSED`].
    #[must_use]
    pub fn refused(message: impl Into<String>) -> Self {
        Self::new("refused", message, REFUSED)
    }

    /// Something this build does not carry. [`UNSUPPORTED`].
    #[must_use]
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new("unsupported", message, UNSUPPORTED)
    }

    /// The name this failure reports under. It is the same namespace as the
    /// error codes of the library.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        self.kind
    }

    /// The line a person reads.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The code the process exits with.
    #[must_use]
    pub const fn code(&self) -> u8 {
        self.code
    }

    fn new(kind: &'static str, message: impl Into<String>, code: u8) -> Self {
        Self {
            kind,
            message: message.into(),
            code,
        }
    }
}

#[cfg(feature = "transport")]
impl From<crate::Error> for Failure {
    // The kind is the crate's own `Error::code`, so one name travels from the
    // library to the JSON a script reads. The exit code groups those names.
    fn from(error: crate::Error) -> Self {
        let kind = error.code();
        let code = match kind {
            "config" | "configuration" | "local_devices" | "env" => CONFIG,
            // A target the command line names badly is a usage fault; one
            // that matches nothing is the rig answering.
            "target_not_understood" | "ambiguous_target" => USAGE,
            "no_such_target" => UNREACHABLE,
            "no_mode_available" | "mode_unavailable" | "unreachable" | "unknown_device" => {
                UNREACHABLE
            }
            "mode_not_implemented" | "missing_credential" => UNSUPPORTED,
            // A device file that declares nothing for what was asked, and a
            // zone the mode cannot address, refuse the command. Nothing went
            // out either way.
            "no_status_command"
            | "no_verb_command"
            | "no_segment_command"
            | "no_provisioning_command"
            | "no_role_arg"
            | "zone_count_unknown"
            | "zone_count_mismatch"
            | "color_count_mismatch"
            | "zone_list_color_count"
            | "resolution_not_distinct"
            | "zone_out_of_range"
            | "zone_count_unsupported"
            | "zone_mask_unbounded"
            | "native_zones_unreachable"
            | "stream_rate_out_of_range" => REFUSED,
            _ if matches!(error, crate::Error::Codec(_)) => REFUSED,
            _ => INTERNAL,
        };
        Self::new(kind, error.to_string(), code)
    }
}

#[cfg(feature = "transport")]
impl From<crate::select::Error> for Failure {
    fn from(error: crate::select::Error) -> Self {
        Self::from(crate::Error::Select(error))
    }
}

#[cfg(not(feature = "transport"))]
impl From<crate::codec::Error> for Failure {
    fn from(error: crate::codec::Error) -> Self {
        Self::new(error.code(), error.to_string(), REFUSED)
    }
}

#[cfg(feature = "transport")]
impl From<crate::codec::Error> for Failure {
    fn from(error: crate::codec::Error) -> Self {
        Self::from(crate::Error::Codec(error))
    }
}

/// Which form the process streams carry: the records a script reads, or the
/// lines a person reads.
#[derive(Debug, Clone, Copy)]
pub struct Writer {
    json: bool,
}

impl Writer {
    /// A writer for `--json`, or for the text form.
    #[must_use]
    pub const fn new(json: bool) -> Self {
        Self { json }
    }

    /// Write one record to stdout. A subcommand that reports several records
    /// writes one line each, so JSON output is newline-delimited.
    pub fn emit(&self, value: &Value, text: &str) {
        if self.json {
            println!("{value}");
        } else {
            println!("{text}");
        }
    }

    /// Write one record to stderr. A run that carries on after a fault
    /// reports it here, so stdout keeps the records a script reads.
    pub fn warn(&self, value: &Value, text: &str) {
        if self.json {
            eprintln!("{value}");
        } else {
            eprintln!("{text}");
        }
    }

    /// Write the failure to stderr, and answer the code to exit with.
    #[must_use]
    pub fn failure(&self, failure: &Failure) -> ExitCode {
        if self.json {
            eprintln!(
                "{}",
                json!({ "error": { "kind": failure.kind, "message": failure.message } })
            );
        } else {
            eprintln!("error: {}", failure.message);
        }
        ExitCode::from(failure.code)
    }
}
