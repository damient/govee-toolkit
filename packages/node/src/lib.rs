//! The Node.js binding over `govee-toolkit`.
//!
//! Nothing here is protocol logic: it reads JavaScript values, calls the core
//! and hands back what the core answered. The device files decide what bytes
//! reach the hardware.
//!
//! napi reads every argument off a JavaScript value, so a string argument is
//! an owned `String` and the environment arrives by reference. Both are what
//! the macro accepts, not a choice made here.
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
