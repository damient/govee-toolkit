//! What one pass puts on the wire.
//!
//! The pass writes what changed of one look, and nothing else: a desk that
//! holds a static look puts no traffic on the device. It writes the power
//! first, then the brightness, then the colors.

use govee_toolkit::Result;
use tokio::time::Instant;

use super::Feeder;
use crate::apply::look::Look;

/// What a fixture at dimmer 0 is doing.
///
/// A device that is powered off answers nothing until it is powered on again,
/// and the firmware needs time between the two. A dimmer that dips through 0
/// therefore takes the device dark and leaves it on, and the power off waits
/// for the dimmer to stay at 0 — see `docs/dmx.md`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dark {
    /// The dimmer carries a value.
    #[default]
    No,
    /// On and black since this instant. The power off is due one off delay
    /// after it.
    Since(Instant),
    /// The power off is due on the next pass, whatever the off delay. The
    /// signal loss answer takes this: a sender that went away is not a dimmer
    /// that dipped, and the rig must go off at once.
    Due,
    /// Powered off.
    Off,
}

/// What the pass does with a look at dimmer 0.
enum Step {
    /// The fixture is already dark, and the power off is not due.
    Nothing,
    /// Take every color to 0 and leave the device on.
    Black(Look),
    /// Power the device off.
    Off,
}

/// What one fixture last received. Every field is compared before a write, so
/// a desk that holds a static look puts no traffic on the device.
#[derive(Debug, Default)]
pub(super) struct Sent {
    pub(super) on: Option<bool>,
    pub(super) brightness: Option<i64>,
    pub(super) color: Option<[u8; 3]>,
    pub(super) white_temp: Option<i64>,
    pub(super) zones: Vec<[u8; 3]>,
}

impl Feeder {
    /// Wait out the mode's command gap since the last datagram to this
    /// device.
    ///
    /// The wait spans two passes: a dimmer taken to 0 and straight back up
    /// writes the power off and the power on from two passes, and a device
    /// that reads them back to back can apply them in the other order. A pass
    /// that changes one slot, behind a quiet device, waits not at all, so a
    /// color chase pays nothing.
    ///
    /// # Errors
    ///
    /// As for [`govee_toolkit::DeviceHandle::command_gap`].
    pub(super) async fn space(&self) -> Result<()> {
        let Some(last) = self.last_sent else {
            return Ok(());
        };
        if let Some(left) = self.device().command_gap()?.checked_sub(last.elapsed()) {
            tokio::time::sleep(left).await;
        }
        Ok(())
    }

    /// Note the datagram that just went out, which the next one waits behind.
    pub(super) fn mark(&mut self) {
        self.last_sent = Some(Instant::now());
    }

    pub(super) async fn write(&mut self, look: &Look) -> Result<bool> {
        let dark;
        let look = if look.on {
            self.dark = Dark::No;
            look
        } else {
            match self.darken(look) {
                Step::Nothing => return Ok(false),
                Step::Off => return self.power_off().await,
                Step::Black(black) => {
                    dark = black;
                    &dark
                }
            }
        };
        let mut wrote = self.power_on().await?;
        if let Some(level) = look.brightness
            && self.sent.brightness != Some(level)
        {
            self.space().await?;
            self.device().brightness(level).await?;
            self.mark();
            self.sent.brightness = Some(level);
            self.counts.frames_sent += 1;
            wrote = true;
        }
        // The white channel back at 0 sends no white command, and the color
        // did not change, so nothing else would take the device out of white.
        // The color is what does it, so the pass sends it again.
        if look.white_temp.is_none() && self.sent.white_temp.is_some() {
            self.sent.white_temp = None;
            self.sent.color = None;
            self.sent.zones.clear();
        }
        if self.options.is_some() {
            return Ok(self.zones(&look.zones).await? || wrote);
        }
        let white = look
            .white_temp
            .filter(|kelvin| self.sent.white_temp != Some(*kelvin));
        if let Some(rgb) = look.color
            && self.sent.color != Some(rgb)
        {
            if white.is_some() {
                // A white command replaces the color on the device, and it
                // goes out last. The color would show for the milliseconds
                // before it, which reads as a blink. The pass counts the
                // color as applied: the device shows the white, and the pass
                // that takes the white channel back to 0 sends the color.
                self.sent.color = Some(rgb);
            } else {
                self.space().await?;
                self.device().color(rgb).await?;
                self.mark();
                self.sent.color = Some(rgb);
                self.counts.frames_sent += 1;
                wrote = true;
            }
        }
        if let Some(kelvin) = white {
            self.space().await?;
            self.device().color_temp(kelvin).await?;
            self.mark();
            self.sent.white_temp = Some(kelvin);
            self.counts.frames_sent += 1;
            wrote = true;
        }
        Ok(wrote)
    }

    /// Power the device on, and arm the channel where the personality paints
    /// zones. Arming a dark strip paints nothing, so the order is fixed.
    ///
    /// An armed channel is proof the device is on, so the power command is
    /// skipped there whatever the refresh cleared: a power command ends the
    /// armed channel on some devices, and the device then shows the color it
    /// held before the stream — see `docs/protocol/lan.md` 2.3.
    async fn power_on(&mut self) -> Result<bool> {
        if self.sent.on == Some(true) || self.stream.is_some() {
            return Ok(false);
        }
        self.space().await?;
        self.device().power(true).await?;
        self.mark();
        self.sent.on = Some(true);
        self.counts.frames_sent += 1;
        if let Some(options) = self.options.clone()
            && self.stream.is_none()
        {
            self.space().await?;
            let stream = self.device().open_stream(options).await?;
            self.mark();
            self.stream = Some(stream);
        }
        Ok(true)
    }

    /// What a look at dimmer 0 asks of the device.
    ///
    /// The first pass takes the fixture dark and leaves it on. The power off
    /// follows one off delay later, and an off delay of zero powers the
    /// device off at once.
    fn darken(&mut self, look: &Look) -> Step {
        match self.dark {
            Dark::Off => Step::Nothing,
            Dark::Since(since) if since.elapsed() < self.timing.off_delay => Step::Nothing,
            Dark::Since(_) | Dark::Due => {
                self.dark = Dark::Off;
                Step::Off
            }
            Dark::No if self.timing.off_delay.is_zero() => {
                self.dark = Dark::Off;
                Step::Off
            }
            Dark::No => {
                self.dark = Dark::Since(Instant::now());
                Step::Black(look.black())
            }
        }
    }

    /// When the power off of a dark fixture is due, and `None` where the
    /// fixture is not waiting for one.
    pub(super) fn off_due(&self) -> Option<Instant> {
        match self.dark {
            Dark::Since(since) => Some(since + self.timing.off_delay),
            Dark::No | Dark::Due | Dark::Off => None,
        }
    }

    /// The dimmer at 0. The channel is disarmed first: it holds the colors
    /// only while it is armed, and a device that comes back on arms it again.
    async fn power_off(&mut self) -> Result<bool> {
        if self.sent.on == Some(false) {
            return Ok(false);
        }
        self.close().await;
        self.space().await?;
        self.device().power(false).await?;
        self.mark();
        self.sent = Sent {
            on: Some(false),
            ..Sent::default()
        };
        self.counts.frames_sent += 1;
        Ok(true)
    }

    /// Hand the zones to the stream, which paces them and drops what a later
    /// frame replaced.
    ///
    /// The wait comes before the paint, not before the pass: a look that
    /// repeats the zones writes nothing, so a static look waits not at all.
    async fn zones(&mut self, zones: &[[u8; 3]]) -> Result<bool> {
        if self.stream.is_none() || self.sent.zones == zones {
            return Ok(false);
        }
        self.space().await?;
        if let Some(stream) = &self.stream {
            stream.set_all(zones)?;
        }
        self.sent.zones.clear();
        self.sent.zones.extend_from_slice(zones);
        Ok(true)
    }
}
