//! The channel table: what a device answers on each DMX slot.
//!
//! The table is derived. No SKU name and no per-model table exists here: the
//! device file states the capabilities, what `lan` reaches of them and the
//! `role:` each `lan` command claims, and those three decide every channel.
//! See `docs/dmx.md`.
//!
//! The module does no I/O, so it can move into `govee-toolkit` where the site
//! and the bindings want the same table. Keep it that way —
//! `tools/check-no-io.sh` re-checks it.

mod channel;
mod reach;
mod scale;
#[cfg(test)]
mod tests;

use std::fmt;

pub use channel::{Channel, Component, Slot};
use govee_toolkit::codec::{ArgRole, Device, Role};
pub use scale::{OFF, Scale};
use thiserror::Error;

use self::reach::{BRIGHTNESS, COLOR, COLORTEMP, POWER, SEGMENTS, bounds, reaches};

/// How many channels one universe holds.
pub const UNIVERSE: u16 = 512;

/// One layout of the channels a device answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Personality {
    /// A dimmer and one color over the whole device.
    Basic,
    /// The same, plus the white temperature and the control channel.
    Full,
    /// A dimmer, then one RGB triple for each zone the Govee app exposes.
    Pixel,
    /// A dimmer, then one RGB triple for each addressable LED measured on the
    /// unit.
    PixelNative,
}

impl Personality {
    /// Every personality, in the order `docs/dmx.md` lists them.
    pub const ALL: [Self; 4] = [Self::Basic, Self::Full, Self::Pixel, Self::PixelNative];

    /// The name a patch and an operator use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Full => "full",
            Self::Pixel => "pixel",
            Self::PixelNative => "pixel-native",
        }
    }

    /// How many channels this personality takes over `zones` zones.
    fn width(self, zones: u32) -> u64 {
        match self {
            Self::Basic => 4,
            Self::Full => 6,
            Self::Pixel | Self::PixelNative => 1 + 3 * u64::from(zones),
        }
    }
}

impl fmt::Display for Personality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a device file gives the bridge no way to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// `lan` reaches the capability, or claims a command for it, through
    /// nothing.
    Capability(&'static str),
    /// The command that drives it declares no pair to scale into.
    Bounds(&'static str),
    /// `capabilities.segments` counts no zone.
    Zones,
    /// Nobody measured `capabilities.segments.native_pixels`. It is never
    /// extrapolated from the zone count.
    NativePixels,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capability(name) => write!(f, "`lan` drives no `{name}`"),
            Self::Bounds(name) => {
                write!(
                    f,
                    "the `lan` command for `{name}` declares no pair to scale into"
                )
            }
            Self::Zones => f.write_str("`capabilities.segments` counts no zone"),
            Self::NativePixels => {
                f.write_str("nobody measured `capabilities.segments.native_pixels`")
            }
        }
    }
}

/// Why a device answers no channel table.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// The device file gives the bridge no way to drive one channel of the
    /// personality.
    #[error("{sku} serves no `{personality}` personality: {missing}")]
    Unserved {
        /// The SKU asked for.
        sku: String,
        /// The personality asked for.
        personality: Personality,
        /// What the device file does not give.
        missing: Missing,
    },
    /// The table is wider than one universe. It is never truncated to fit.
    #[error(
        "`{personality}` on {sku} takes {channels} channels, over the {} a universe holds",
        UNIVERSE
    )]
    TooWide {
        /// The SKU asked for.
        sku: String,
        /// The personality asked for.
        personality: Personality,
        /// How many channels it takes.
        channels: u64,
    },
}

/// The channel table of one personality on one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    personality: Personality,
    channels: Vec<Channel>,
}

impl Profile {
    /// The table `device` answers to over `lan`, under `personality`.
    ///
    /// # Errors
    ///
    /// [`Error::Unserved`] where the device file gives the bridge no way to
    /// drive a channel of the personality, and [`Error::TooWide`] where the
    /// table is wider than one universe.
    pub fn of(device: &Device, personality: Personality) -> Result<Self, Error> {
        let zones = zone_count(device, personality).map_err(|missing| Error::Unserved {
            sku: device.sku.clone(),
            personality,
            missing,
        })?;
        let width = personality.width(zones);
        if width > u64::from(UNIVERSE) {
            return Err(Error::TooWide {
                sku: device.sku.clone(),
                personality,
                channels: width,
            });
        }
        let channels = channels(device, personality, zones).map_err(|missing| Error::Unserved {
            sku: device.sku.clone(),
            personality,
            missing,
        })?;
        Ok(Self {
            personality,
            channels,
        })
    }

    /// The personality this table lays out.
    #[must_use]
    pub fn personality(&self) -> Personality {
        self.personality
    }

    /// Every channel, in offset order.
    #[must_use]
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    /// How many channels the fixture takes from its start address.
    #[must_use]
    pub fn width(&self) -> u16 {
        u16::try_from(self.channels.len()).unwrap_or(UNIVERSE)
    }
}

/// The personalities `device` serves over `lan`, in the order
/// [`Personality::ALL`] lists them.
///
/// A personality here can still be wider than one universe. [`Profile::of`]
/// is what reports that.
#[must_use]
pub fn served(device: &Device) -> Vec<Personality> {
    Personality::ALL
        .into_iter()
        .filter(|personality| {
            !matches!(
                Profile::of(device, *personality),
                Err(Error::Unserved { .. })
            )
        })
        .collect()
}

/// How many zones the personality lays out, and `0` where it lays out none.
fn zone_count(device: &Device, personality: Personality) -> Result<u32, Missing> {
    if matches!(personality, Personality::Basic | Personality::Full) {
        return Ok(0);
    }
    if !paints_zones(device) {
        return Err(Missing::Capability(SEGMENTS));
    }
    let measured = match personality {
        Personality::PixelNative => device
            .capabilities
            .native_pixels()
            .ok_or(Missing::NativePixels)?,
        _ => device.capabilities.segment_count().ok_or(Missing::Zones)?,
    };
    if measured == 0 {
        return Err(Missing::Zones);
    }
    Ok(measured)
}

/// Whether `lan` paints zones, in one frame or through a mask.
fn paints_zones(device: &Device) -> bool {
    reaches(device, SEGMENTS, Role::SegmentColor)
        || reaches(device, SEGMENTS, Role::SegmentColorMasked)
}

fn channels(
    device: &Device,
    personality: Personality,
    zones: u32,
) -> Result<Vec<Channel>, Missing> {
    let mut channels = vec![dimmer(device)?];
    match personality {
        Personality::Basic => color(device, &mut channels)?,
        Personality::Full => {
            color(device, &mut channels)?;
            channels.push(white(device, next(&channels))?);
            channels.push(Channel::plain(next(&channels), Slot::Control));
        }
        Personality::Pixel | Personality::PixelNative => {
            for index in 0..zones {
                for component in [Component::Red, Component::Green, Component::Blue] {
                    let slot = Slot::Zone { index, component };
                    channels.push(Channel::plain(next(&channels), slot));
                }
            }
        }
    }
    Ok(channels)
}

/// The offset the next channel takes. Offsets count from 1, the way a desk
/// counts.
fn next(channels: &[Channel]) -> u16 {
    u16::try_from(channels.len() + 1).unwrap_or(UNIVERSE)
}

fn dimmer(device: &Device) -> Result<Channel, Missing> {
    if !reaches(device, POWER, Role::Power) {
        return Err(Missing::Capability(POWER));
    }
    if !reaches(device, BRIGHTNESS, Role::Brightness) {
        return Err(Missing::Capability(BRIGHTNESS));
    }
    let scale = bounds(device, Role::Brightness, ArgRole::Brightness)
        .and_then(Scale::new)
        .ok_or(Missing::Bounds(BRIGHTNESS))?;
    Ok(Channel::scaled(1, Slot::Dimmer, scale))
}

fn color(device: &Device, channels: &mut Vec<Channel>) -> Result<(), Missing> {
    if !reaches(device, COLOR, Role::Color) {
        return Err(Missing::Capability(COLOR));
    }
    for component in [Component::Red, Component::Green, Component::Blue] {
        channels.push(Channel::plain(next(channels), Slot::Color(component)));
    }
    Ok(())
}

fn white(device: &Device, offset: u16) -> Result<Channel, Missing> {
    if !reaches(device, COLORTEMP, Role::ColorTemp) {
        return Err(Missing::Capability(COLORTEMP));
    }
    let scale = bounds(device, Role::ColorTemp, ArgRole::ColorTemp)
        .and_then(Scale::new)
        .ok_or(Missing::Bounds(COLORTEMP))?;
    Ok(Channel::scaled(offset, Slot::WhiteTemp, scale))
}
