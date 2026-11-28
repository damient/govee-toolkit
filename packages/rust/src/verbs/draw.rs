//! Store a drawing of the LEDs.

use super::fill;
use crate::codec::{ArgValue, Args};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;
use crate::stream::resolve::{Painter, Purpose, plan};
use crate::stream::{Resolution, StreamOptions, paint};

/// One drawing of a device's LEDs.
#[derive(Debug, Clone, Copy)]
pub struct Drawing<'a> {
    /// The LEDs to draw, zero-based. The other LEDs show the background.
    /// `None` draws every LED.
    pub cells: Option<&'a [u16]>,
    /// One color draws every cell. A longer list states one cell each. With
    /// no cell list, the list states one zone each, and must be as long as
    /// the zone count [`Drawing::resolution`] resolves: the SDK spreads the
    /// zones over the LEDs.
    pub colors: &'a [[u8; 3]],
    /// How many zones the colors state, read where the drawing names no cell
    /// list.
    pub resolution: Resolution,
    /// Other arguments of the drawing command, by the names the device file
    /// gives them: the background, the speed, the effect. An argument left out
    /// takes the `default:` of the device file.
    pub args: &'a Args,
}

impl DeviceHandle<'_> {
    /// Store a drawing of the LEDs.
    ///
    /// Takes `role: draw`. The device keeps playing the drawing after the
    /// sender stops, and arms no channel. The drawing renders no zone
    /// interpolation.
    ///
    /// Serves [`Role::Draw`](crate::codec::Role::Draw).
    ///
    /// # Errors
    ///
    /// [`Error::NoRoleCommand`], [`Error::NoRoleArg`],
    /// [`Error::ZoneCountUnknown`], [`Error::ZoneCountUnsupported`],
    /// [`Error::ColorCountMismatch`], [`Error::Codec`] for a cell or an
    /// argument the command does not declare, plus what
    /// [`DeviceHandle::send`] fails with.
    pub async fn draw(&self, drawing: &Drawing<'_>) -> Result<Served> {
        let call = self.resolve()?;
        let (mode, sku) = (call.mode(), call.sku());
        let plan = plan(
            call.spec(),
            mode,
            &StreamOptions {
                resolution: drawing.resolution,
                ..StreamOptions::default()
            },
            Purpose::Draw,
        )?;
        let Painter::Cells { cells, colors, .. } = &plan.painter else {
            return Err(Error::ZoneCountUnknown {
                sku: sku.to_owned(),
            });
        };

        let mut args = if let Some(list) = drawing.cells {
            let painted = fill(sku, drawing.colors, list.len())?;
            Args::new()
                .zones(cells.as_str(), list.to_vec())
                .rgb(colors.as_str(), painted)
        } else {
            let painted = fill(sku, drawing.colors, plan.zones)?;
            let painted = match plan.spread {
                Some(spread) => spread.apply(&painted),
                None => painted,
            };
            paint::frames(&plan.painter, painted)?
                .pop()
                .unwrap_or_default()
        };
        for name in drawing.args.names() {
            if let Some(value) = drawing.args.get(name) {
                args.insert(name, ArgValue::clone(value));
            }
        }
        call.send(plan.painter.command(), &args).await
    }
}
