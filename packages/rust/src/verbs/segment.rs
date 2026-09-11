//! Paint one color over zones.
//!
//! Two roles paint, and the device file decides which one a mode carries. A
//! `role: segment_color` command carries every zone in one frame, so it cannot
//! paint a subset: the frame states the color of every zone, and this crate
//! does not hold what the other zones show. A `role: segment_color_masked`
//! command carries one color and the zones that take it, and leaves the rest
//! alone.

use super::{arg_for, command_for};
use crate::codec::catalog::Device;
use crate::codec::{ArgRole, Args, Mode, Role};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;
use crate::stream::resolve::{Painter, plan};
use crate::stream::{StreamOptions, paint};

impl DeviceHandle<'_> {
    /// Paint zones one color.
    ///
    /// `zones` names the zones to paint, zero-based, and leaves every other
    /// zone alone. Only `role: segment_color_masked` can do that. `None`
    /// paints every zone, over whichever painting role the file marks, and
    /// takes the zone count from `capabilities.segments.count`.
    ///
    /// `gradient` asks the firmware to interpolate between zones and to wrap
    /// from the last zone back to the first. `true` is refused where the file
    /// can carry the setting nowhere, rather than dropped.
    ///
    /// The channel is armed where the file marks `role: segment_enable`.
    /// Nothing disarms it: a disarm ends the channel, and the colors with it.
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`] if the device file marks no painting entry the
    /// call needs, [`Error::NoRoleArg`] if such an entry marks no argument to
    /// put the color in, [`Error::ZoneCountUnknown`] if every zone was asked
    /// for and nothing records the count, [`Error::Codec`] if a zone index is
    /// outside what the command declares, plus what
    /// [`DeviceHandle::send`] fails with.
    pub async fn segment(
        &self,
        zones: Option<&[u16]>,
        rgb: [u8; 3],
        gradient: bool,
    ) -> Result<Served> {
        match zones {
            Some(zones) => self.segment_zones(zones, rgb, gradient).await,
            None => self.segment_all(rgb, gradient).await,
        }
    }

    /// One color over every zone, over whichever painting role the file marks.
    async fn segment_all(&self, rgb: [u8; 3], gradient: bool) -> Result<Served> {
        let mode = self.serving_mode()?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;

        let plan = plan(
            device,
            mode,
            &StreamOptions {
                gradient,
                ..StreamOptions::default()
            },
        )?;
        if gradient && plan.gradient.is_none() && !carries_gradient(&plan.painter) {
            return Err(no_gradient(&sku, mode));
        }

        self.arm(mode, &sku, device).await?;
        if let Some((entry, value)) = &plan.gradient {
            let args = Args::new().int(entry.arg.as_str(), *value);
            self.send_on(mode, &entry.command, &args).await?;
        }

        let command = plan.painter.command().to_owned();
        let mut served = None;
        for args in paint::frames(&plan.painter, vec![rgb; plan.zones])? {
            served = Some(self.send_on(mode, &command, &args).await?);
        }
        // The plan refuses a zone count of zero, so one color gives one frame.
        served.ok_or(Error::ZoneCountUnknown { sku })
    }

    /// One color over the zones the caller names, leaving the rest alone.
    async fn segment_zones(&self, zones: &[u16], rgb: [u8; 3], gradient: bool) -> Result<Served> {
        let mode = self.serving_mode()?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;

        let command = command_for(&sku, device, mode, Role::SegmentColorMasked)?.to_owned();
        let colors = arg_for(&sku, device, mode, &command, ArgRole::Colors)?.to_owned();
        let mask = arg_for(&sku, device, mode, &command, ArgRole::Zones)?.to_owned();

        // The painting frame carries the setting on some modes, and a frame of
        // its own carries it on others.
        let in_frame = device
            .commands
            .get(mode)
            .get(&command)
            .and_then(|spec| spec.arg_for(ArgRole::Gradient))
            .map(ToOwned::to_owned);
        let alone = device
            .command_for(mode, Role::SegmentGradient)
            .map(ToOwned::to_owned);
        if gradient && in_frame.is_none() && alone.is_none() {
            return Err(no_gradient(&sku, mode));
        }

        self.arm(mode, &sku, device).await?;
        if in_frame.is_none()
            && let Some(entry) = alone
        {
            let arg = arg_for(&sku, device, mode, &entry, ArgRole::Gradient)?.to_owned();
            let args = Args::new().int(arg, i64::from(gradient));
            self.send_on(mode, &entry, &args).await?;
        }

        let mut args = Args::new()
            .rgb(colors, vec![rgb])
            .zones(mask, zones.to_vec());
        if let Some(arg) = in_frame {
            args = args.int(arg, i64::from(gradient));
        }
        self.send_on(mode, &command, &args).await
    }

    /// Arm the segment channel, where the mode has an entry that arms it.
    async fn arm(&self, mode: Mode, sku: &str, device: &Device) -> Result<()> {
        let Some(command) = device.command_for(mode, Role::SegmentEnable) else {
            return Ok(());
        };
        let command = command.to_owned();
        let arg = arg_for(sku, device, mode, &command, ArgRole::Enable)?.to_owned();
        self.send_on(mode, &command, &Args::new().int(arg, 1))
            .await?;
        Ok(())
    }
}

/// Whether the painting frame itself carries the gradient setting.
fn carries_gradient(painter: &Painter) -> bool {
    matches!(
        painter,
        Painter::Whole {
            gradient: Some(_),
            ..
        }
    )
}

fn no_gradient(sku: &str, mode: Mode) -> Error {
    Error::NoRoleCommand {
        sku: sku.to_owned(),
        mode,
        role: Role::SegmentGradient,
    }
}
