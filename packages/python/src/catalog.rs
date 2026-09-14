//! What `devices/*.yaml` declares, as the SDK reads it.

use govee_toolkit::Catalog as CoreCatalog;
use pyo3::prelude::*;

use crate::conv::to_py;
use crate::errors::map;

/// Every device the build knows. Reads no hardware.
#[pyclass(frozen, from_py_object, module = "govee_toolkit", name = "Catalog")]
#[derive(Debug, Clone)]
pub(crate) struct Catalog {
    pub(crate) inner: CoreCatalog,
}

#[pymethods]
impl Catalog {
    /// The catalog compiled into this build.
    #[staticmethod]
    fn embedded() -> PyResult<Self> {
        Ok(Self {
            inner: map(CoreCatalog::embedded().map_err(Into::into))?,
        })
    }

    /// Every SKU that resolves, verified aliases included.
    fn skus(&self) -> Vec<String> {
        self.inner.skus().map(ToOwned::to_owned).collect()
    }

    /// Whether a SKU resolves.
    fn has(&self, sku: &str) -> bool {
        self.inner.device(sku).is_ok()
    }

    /// One device file, with every `include:` and every override applied.
    ///
    /// Raises `CodecError` with the code `unknown_sku` when nothing declares
    /// it.
    fn device(&self, py: Python<'_>, sku: &str) -> PyResult<Py<PyAny>> {
        let device = map(self.inner.device(sku).map_err(Into::into))?;
        to_py(py, device)
    }

    fn __len__(&self) -> usize {
        self.inner.devices().count()
    }

    fn __repr__(&self) -> String {
        format!("Catalog(devices={})", self.inner.devices().count())
    }
}

/// Add the catalog to the module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Catalog>()
}
