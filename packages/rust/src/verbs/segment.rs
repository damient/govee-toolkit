//! Paint zones.
//!
//! Two roles paint, and the device file decides which one a mode carries. A
//! `role: segment_color` command states the color of every zone in one frame,
//! so it cannot paint a subset: this crate does not hold what the other zones
//! show. A `role: segment_color_masked` command carries one color and the
//! zones that take it, and leaves the rest alone.

use super::arg_for;
use crate::codec::catalog::Device;
use crate::codec::{ArgRole, Args, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;
use crate::stream::resolve::{Painter, Plan, plan};
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

impl DeviceHandle<'_> {
    /// Paint zones.
    ///
    /// A color per zone needs `role: segment_color`, and reaches one LED at a
    /// time under [`Resolution::Native`]. A zone list needs
    /// `role: segment_color_masked`. "Every zone" is the bound of the mask
    /// where the frame names its zones, and the count [`Paint::resolution`]
    /// resolves where one frame states them all.
    ///
    /// The channel is armed where the file marks `role: segment_enable`.
    /// Nothing disarms it: a disarm ends the channel, and the colors with it.
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`], [`Error::NoRoleArg`],
    /// [`Error::ZoneCountUnknown`], [`Error::ColorCountMismatch`],
    /// [`Error::ZoneListColorCount`], [`Error::ResolutionNotDistinct`],
    /// [`Error::Codec`] for a zone index the command does not declare, plus
    /// what [`DeviceHandle::send`] fails with.
    pub async fn segment(&self, paint: &Paint<'_>) -> Result<Served> {
        match paint.zones {
            Some(zones) => self.segment_zones(zones, paint).await,
            None => self.segment_all(paint).await,
        }
    }

    async fn segment_all(&self, paint: &Paint<'_>) -> Result<Served> {
        let mode = self.serving_mode()?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;

        let plan = plan(
            device,
            mode,
            &StreamOptions {
                resolution: paint.resolution,
                gradient: paint.gradient,
                ..StreamOptions::default()
            },
        )?;
        let colors = colors(&sku, &plan, paint.colors)?;

        self.arm(mode, &sku, device).await?;
        if let Some((entry, value)) = &plan.gradient {
            let args = Args::new().int(entry.arg.as_str(), *value);
            self.send_resolved(mode, &sku, &entry.command, &args)
                .await?;
        }

        let command = plan.painter.command().to_owned();
        let mut served = None;
        for args in paint::frames(&plan.painter, colors)? {
            served = Some(self.send_resolved(mode, &sku, &command, &args).await?);
        }
        // The plan refuses a zone count of zero, so one color gives one frame.
        served.ok_or(Error::ZoneCountUnknown { sku })
    }

    async fn segment_zones(&self, zones: &[u16], paint: &Paint<'_>) -> Result<Served> {
        let [rgb] = *paint.colors else {
            return Err(Error::ZoneListColorCount {
                colors: paint.colors.len(),
            });
        };
        let gradient = paint.gradient;
        let entry = self.resolve(Role::SegmentColorMasked)?;
        let colors = entry.arg(ArgRole::Colors)?.to_owned();
        let mask = entry.arg(ArgRole::Zones)?.to_owned();

        // The painting frame carries the setting on some modes, and a frame
        // of its own carries it on others.
        let in_frame = entry.marked(ArgRole::Gradient).map(ToOwned::to_owned);
        let alone = entry
            .device
            .command_for(entry.mode, Role::SegmentGradient)
            .map(ToOwned::to_owned);
        if gradient && in_frame.is_none() && alone.is_none() {
            return Err(no_gradient(&entry.sku, entry.mode));
        }

        self.arm(entry.mode, &entry.sku, entry.device).await?;
        if in_frame.is_none()
            && let Some(command) = alone
        {
            let arg = arg_for(
                &entry.sku,
                entry.device,
                entry.mode,
                &command,
                ArgRole::Gradient,
            )?
            .to_owned();
            let args = Args::new().int(arg, i64::from(gradient));
            self.send_resolved(entry.mode, &entry.sku, &command, &args)
                .await?;
        }

        let mut args = Args::new()
            .rgb(colors, vec![rgb])
            .zones(mask, zones.to_vec());
        if let Some(arg) = in_frame {
            args = args.int(arg, i64::from(gradient));
        }
        entry.send(self, &args).await
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
    let expected = whole(plan);
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

/// How many zones "every zone" is. A masked frame covers what its mask can
/// name, and not `capabilities.segments.count`: the zones between the two
/// would keep the color they had.
fn whole(plan: &Plan) -> usize {
    match plan.painter {
        Painter::Masked { limit, .. } => limit,
        Painter::Whole { .. } => plan.zones,
    }
}

fn no_gradient(sku: &str, mode: Mode) -> Error {
    Error::NoRoleCommand {
        sku: sku.to_owned(),
        mode,
        role: Role::SegmentGradient,
    }
}
