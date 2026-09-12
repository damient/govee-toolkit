//! Play an effect the device renders from its own microphone.
//!
//! The device listens, and the host sends nothing per beat: one command starts
//! the effect and the firmware plays it until another command replaces it.
//! Nothing stops it on its own — no mode declares a command that leaves music
//! and stays where it was. To end it, set a color, a temperature or the power.

use super::{arg_for, command_for};
use crate::codec::{ArgRole, Args, Role};
use crate::device::DeviceHandle;
use crate::error::Result;
use crate::event::Served;

/// What to play, in the terms the device file declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Music {
    /// Which effect. The identifiers are the mode's own, and the accepted ones
    /// are what the argument's `range:` gives: the same number names another
    /// effect over another mode, and no device file maps one to what the
    /// device renders.
    pub effect: i64,
    /// How loud the sound must be for the device to answer it, in the unit the
    /// argument's `range:` gives. Sent where the entry declares the argument,
    /// and dropped where it does not.
    pub sensitivity: i64,
    /// Render in fades rather than on the beat.
    pub soft: bool,
    /// The color to impose. `None` leaves the colors to the firmware.
    pub color: Option<[u8; 3]>,
}

impl DeviceHandle<'_> {
    /// Play a music effect.
    ///
    /// An identifier the entry accepts is not one the device renders: firmware
    /// takes a value, reads it back and plays nothing. See the `notes:` of the
    /// entry for what one unit did.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for the command marked `role: music`
    /// and its argument marked `role: effect`. A value outside the declared
    /// range is an error, never a clamp.
    pub async fn music(&self, music: &Music) -> Result<Served> {
        let mode = self.govee.choose(self.id())?;
        let sku = self.govee.sku(self.id())?;
        let device = self.govee.catalog().device(&sku)?;
        let command = command_for(&sku, device, mode, Role::Music)?.to_owned();
        let marked = |arg_role: ArgRole| {
            device
                .commands
                .get(mode)
                .get(&command)
                .and_then(|spec| spec.arg_for(arg_role))
        };

        let effect = arg_for(&sku, device, mode, &command, ArgRole::Effect)?;
        let mut args = Args::new().int(effect, music.effect);

        let rgb = music.color.unwrap_or_default();
        let optional = [
            (ArgRole::Sensitivity, music.sensitivity),
            (ArgRole::Soft, i64::from(music.soft)),
            (ArgRole::ColorMode, i64::from(music.color.is_some())),
            (ArgRole::Red, i64::from(rgb[0])),
            (ArgRole::Green, i64::from(rgb[1])),
            (ArgRole::Blue, i64::from(rgb[2])),
        ];
        for (arg_role, value) in optional {
            if let Some(name) = marked(arg_role) {
                args = args.int(name, value);
            }
        }

        self.send_on(mode, &command, &args).await
    }
}
