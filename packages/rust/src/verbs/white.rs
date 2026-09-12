//! Set the white temperature, and the white it renders.
//!
//! One call sets both, because one frame carries both. A mode whose firmware
//! renders the temperature declares the kelvin argument alone, and the file
//! says so: the SDK fills what the entry marks, and nothing else.

use super::Resolved;
use crate::codec::{ArgRole, Args, Role, white};
use crate::device::DeviceHandle;
use crate::error::{Error, Result};
use crate::event::Served;
use crate::stream::resolve::mask_limit;

impl DeviceHandle<'_> {
    /// Set the white temperature, in kelvin.
    ///
    /// White and color are mutually exclusive states: this ends the color the
    /// device showed. The accepted range is the one the argument's `range:`
    /// gives. A value outside it is an error, never a clamp.
    ///
    /// Where the device file marks the three white components, the SDK renders
    /// the temperature and fills them, because that firmware renders nothing
    /// itself (`docs/protocol/ble.md` 2.3). [`crate::codec::white`] documents
    /// the curve. To send another rendering, name the entry through
    /// [`DeviceHandle::send`] and pass the components.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for the command marked
    /// `role: color_temp` and its argument marked `role: color_temp`, plus
    /// [`Error::ZoneMaskUnbounded`] where the entry paints by zone mask and
    /// nothing bounds that mask.
    pub async fn color_temp(&self, kelvin: i64) -> Result<Served> {
        let entry = self.resolve(Role::ColorTemp)?;
        let mut args = Args::new().int(entry.arg(ArgRole::ColorTemp)?, kelvin);

        let [red, green, blue] = white::rgb(kelvin);
        for (arg_role, value) in [
            (ArgRole::WhiteRed, red),
            (ArgRole::WhiteGreen, green),
            (ArgRole::WhiteBlue, blue),
        ] {
            if let Some(name) = entry.marked(arg_role) {
                args = args.int(name, i64::from(value));
            }
        }

        // An entry that paints by zone mask names the zones it applies to, and
        // a temperature applies to the whole device.
        if let Some(name) = entry.marked(ArgRole::Zones) {
            args = args.zones(name, every_zone(&entry)?);
        }

        entry.send(self, &args).await
    }
}

/// Every zone the mask of the entry can name, zero-based.
///
/// The mask's own bound, not `capabilities.segments.count`: the count is what
/// the vendor app exposes, and a mask that reaches further would leave the
/// zones past it holding the color they had.
fn every_zone(entry: &Resolved<'_>) -> Result<Vec<u16>> {
    let count = mask_limit(entry.device, entry.mode, &entry.command).ok_or_else(|| {
        Error::ZoneMaskUnbounded {
            sku: entry.sku.clone(),
            mode: entry.mode,
            command: entry.command.clone(),
        }
    })?;
    Ok((0..u16::try_from(count).unwrap_or(u16::MAX)).collect())
}
