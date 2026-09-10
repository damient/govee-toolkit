//! What reaches the process streams, and with which exit code.
//!
//! The JSON form is the contract: one object per line on stdout, an error
//! object on stderr, and the exit codes below. The text form is for a person
//! and its layout can change at any release.

use std::process::ExitCode;

use govee_toolkit::Error;
use serde_json::{Value, json};

/// Something failed that no other code names.
pub(crate) const INTERNAL: u8 = 1;
/// The command line is wrong. clap reports its own with the same code.
pub(crate) const USAGE: u8 = 2;
/// The configuration cannot be read or cannot be applied.
pub(crate) const CONFIG: u8 = 3;
/// No enabled mode reaches the device. There is no fallback beyond this.
pub(crate) const UNREACHABLE: u8 = 4;
/// The command or an argument is refused. Nothing was sent.
pub(crate) const REFUSED: u8 = 5;
/// The command line names something this build does not carry yet.
pub(crate) const UNSUPPORTED: u8 = 6;

/// A failed run, as the caller sees it.
#[derive(Debug)]
pub(crate) struct Failure {
    kind: &'static str,
    message: String,
    code: u8,
}

impl Failure {
    /// A failure no other constructor names.
    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: "internal",
            message: message.into(),
            code: INTERNAL,
        }
    }

    /// A value on the command line that cannot be read.
    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: "usage",
            message: message.into(),
            code: USAGE,
        }
    }

    /// A command line that names something this build does not carry yet.
    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: "unsupported",
            message: message.into(),
            code: UNSUPPORTED,
        }
    }
}

impl From<Error> for Failure {
    // The kind is the crate's own `Error::code`, so one name travels from the
    // library to the JSON a script reads. The exit code groups those names.
    fn from(error: Error) -> Self {
        let kind = error.code();
        let code = match kind {
            "config" | "configuration" => CONFIG,
            "no_mode_available" | "mode_unavailable" | "unreachable" | "unknown_device" => {
                UNREACHABLE
            }
            "mode_not_implemented" | "missing_credential" => UNSUPPORTED,
            _ if matches!(error, Error::Codec(_)) => REFUSED,
            _ => INTERNAL,
        };
        Self {
            kind,
            message: error.to_string(),
            code,
        }
    }
}

/// Writes the result of one run.
#[derive(Debug)]
pub(crate) struct Writer {
    json: bool,
}

impl Writer {
    /// Build a writer for the requested form.
    pub(crate) fn new(json: bool) -> Self {
        Self { json }
    }

    /// Write one record to stdout, in the form the caller asked for. The
    /// caller builds both: a subcommand that reports several records writes
    /// one line each, so JSON output is newline-delimited.
    pub(crate) fn emit(&self, value: &Value, text: &str) {
        if self.json {
            println!("{value}");
        } else {
            println!("{text}");
        }
    }

    /// Report a failure and answer with the exit code to return.
    pub(crate) fn failure(&self, failure: &Failure) -> ExitCode {
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
