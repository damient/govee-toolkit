use crate::codec::{ArgSpec, Device, Mode};
use crate::stream::resolve::{Painter, painter};

/// What one mode's segment channel paints on one device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reach {
    /// How many zones one paint states over this mode.
    pub zones: usize,
    /// Whether the mode reaches every addressable LED of the unit.
    pub native: bool,
}

/// What the mode's painting command reaches, read off the device file. `None`
/// where the mode paints no zones, or where the file bounds the paint by
/// nothing.
#[must_use]
pub fn reach(device: &Device, mode: Mode) -> Option<Reach> {
    let painter = painter(device, mode, false).ok()?;
    let zones = ceiling(device, mode, &painter)?;
    let native = match painter {
        Painter::Whole { .. } => device
            .capabilities
            .native_pixels()
            .and_then(|pixels| usize::try_from(pixels).ok())
            .is_some_and(|pixels| pixels <= zones),
        Painter::Masked { .. } => false,
    };
    Some(Reach { zones, native })
}

/// How many zones one paint carries over `mode`.
///
/// The bound of the color list where one frame states every zone, and the
/// bound of the mask where the frame names the zones it paints. `None` where
/// the file bounds the color list by nothing, and the paint then carries what
/// the caller states.
pub(crate) fn ceiling(device: &Device, mode: Mode, painter: &Painter) -> Option<usize> {
    match painter {
        Painter::Whole {
            command, colors, ..
        } => color_limit(device, mode, command, colors),
        Painter::Masked { limit, .. } => Some(*limit),
    }
}

fn color_limit(device: &Device, mode: Mode, command: &str, arg: &str) -> Option<usize> {
    match device.commands.get(mode).get(command)?.args.get(arg)? {
        ArgSpec::RgbList { max_len, .. } => *max_len,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::codec::Catalog;
    use crate::stream::resolve::plan;
    use crate::stream::{Resolution, StreamOptions};

    const NARROW_MASK: &str = include_str!("../../tests/fixtures/narrow-mask.yaml");
    const NARROW_COLORS: &str = include_str!("../../tests/fixtures/narrow-colors.yaml");

    #[test]
    fn a_whole_frame_mode_reaches_every_led_of_this_unit() {
        let catalog = Catalog::embedded().expect("the catalog parses");
        let device = catalog.device("H61A0").expect("the SKU resolves");

        let lan = reach(device, Mode::Lan).expect("lan paints");
        assert_eq!(lan.zones, 255);
        assert!(lan.native);
    }

    #[test]
    fn a_masked_mode_reaches_what_its_mask_names_and_no_led() {
        let catalog = Catalog::embedded().expect("the catalog parses");
        let device = catalog.device("H61A0").expect("the SKU resolves");

        let ble = reach(device, Mode::Ble).expect("ble paints");
        assert_eq!(ble.zones, 15);
        assert!(!ble.native);
    }

    #[test]
    fn a_mode_that_paints_nothing_reaches_nothing() {
        let catalog = Catalog::embedded().expect("the catalog parses");
        let device = catalog.device("H6114").expect("the SKU resolves");
        assert_eq!(reach(device, Mode::Lan), None);
    }

    #[test]
    fn the_app_count_falls_to_the_width_of_the_mask() {
        let catalog = Catalog::from_sources([("narrow-mask.yaml", NARROW_MASK)])
            .expect("the device file parses");
        let device = catalog.device("HTEST5").expect("the SKU resolves");
        let planned = |resolution| {
            plan(
                device,
                Mode::Ble,
                &StreamOptions {
                    resolution,
                    ..StreamOptions::default()
                },
            )
        };

        assert_eq!(planned(Resolution::App).unwrap().zones, 15);

        let error = planned(Resolution::Exact(132)).expect_err("the mask names 15");
        assert_eq!(error.code(), "zone_count_unsupported");
    }

    #[test]
    fn a_whole_frame_mode_carries_no_more_than_its_color_list() {
        let catalog = Catalog::from_sources([("narrow-colors.yaml", NARROW_COLORS)])
            .expect("the device file parses");
        let device = catalog.device("HTEST6").expect("the SKU resolves");
        let planned = |resolution| {
            plan(
                device,
                Mode::Lan,
                &StreamOptions {
                    resolution,
                    ..StreamOptions::default()
                },
            )
        };

        assert_eq!(planned(Resolution::App).unwrap().zones, 20);
        assert_eq!(planned(Resolution::Exact(20)).unwrap().zones, 20);

        for resolution in [Resolution::Native, Resolution::Exact(24)] {
            let error = planned(resolution).expect_err("the frame carries 20");
            assert_eq!(error.code(), "zone_count_unsupported");
        }
    }
}
