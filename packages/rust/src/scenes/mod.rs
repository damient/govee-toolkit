//! The scenes that a model plays, from the library that the cloud serves per
//! model.
//!
//! [`fetch`] stores that library in the cache folder. [`load`] reads it back,
//! so a scene goes out over any mode with no network call at send time. The
//! library is the user's own copy: no scene ships with this crate.

mod cache;
mod error;
#[cfg(feature = "cloud")]
mod fetch;
mod library;

pub use cache::{load, store};
pub use error::Error;
#[cfg(feature = "cloud")]
pub use fetch::fetch;
pub use library::{Scene, find, parse};
