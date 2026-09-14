//! The Python binding over `govee-toolkit`.
//!
//! Nothing here is protocol logic: it reads Python values, calls the core and
//! hands back what the core answered. The device files decide what bytes
//! reach the hardware.

mod catalog;
mod config;
mod conv;
mod device;
mod errors;
mod events;
mod govee;
mod stream;
mod types;
mod verbs;

use pyo3::prelude::*;

/// The version of the core this binding was built from.
const CORE_VERSION: &str = "0.8.0";

#[pymodule]
fn _govee_toolkit(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("CORE_VERSION", CORE_VERSION)?;
    module.add("MODES", ("lan", "ble", "cloud"))?;
    errors::register(module)?;
    types::register(module)?;
    catalog::register(module)?;
    config::register(module)?;
    device::register(module)?;
    events::register(module)?;
    govee::register(module)?;
    stream::register(module)?;
    Ok(())
}
