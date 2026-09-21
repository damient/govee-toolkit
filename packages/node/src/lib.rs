//! The Node.js binding over `govee-toolkit`.
//!
//! Nothing here is protocol logic: it reads JavaScript values, calls the core
//! and hands back what the core answered.
//!
//! The two allows below are what the napi macro accepts, not a choice made
//! here: an argument arrives as an owned `String`, and the environment by
//! reference.
#![allow(clippy::needless_pass_by_value, clippy::trivially_copy_pass_by_ref)]

mod catalog;
mod config;
mod conv;
mod device;
mod errors;
mod events;
mod govee;
mod promise;
mod stream;
mod types;
mod verbs;

use napi_derive::napi;

/// The version of this binding.
#[napi]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version of the core this binding was built from.
#[napi]
pub const CORE_VERSION: &str = govee_toolkit::VERSION;

/// Every mode name the core knows. A build carries a transport for those its
/// features name — `Govee.modes()` answers which.
#[napi]
#[must_use]
pub fn modes() -> Vec<&'static str> {
    govee_toolkit::Mode::NAMES.to_vec()
}
