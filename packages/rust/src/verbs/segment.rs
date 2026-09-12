//! Paint zones.
//!
//! Two roles paint, and the device file decides which one a mode carries. A
//! `role: segment_color` command carries every zone in one frame, so it cannot
//! paint a subset: the frame states the color of every zone, and this crate
//! does not hold what the other zones show. It can state a color per zone,
//! which is what reaches one LED at a time where the unit addresses them. A
//! `role: segment_color_masked` command carries one color and the zones that
//! take it, and leaves the rest alone.

use super::arg_for;
use crate::codec::catalog::Device;
use crate::codec::{ArgRole, Args, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;
use crate::stream::resolve::{Painter, Plan, plan};
use crate::stream::{Resolution, StreamOptions, paint};

/// One painting of a device's zones.
///
/// The same three questions the `segment` verb of the CLI asks: which zones,
/// which colors, and how many zones the frame states.
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
    /// One color in [`Paint::colors`] paints every zone. A longer list states
    /// one zone each, which is how a mode with a per-LED channel reaches one
    /// LED: ask for [`Resolution::Native`] and pass that many colors. Only
    /// `role: segment_color` states a color per zone.
    ///
    /// [`Paint::zones`] names the zones to paint and leaves every other zone
    /// alone. Only `role: segment_color_masked` can do that, and it takes one
    /// color. `None` paints every zone, over whichever painting role the file
    /// marks. Every zone is what the frame reaches: the bound of its mask
    /// where it names its zones, and the zone count
    /// [`Paint::resolution`] resolves where one frame states them all.
    ///
    /// [`Paint::gradient`] asks the firmware to interpolate between zones and
    /// to wrap from the last zone back to the first. `true` is refused where
    /// the file can carry the setting nowhere, rather than dropped.
    ///
    /// The channel is armed where the file marks `role: segment_enable`.
    /// Nothing disarms it: a disarm ends the channel, and the colors with it.
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`] if the device file marks no painting entry the
    /// call needs, [`Error::NoRoleArg`] if such an entry marks no argument to
    /// put the color in, [`Error::ZoneCountUnknown`] if every zone was asked
    /// for and nothing records the count, [`Error::ColorCountMismatch`] if the
    /// color list is neither one color nor one per zone,
    /// [`Error::ZoneListColorCount`] if a zone list came with more than one
    /// color, [`Error::ResolutionNotDistinct`] if the unit renders the zone
    /// count asked for as a smaller one, [`Error::Codec`] if a zone index is
    /// outside what the command declares, plus what
    /// [`DeviceHandle::send`] fails with.
    pub async fn segment(&self, paint: &Paint<'_>) -> Result<Served> {
        match paint.zones {
            Some(zones) => self.segment_zones(zones, paint).await,
            None => self.segment_all(paint).await,
        }
    }

    /// Every zone, over whichever painting role the file marks.
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

    /// One color over the zones the caller names, leaving the rest alone.
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

        // The painting frame carries the setting on some modes, and a frame of
        // its own carries it on others.
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
    /// lit zone at one end also lights the other. `false` gives hard-edged
    /// zones. The setting stays until something changes it.
    ///
    /// This needs a mode whose file marks `role: segment_gradient`, which is a
    /// command that carries the setting alone. A mode that carries the setting
    /// inside its painting frame cannot serve this call: the SDK does not hold
    /// what the device shows, so it cannot repaint the same colors under the
    /// other setting. Pass [`Paint::gradient`] there, which sets both at once.
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`] if the device file marks no entry
    /// `role: segment_gradient` for the chosen mode, [`Error::NoRoleArg`] if
    /// that entry marks no argument `role: gradient`, plus what
    /// [`DeviceHandle::send`] fails with.
    pub async fn gradient(&self, on: bool) -> Result<Served> {
        self.one_arg(Role::SegmentGradient, ArgRole::Gradient, i64::from(on))
            .await
    }

    /// Arm the segment channel, where the mode has an entry that arms it.
    ///
    /// Waits for the firmware to switch channel before it returns. A paint
    /// that follows the arming frame at once is dropped in silence, so the
    /// wait is what makes the first paint render — see
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
/// One color fills the frame. A longer list states the zones itself, and must
/// state every one of them: the firmware reads the count off the frame and
/// groups the LEDs around it, so a shorter list would re-group them rather
/// than leave the rest alone.
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

/// How many zones "every zone" is, for a paint that covers the whole device.
///
/// A frame that names its zones covers what its mask can name, which is not
/// `capabilities.segments.count`: the count is what the vendor app exposes,
/// and the zones between it and the end of the mask would keep the color they
/// had. A frame that states every zone at once covers the count the plan
/// resolved, because its length is that count.
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
