//! The Python binding over `govee-toolkit`.
//!
//! Nothing here is protocol logic: it reads Python values, calls the core and
//! hands back what the core answered. The device files decide what bytes
//! reach the hardware.

mod catalog;
mod config;
mod conv;
mod device;
mod driver;
mod errors;
mod events;
mod govee;
mod stream;
mod types;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

#[pymodule]
fn _govee_toolkit(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("CORE_VERSION", govee_toolkit::VERSION)?;
    module.add(
        "MODES",
        PyTuple::new(module.py(), govee_toolkit::Mode::NAMES)?,
    )?;
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
