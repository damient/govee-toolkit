//! What the role verbs accept on one mode, read from the device file alone.

use serde::Serialize;

use crate::codec::catalog::Device;
use crate::codec::{ArgBound, ArgRole, Mode, Role};

/// What the role verbs of [`DeviceHandle`](crate::DeviceHandle) accept on one
/// mode of a device file. A verb that this answers `false` or `None` for
/// fails with [`Error::NoRoleCommand`](crate::Error::NoRoleCommand) or
/// [`Error::NoRoleArg`](crate::Error::NoRoleArg) on that mode.
///
/// It reads the device file, not the device: the firmware can still refuse a
/// value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Supported {
    /// `power` has an entry.
    pub power: bool,
    /// The bounds of `brightness`, or `None` where the mode has no entry or
    /// the argument declares no range.
    pub brightness: Option<[i64; 2]>,
    /// `color` has an entry.
    pub color: bool,
    /// The bounds of `color_temp`, in kelvin, as for `brightness`.
    pub color_temp: Option<[i64; 2]>,
    /// The zones that a paint can address, or `None` where the mode has no
    /// entry that paints zones by mask.
    pub segments: Option<Segments>,
    /// `gradient` has an entry.
    pub gradient: bool,
}

/// The zones of [`Supported::segments`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Segments {
    /// The zone count of the entry.
    pub zones: usize,
    /// The mode arms a segment channel before it paints zones.
    pub armed: bool,
}

impl Supported {
    /// Read the entries of `device` on `mode`.
    #[must_use]
    pub fn of(device: &Device, mode: Mode) -> Self {
        let has = |role| device.entry_for(mode, role).is_some();
        Self {
            power: has(Role::Power),
            brightness: range(device, mode, Role::Brightness, ArgRole::Brightness),
            color: has(Role::Color),
            color_temp: range(device, mode, Role::ColorTemp, ArgRole::ColorTemp),
            segments: match bound(device, mode, Role::SegmentColorMasked, ArgRole::Zones) {
                Some(ArgBound::Zones(zones)) => Some(Segments {
                    zones,
                    armed: has(Role::SegmentEnable),
                }),
                _ => None,
            },
            gradient: has(Role::SegmentGradient),
        }
    }
}

fn bound(device: &Device, mode: Mode, role: Role, arg: ArgRole) -> Option<ArgBound> {
    let (_, command) = device.entry_for(mode, role)?;
    Some(command.args.get(command.arg_for(arg)?)?.bound())
}

fn range(device: &Device, mode: Mode, role: Role, arg: ArgRole) -> Option<[i64; 2]> {
    match bound(device, mode, role, arg)? {
        ArgBound::Range(pair) => Some(pair),
        ArgBound::Zones(_) | ArgBound::MaxLen(_) | ArgBound::None => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::codec::Catalog;

    #[test]
    fn every_file_reads_and_a_light_with_power_has_a_brightness_range() {
        let catalog = Catalog::embedded().unwrap();
        let mut lights = 0;
        for device in catalog.devices() {
            let supported = Supported::of(device, Mode::Lan);
            if supported.power && supported.color {
                lights += 1;
                assert!(supported.brightness.is_some(), "{}", device.sku);
            }
            assert!(
                supported.segments.is_none_or(|segments| segments.zones > 0),
                "{}",
                device.sku
            );
        }
        assert!(lights > 0);
    }
}
