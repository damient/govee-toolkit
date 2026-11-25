//! Paint zones.
//!
//! A `role: segment_color_masked` command carries one color and the zones that
//! take it, and leaves the other zones alone. A paint of every zone sends one
//! such frame per distinct color.

use super::arg_for;
use crate::codec::catalog::Device;
use crate::codec::{ArgRole, Args, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;
use crate::stream::resolve::{Painter, Plan, Purpose, plan};
use crate::stream::{Resolution, StreamOptions, paint};

/// One painting of a device's zones.
#[derive(Debug, Clone, Copy)]
pub struct Paint<'a> {
    /// The zones to paint, zero-based, leaving every other zone alone. `None`
    /// paints every zone. A list needs `role: segment_color_masked`, and takes
    /// one color.
    pub zones: Option<&'a [u16]>,
    /// One color paints every zone. A longer list states one zone each, and
    /// must be as long as the zone count [`Paint::resolution`] resolves.
    pub colors: &'a [[u8; 3]],
    /// How many zones the frame states. Read where the paint names no zone
    /// list; a mode that paints by mask states what its mask names instead.
    pub resolution: Resolution,
    /// Ask the firmware to interpolate between zones, and to wrap from the
    /// last zone back to the first.
    pub gradient: bool,
}

/// A [`Paint`] that owns its zones and colors, for a call that outlives the
/// values it was read from. [`crate::input::painting`] reads one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Painting {
    /// [`Paint::zones`].
    pub zones: Option<Vec<u16>>,
    /// [`Paint::colors`].
    pub colors: Vec<[u8; 3]>,
    /// [`Paint::resolution`].
    pub resolution: Resolution,
    /// [`Paint::gradient`].
    pub gradient: bool,
}

impl Painting {
    /// The paint to send.
    #[must_use]
    pub fn paint(&self) -> Paint<'_> {
        Paint {
            zones: self.zones.as_deref(),
            colors: &self.colors,
            resolution: self.resolution,
            gradient: self.gradient,
        }
    }
}

impl DeviceHandle<'_> {
    /// Paint zones.
    ///
    /// Takes `role: segment_color_masked`: one frame per distinct color,
    /// each naming the zones of the mask that take it. Where the mode marks
    /// `role: segment_gradient`, the setting goes first. Where the frames
    /// share an envelope, one datagram carries all of them
    /// ([`crate::codec::encode_batch`]).
    ///
    /// The paint arms the channel where the mode marks `role: segment_enable`
    /// and no `role: segment_color`. A stream owns the whole-frame channel,
    /// and [`DeviceHandle::draw`] paints every LED.
    ///
    /// Serves [`Role::SegmentColorMasked`], [`Role::SegmentEnable`] and
    /// [`Role::SegmentGradient`].
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`], [`Error::NoRoleArg`],
    /// [`Error::ZoneCountUnknown`], [`Error::ColorCountMismatch`],
    /// [`Error::ZoneListColorCount`], [`Error::Codec`] for a zone index the
    /// command does not declare, plus what [`DeviceHandle::send`] fails with.
    pub async fn segment(&self, paint: &Paint<'_>) -> Result<Served> {
        let call = self.resolve()?;
        let (mode, sku, device) = (call.mode(), call.sku(), call.spec());

        // A zone list names its zones, so the count of every zone is not read.
        let resolution = match paint.zones {
            Some(_) => Resolution::App,
            None => paint.resolution,
        };
        let plan = plan(
            device,
            mode,
            &StreamOptions {
                resolution,
                gradient: paint.gradient,
                ..StreamOptions::default()
            },
            Purpose::Paint,
        )?;
        let frames = if let (
            Some(zones),
            Painter::Masked {
                colors,
                zones: mask,
                ..
            },
        ) = (paint.zones, &plan.painter)
        {
            let [rgb] = *paint.colors else {
                return Err(Error::ZoneListColorCount {
                    colors: paint.colors.len(),
                });
            };
            vec![
                Args::new()
                    .rgb(colors.as_str(), vec![rgb])
                    .zones(mask.as_str(), zones.to_vec()),
            ]
        } else {
            let colors = colors(sku, &plan, paint.colors)?;
            let colors = match plan.spread {
                Some(spread) => spread.apply(&colors),
                None => colors,
            };
            paint::frames(&plan.painter, colors)?
        };

        if device.command_for(mode, Role::SegmentColor).is_none() {
            self.arm(mode, sku, device).await?;
        }
        let mut commands = Vec::new();
        if let Some((entry, value)) = &plan.gradient {
            commands.push((
                entry.command.clone(),
                Args::new().int(entry.arg.as_str(), *value),
            ));
        }
        let command = plan.painter.command();
        commands.extend(frames.into_iter().map(|args| (command.to_owned(), args)));
        // The plan refuses a zone count of zero, so one color gives one frame.
        call.send_batch(&commands)
            .await?
            .ok_or_else(|| Error::ZoneCountUnknown {
                sku: sku.to_owned(),
            })
    }

    /// Set whether the firmware interpolates between zones, without painting.
    ///
    /// The interpolation wraps from the last zone back to the first, so one
    /// lit zone at one end also lights the other. The setting stays until
    /// something changes it — `docs/protocol/state.md` 5.
    ///
    /// This needs a mode whose file marks `role: segment_gradient`, a command
    /// that carries the setting alone. A mode that carries it inside the
    /// painting frame takes [`Paint::gradient`] instead, which sets both at
    /// once: the SDK does not hold what the device shows, so it cannot repaint
    /// the same colors under the other setting.
    ///
    /// Serves [`Role::SegmentGradient`].
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`], [`Error::NoRoleArg`], plus what
    /// [`DeviceHandle::send`] fails with.
    pub async fn gradient(&self, on: bool) -> Result<Served> {
        self.one_arg(Role::SegmentGradient, ArgRole::Gradient, i64::from(on))
            .await
    }

    /// Arm the segment channel, where the mode has an entry that arms it.
    ///
    /// A paint that follows the arming frame at once is dropped in silence, so
    /// this waits for the firmware to switch channel before it returns — see
    /// `docs/protocol/lan.md` 2.3.
    async fn arm(&self, mode: Mode, sku: &str, device: &Device) -> Result<()> {
        let Some(command) = device.command_for(mode, Role::SegmentEnable) else {
            return Ok(());
        };
        let command = command.to_owned();
        let arg = arg_for(sku, device, mode, &command, ArgRole::Enable)?.to_owned();
        self.send_resolved(mode, sku, &command, &Args::new().int(arg, 1))
            .await?;
        tokio::time::sleep(device.measurements.arm_settle()).await;
        Ok(())
    }
}

/// The color of every zone, from what the caller supplied.
///
/// A list must state every zone: the firmware reads the count off the frame
/// and groups the LEDs around it, so a shorter list re-groups them rather than
/// leaving the rest alone.
fn colors(sku: &str, plan: &Plan, supplied: &[[u8; 3]]) -> Result<Vec<[u8; 3]>> {
    let expected = plan.zones;
    match supplied {
        [one] => Ok(vec![*one; expected]),
        many if many.len() == expected => Ok(many.to_vec()),
        many => Err(Error::ColorCountMismatch {
            sku: sku.to_owned(),
            expected,
            got: many.len(),
        }),
    }
}
