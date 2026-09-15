//! How fast a stream sends, read off the device file.

use crate::codec::{Device, Mode};
use crate::stream::Rate;

pub(crate) fn rate_hz(
    device: &Device,
    sku: &str,
    mode: Mode,
    zones: usize,
    rate: Rate,
    fallback: f64,
) -> f64 {
    match rate {
        Rate::Fixed(hz) => hz,
        Rate::Measured => device
            .measurements
            .clean_hz(mode, u32::try_from(zones).unwrap_or(u32::MAX))
            .unwrap_or_else(|| {
                tracing::warn!(
                    %sku,
                    %mode,
                    fallback_hz = fallback,
                    "no `measurements.frame_rate` for this unit on this mode; streaming at the fallback rate"
                );
                fallback
            }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::codec::Catalog;

    const MASKED: &str = include_str!("../../tests/fixtures/masked-zones.yaml");

    fn catalog() -> Catalog {
        Catalog::from_sources([("masked-zones.yaml", MASKED)]).expect("the device file parses")
    }

    #[test]
    fn the_rate_comes_from_the_row_measured_over_this_mode() {
        let catalog = catalog();
        let device = catalog.device("HTEST3").expect("the SKU resolves");
        let hz = rate_hz(device, "HTEST3", Mode::Ble, 15, Rate::Measured, 10.0);
        assert!((hz - 8.0).abs() < f64::EPSILON);

        // Nothing was measured over `lan`, and a `ble` row does not stand in
        // for it.
        let hz = rate_hz(device, "HTEST3", Mode::Lan, 15, Rate::Measured, 10.0);
        assert!((hz - 10.0).abs() < f64::EPSILON);
    }
}
