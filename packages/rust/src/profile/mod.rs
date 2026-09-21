//! The channel table: what a device answers on each DMX slot.
//!
//! The table is derived. No SKU name and no per-model table exists here: the
//! device file states the capabilities, what `lan` reaches of them and the
//! `role:` each `lan` command claims, and those three decide every channel.
//! See `docs/dmx.md`.
//!
//! The module does no I/O, so the bridge, the catalog task, the site and the
//! bindings read one table. Keep it that way — `tools/check-no-io.sh`
//! re-checks it.

mod channel;
mod reach;
pub mod report;
mod scale;
#[cfg(test)]
mod tests;

use std::fmt;

pub use channel::{Channel, Component, MODE_IDLE_TOP, MODE_RESEND, Slot};
pub use scale::{OFF, Scale, Zero};
use thiserror::Error;

use self::reach::{BRIGHTNESS, COLOR, COLORTEMP, POWER, SEGMENTS, bounds, reaches};
use crate::codec::{ArgRole, Device, Role};

/// How many channels one universe holds.
pub const UNIVERSE: u16 = 512;

/// One layout of the channels a device answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Personality {
    /// One color over the whole device, plus the white temperature. The
    /// white channel stays in the table where `lan` reaches no white
    /// temperature, and drives nothing: one device takes the same 6 channels
    /// as the next one, so a cue file carries between models.
    Full,
    /// One RGB triple for each zone the Govee app exposes.
    Segment,
    /// One RGB triple for each addressable LED measured on the unit.
    Pixel,
}

impl Personality {
    /// Every personality, in the order `docs/dmx.md` lists them.
    pub const ALL: [Self; 3] = [Self::Full, Self::Segment, Self::Pixel];

    /// The personality `name` spells, written the way [`Self::as_str`]
    /// writes it. `None` where no personality carries that name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|personality| personality.as_str() == name)
    }

    /// The name a patch and an operator use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Segment => "segment",
            Self::Pixel => "pixel",
        }
    }

    /// How many channels this personality takes over `zones` zones. The
    /// dimmer and the mode channel are the two every personality carries.
    fn width(self, zones: u32) -> u64 {
        match self {
            Self::Full => 6,
            Self::Segment | Self::Pixel => 2 + 3 * u64::from(zones),
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
    /// Every zone is one addressable LED, so `segment` would lay out the
    /// table `pixel` already lays out. One table carries one name.
    OnePixelPerZone,
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
            Self::OnePixelPerZone => {
                f.write_str("every zone is one addressable LED, so `pixel` lays out the same table")
            }
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

impl Error {
    /// The personality that answered no table.
    #[must_use]
    pub fn personality(&self) -> Personality {
        match self {
            Self::Unserved { personality, .. } | Self::TooWide { personality, .. } => *personality,
        }
    }
}

/// The channel table of one personality on one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    personality: Personality,
    channels: Vec<Channel>,
    zones: usize,
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
        let zones = channels
            .iter()
            .filter_map(|channel| match channel.slot {
                Slot::Zone { index, .. } => usize::try_from(index).ok().map(|index| index + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        Ok(Self {
            personality,
            channels,
            zones,
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

    /// How many zones the table lays out, and 0 where it lays out none.
    #[must_use]
    pub fn zones(&self) -> usize {
        self.zones
    }

    /// How many channels the fixture takes from its start address.
    #[must_use]
    pub fn width(&self) -> u16 {
        u16::try_from(self.channels.len()).unwrap_or(UNIVERSE)
    }
}

/// The channel tables `device` serves over `lan`, in the order
/// [`Personality::ALL`] lists them.
///
/// A table here can still be wider than one universe, which is the error it
/// carries.
#[must_use]
pub fn served(device: &Device) -> Vec<Result<Profile, Error>> {
    Personality::ALL
        .into_iter()
        .map(|personality| Profile::of(device, personality))
        .filter(|table| !matches!(table, Err(Error::Unserved { .. })))
        .collect()
}

/// How many zones the personality lays out, and `0` where it lays out none.
fn zone_count(device: &Device, personality: Personality) -> Result<u32, Missing> {
    if matches!(personality, Personality::Full) {
        return Ok(0);
    }
    if !paints_zones(device) {
        return Err(Missing::Capability(SEGMENTS));
    }
    let measured = if matches!(personality, Personality::Pixel) {
        device
            .capabilities
            .native_pixels()
            .ok_or(Missing::NativePixels)?
    } else {
        let zones = device.capabilities.segment_count().ok_or(Missing::Zones)?;
        if device.capabilities.native_pixels() == Some(zones) {
            return Err(Missing::OnePixelPerZone);
        }
        zones
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
    let mut channels = vec![dimmer(device)?, Channel::plain(2, Slot::Mode)];
    match personality {
        Personality::Full => {
            color(device, &mut channels)?;
            channels.push(white(device, next(&channels))?);
        }
        Personality::Segment | Personality::Pixel => {
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
        .and_then(|range| Scale::new(range, Zero::Off))
        .ok_or(Missing::Bounds(BRIGHTNESS))?;
    Ok(Channel::scaled(1, Slot::Dimmer, scale))
}

fn color(device: &Device, channels: &mut Vec<Channel>) -> Result<(), Missing> {
    if !reaches(device, COLOR, Role::Color) {
        return Err(Missing::Capability(COLOR));
    }
    for (component, arg) in [
        (Component::Red, ArgRole::Red),
        (Component::Green, ArgRole::Green),
        (Component::Blue, ArgRole::Blue),
    ] {
        let scale = bounds(device, Role::Color, arg)
            .and_then(Scale::bytes)
            .ok_or(Missing::Bounds(COLOR))?;
        channels.push(Channel::scaled(
            next(channels),
            Slot::Color(component),
            scale,
        ));
    }
    Ok(())
}

/// The white channel, scaled where `lan` reaches the white temperature.
///
/// A device `lan` reaches no white temperature on keeps the channel and
/// drives nothing from it, so every `full` fixture takes 6 channels.
fn white(device: &Device, offset: u16) -> Result<Channel, Missing> {
    if !reaches(device, COLORTEMP, Role::ColorTemp) {
        return Ok(Channel::plain(offset, Slot::WhiteTemp));
    }
    let scale = bounds(device, Role::ColorTemp, ArgRole::ColorTemp)
        .and_then(|range| Scale::new(range, Zero::Off))
        .ok_or(Missing::Bounds(COLORTEMP))?;
    Ok(Channel::scaled(offset, Slot::WhiteTemp, scale))
}
