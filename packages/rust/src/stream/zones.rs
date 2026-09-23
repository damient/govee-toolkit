//! How many zones a stream carries, read off the device file.

use crate::codec::{Device, Mode, Spread};
use crate::error::{Error, Result};
use crate::stream::Resolution;
use crate::stream::reach::ceiling;
use crate::stream::resolve::Painter;

/// Where the mode paints fewer zones than the device file states, `App` falls
/// to what the mode carries, and every other resolution is refused.
///
/// A count of 0 is refused: the codec refuses its frames, and nothing reads
/// that refusal on a stream.
pub(super) fn zone_count(
    device: &Device,
    mode: Mode,
    painter: &Painter,
    resolution: Resolution,
) -> Result<(usize, Option<Spread>)> {
    if let (Painter::Masked { .. }, Resolution::Native) = (painter, resolution) {
        return Err(Error::NativeZonesUnreachable {
            sku: device.sku.clone(),
            mode,
        });
    }
    let count = match resolution {
        Resolution::App => device.capabilities.segment_count().unwrap_or(0),
        Resolution::Native => device.capabilities.native_pixels().unwrap_or(0),
        Resolution::Exact(n) => u32::from(n),
        Resolution::Groups => return groups(device, mode, painter),
    };
    if count == 0 {
        return Err(Error::ZoneCountUnknown {
            sku: device.sku.clone(),
        });
    }
    // `App` and `Native` come from the device file, which states that they
    // render.
    if let Resolution::Exact(_) = resolution
        && let Some(rendered) = device.measurements.renders_as(count)
        && rendered != count
    {
        return Err(Error::ResolutionNotDistinct {
            sku: device.sku.clone(),
            zones: usize::try_from(count).unwrap_or(usize::MAX),
            rendered,
            changepoints: device.measurements.resolution_changepoints.clone(),
        });
    }
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    let Some(limit) = ceiling(device, mode, painter) else {
        return Ok((count, None));
    };
    if count <= limit {
        return Ok((count, None));
    }
    if let Resolution::App = resolution {
        return Ok((limit, None));
    }
    Err(Error::ZoneCountUnsupported {
        sku: device.sku.clone(),
        mode,
        zones: count,
        limit,
    })
}

fn groups(device: &Device, mode: Mode, painter: &Painter) -> Result<(usize, Option<Spread>)> {
    let unknown = || Error::ZoneCountUnknown {
        sku: device.sku.clone(),
    };
    let groups = device.capabilities.segment_groups().ok_or_else(unknown)?;
    let (width, spread) = match painter {
        Painter::Masked { .. } => (groups, None),
        Painter::Whole { .. } => {
            let pixels = device.capabilities.native_pixels().ok_or_else(unknown)?;
            let spread = Spread::new(groups, pixels).ok_or_else(unknown)?;
            (pixels, (groups != pixels).then_some(spread))
        }
    };
    let width = usize::try_from(width).unwrap_or(usize::MAX);
    if let Some(limit) = ceiling(device, mode, painter)
        && width > limit
    {
        return Err(Error::ZoneCountUnsupported {
            sku: device.sku.clone(),
            mode,
            zones: width,
            limit,
        });
    }
    Ok((usize::try_from(groups).unwrap_or(usize::MAX), spread))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use crate::codec::{Catalog, Mode};
    use crate::error::Result;
    use crate::stream::resolve::{Plan, plan};
    use crate::stream::{Resolution, StreamOptions};

    const MASKED: &str = include_str!("../../tests/fixtures/masked-zones.yaml");

    fn planned_on(
        catalog: &Catalog,
        sku: &str,
        mode: Mode,
        resolution: Resolution,
    ) -> Result<Plan> {
        let device = catalog.device(sku).expect("the SKU resolves");
        plan(
            device,
            mode,
            &StreamOptions {
                resolution,
                ..StreamOptions::default()
            },
        )
    }

    fn planned(resolution: Resolution) -> Result<Plan> {
        let catalog =
            Catalog::from_sources([("masked-zones.yaml", MASKED)]).expect("the device file parses");
        planned_on(&catalog, "HTEST3", Mode::Ble, resolution)
    }

    #[test]
    fn native_resolution_is_refused_rather_than_masked() {
        // The firmware drops in silence the mask bits past the last zone.
        let error = planned(Resolution::Native).expect_err("a mask reaches no pixel");
        assert_eq!(error.code(), "native_zones_unreachable");
    }

    #[test]
    fn more_zones_than_the_mask_names_are_refused() {
        let error = planned(Resolution::Exact(20)).expect_err("the mask names 15");
        assert_eq!(error.code(), "zone_count_unsupported");
        assert_eq!(planned(Resolution::Exact(15)).unwrap().zones, 15);
    }

    #[test]
    fn a_mask_names_the_groups_as_its_zones() {
        let catalog = Catalog::embedded().expect("the catalog parses");
        let plan = planned_on(&catalog, "H6022", Mode::Ble, Resolution::Groups).unwrap();
        assert_eq!(plan.zones, 15);
        assert_eq!(plan.width(), 15);
        assert!(plan.spread.is_none());
    }

    #[test]
    fn a_whole_frame_spreads_the_groups_over_every_led() {
        let catalog = Catalog::embedded().expect("the catalog parses");
        for (sku, pixels) in [("H6022", 132), ("H61A0", 42)] {
            let plan = planned_on(&catalog, sku, Mode::Lan, Resolution::Groups).unwrap();
            assert_eq!(plan.zones, 15, "{sku}");
            assert_eq!(plan.width(), pixels, "{sku}");
            assert!(plan.spread.is_some(), "{sku}");
        }
    }

    #[test]
    fn a_file_that_declares_no_groups_refuses_them() {
        let error = planned(Resolution::Groups).expect_err("the file declares no groups");
        assert_eq!(error.code(), "zone_count_unknown");
    }

    #[test]
    fn a_whole_frame_narrower_than_the_leds_refuses_the_groups() {
        let file = include_str!("../../tests/fixtures/narrow-colors.yaml")
            .replace("native_pixels: 40", "native_pixels: 40\n    groups: 8");
        let catalog =
            Catalog::from_sources([("narrow-colors.yaml", file.as_str())]).expect("parses");
        let error = planned_on(&catalog, "HTEST6", Mode::Lan, Resolution::Groups)
            .expect_err("the frame carries 20");
        assert_eq!(error.code(), "zone_count_unsupported");
    }
}
