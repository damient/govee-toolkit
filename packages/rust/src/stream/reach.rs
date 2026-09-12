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

/// What the mode's painting command reaches, read off the device file. The
/// zone count is the bound of the color list where one frame states every
/// zone, and the bound of the mask where the frame names the zones it paints.
#[must_use]
pub fn reach(device: &Device, mode: Mode) -> Option<Reach> {
    let native = device.capabilities.native_pixels();
    match painter(device, mode, false).ok()? {
        Painter::Whole {
            command, colors, ..
        } => {
            let zones = color_limit(device, mode, &command, &colors)?;
            Some(Reach {
                zones,
                native: native
                    .and_then(|pixels| usize::try_from(pixels).ok())
                    .is_some_and(|pixels| pixels <= zones),
            })
        }
        Painter::Masked { limit, .. } => Some(Reach {
            zones: limit,
            native: false,
        }),
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
}
