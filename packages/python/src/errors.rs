//! The exceptions the module raises.
//!
//! Every one carries `code`, the stable identifier the core gives the failure.
//! Match on that rather than on the message: the message is written for a
//! person and can change.

use govee_toolkit::{Category, Error};
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::{Bound, create_exception};

create_exception!(
    _govee_toolkit,
    GoveeError,
    PyException,
    "Anything that went wrong between a call and the bytes on the wire."
);
create_exception!(
    _govee_toolkit,
    CodecError,
    GoveeError,
    "An unknown SKU, an unknown command, or an argument out of range. Nothing was sent."
);
create_exception!(
    _govee_toolkit,
    TransportError,
    GoveeError,
    "A mode failed to carry the command, or nothing answered in time."
);
create_exception!(
    _govee_toolkit,
    ConfigError,
    GoveeError,
    "The configuration could not be read, or it enables something that cannot work."
);

pub(crate) fn to_py(error: &Error) -> PyErr {
    let message = error.to_string();
    let raised = match error.category() {
        Category::Codec => CodecError::new_err(message),
        Category::Transport => TransportError::new_err(message),
        Category::Config => ConfigError::new_err(message),
        // `Category` is `#[non_exhaustive]`: a family this build does not know
        // reaches Python as the base class, never as the wrong subclass.
        _ => GoveeError::new_err(message),
    };
    // An exception instance carries a `__dict__`, so the code reaches Python
    // as an attribute rather than as a second argument that `str(e)` prints.
    Python::attach(|py| {
        let _ = raised.value(py).setattr("code", error.code());
    });
    raised
}

pub(crate) fn map<T>(result: Result<T, Error>) -> PyResult<T> {
    result.map_err(|error| to_py(&error))
}

/// A value the binding itself refuses, before the core sees it.
pub(crate) fn value_error(message: impl Into<String>) -> PyErr {
    PyValueError::new_err(message.into())
}

/// The base class carries an empty `code`, so the attribute is there on an
/// exception a caller builds as well as on one the binding raises.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let base = module.py().get_type::<GoveeError>();
    base.setattr("code", "")?;
    module.add("GoveeError", base)?;
    module.add("CodecError", module.py().get_type::<CodecError>())?;
    module.add("TransportError", module.py().get_type::<TransportError>())?;
    module.add("ConfigError", module.py().get_type::<ConfigError>())?;
    Ok(())
}
