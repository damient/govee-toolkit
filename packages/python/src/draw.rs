//! The arguments of `DeviceHandle.draw()`, read before the call.

use govee_toolkit::codec::{Role, Supplied};
use govee_toolkit::input::Value;
use govee_toolkit::{DeviceHandle, Drawing, Error, Resolution, Served, input};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::conv::{self, read};

pub(crate) struct Drawn {
    cells: Option<Vec<u16>>,
    colors: Vec<[u8; 3]>,
    resolution: Resolution,
    supplied: Vec<(String, Supplied)>,
}

impl Drawn {
    pub(crate) fn read(
        colors: &Bound<'_, PyAny>,
        cells: Option<&Bound<'_, PyAny>>,
        resolution: Option<&Bound<'_, PyAny>>,
        args: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let cells = match conv::optional(cells)? {
            Value::Null => None,
            listed => Some(read(input::zones(&listed))?),
        };
        Ok(Self {
            cells,
            colors: conv::colors(colors)?,
            resolution: read(input::resolution(&conv::optional(resolution)?))?,
            supplied: conv::args(args)?,
        })
    }

    /// Reads the arguments under the types of the entry that claims
    /// `role: draw` on the mode that serves the device now.
    pub(crate) async fn send(self, device: DeviceHandle<'_>) -> Result<Served, Error> {
        let args = device.resolve()?.role_args(Role::Draw, self.supplied)?;
        device
            .draw(&Drawing {
                cells: self.cells.as_deref(),
                colors: &self.colors,
                resolution: self.resolution,
                args: &args,
            })
            .await
    }
}
