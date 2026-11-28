//! The scene library of a model, and the scene a call names.

use govee_toolkit::{Error, OwnedDevice, Scene as CoreScene, Served, input, scenes};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::conv::{self, to_py};
use crate::errors::map;

/// One scene of a model's library.
#[pyclass(frozen, skip_from_py_object, module = "govee_toolkit", name = "Scene")]
#[derive(Debug, Clone)]
pub(crate) struct Scene {
    inner: CoreScene,
}

#[pymethods]
impl Scene {
    /// The name in the library. A scene with more than one effect gives each
    /// effect its own entry: `Aurora A`, `Aurora B`.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// The category in the library.
    #[getter]
    fn category(&self) -> String {
        self.inner.category.clone()
    }

    /// The code that the device reports back for the scene.
    #[getter]
    fn code(&self) -> u16 {
        self.inner.code
    }

    /// The type of the parameters. A device file entry declares the types
    /// that it plays.
    #[getter]
    fn scene_type(&self) -> i64 {
        self.inner.scene_type
    }

    /// The parameters. Empty for a scene that the firmware stores.
    #[getter]
    fn param<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.param)
    }

    /// The record that every surface prints, with `param` in hexadecimal.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "Scene(name='{}', code={})",
            self.inner.name, self.inner.code
        )
    }
}

fn wrap(library: Result<Vec<CoreScene>, scenes::Error>) -> PyResult<Vec<Scene>> {
    let library = map(library.map_err(Error::from))?;
    Ok(library.into_iter().map(|inner| Scene { inner }).collect())
}

/// Fetch the scene library of a model over `cloud`, and store it in the
/// cache. Needs no device and no enabled mode.
#[pyfunction]
fn fetch_scenes(py: Python<'_>, sku: String) -> PyResult<Bound<'_, PyAny>> {
    future_into_py(py, async move { wrap(scenes::fetch(&sku).await) })
}

/// The scene library that `fetch_scenes()` stored for a model. Sends no
/// request.
#[pyfunction]
fn load_scenes(sku: &str) -> PyResult<Vec<Scene>> {
    wrap(scenes::load(sku))
}

/// What `DeviceHandle.scene()` plays.
pub(crate) enum Chosen {
    Scene(CoreScene),
    /// A name, a shared name or a code, found in the cached library.
    Query(String),
}

impl Chosen {
    pub(crate) async fn play(self, device: OwnedDevice) -> Result<Served, Error> {
        match self {
            Self::Scene(scene) => device.handle().scene(&scene).await,
            Self::Query(query) => device.handle().scene_named(&query).await,
        }
    }
}

pub(crate) fn chosen(object: &Bound<'_, PyAny>) -> PyResult<Chosen> {
    if let Ok(scene) = object.cast::<Scene>() {
        return Ok(Chosen::Scene(scene.get().inner.clone()));
    }
    query(object).map(Chosen::Query)
}

pub(crate) fn query(object: &Bound<'_, PyAny>) -> PyResult<String> {
    conv::read(input::scene(&conv::value(object)?))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Scene>()?;
    module.add_function(wrap_pyfunction!(fetch_scenes, module)?)?;
    module.add_function(wrap_pyfunction!(load_scenes, module)?)
}
