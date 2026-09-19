//! The subcommands, and what they report.
//!
//! The JSON form is the contract: one object per line on stdout, an error
//! object on stderr, and the exit codes below. The text form is for a person
//! and its layout can change at any release.

pub(crate) mod identify;
pub(crate) mod observe;
pub(crate) mod patch;
pub(crate) mod profile;
pub(crate) mod rig;
pub(crate) mod run;

use std::process::ExitCode;

use serde_json::json;

/// Something failed that no other code names.
pub(crate) const INTERNAL: u8 = 1;
/// The command line is wrong, or it names something no device file carries.
/// clap reports its own with the same code.
pub(crate) const USAGE: u8 = 2;
/// The configuration or the patch cannot be read, or cannot be applied.
pub(crate) const CONFIG: u8 = 3;
/// A device the patch names did not answer, or enables no `lan` mode.
pub(crate) const UNREACHABLE: u8 = 4;
/// The device serves no such channel table. Nothing was printed.
pub(crate) const REFUSED: u8 = 5;

/// What failed, and with which exit code.
#[derive(Debug)]
pub(crate) struct Failure {
    message: String,
    code: u8,
}

impl Failure {
    pub(crate) fn new(message: impl Into<String>, code: u8) -> Self {
        Self {
            message: message.into(),
            code,
        }
    }

    /// Write the failure to stderr, and answer the code to exit with.
    pub(crate) fn report(&self, as_json: bool) -> ExitCode {
        if as_json {
            eprintln!("{}", json!({ "error": { "message": self.message } }));
        } else {
            eprintln!("error: {}", self.message);
        }
        ExitCode::from(self.code)
    }
}
