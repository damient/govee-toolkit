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
    Black(Look),
    Off,
}

/// What one fixture last received. Every field is compared before a write, so
/// a desk that holds a static look puts no traffic on the device.
#[derive(Debug, Default)]
pub(super) struct Sent {
    pub(super) on: Option<bool>,
    pub(super) brightness: Option<i64>,
    paint: Option<Paint>,
}

/// What the device shows. White, color and zones are exclusive states on the
/// device: a write of one ends the other two.
///
/// A change of variant is a change of state, so the white channel back at 0
/// sends the color or the zones again with no reset of its own.
#[derive(Debug, PartialEq, Eq)]
enum Paint {
    White(i64),
    Color([u8; 3]),
    Zones(Vec<[u8; 3]>),
}

impl Feeder {
    /// Wait out the mode's command gap since the last datagram to this
    /// device.
    ///
    /// The wait spans two passes: a device that reads a power off and the
    /// power on that follows back to back can apply them in the other order.
    /// A pass behind a quiet device waits not at all.
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

    /// The next datagram waits behind this one.
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
        let painted = if self.options.is_some() {
            self.zones_or_white(look).await?
        } else {
            self.color_or_white(look).await?
        };
        Ok(painted || wrote)
    }

    /// The white where the white channel carries a value, and the color
    /// otherwise.
    ///
    /// The color is not sent under a white: the white replaces it, and the
    /// color would show for the milliseconds before, which reads as a blink.
    async fn color_or_white(&mut self, look: &Look) -> Result<bool> {
        if let Some(kelvin) = look.white_temp {
            return self.white(kelvin).await;
        }
        let Some(rgb) = look.color else {
            // A table with no color channel paints nothing once the white is
            // gone, so the next white must go out again.
            self.sent.paint = None;
            return Ok(false);
        };
        if self.sent.paint == Some(Paint::Color(rgb)) {
            return Ok(false);
        }
        self.space().await?;
        self.device().color(rgb).await?;
        self.mark();
        self.sent.paint = Some(Paint::Color(rgb));
        self.counts.frames_sent += 1;
        Ok(true)
    }

    async fn white(&mut self, kelvin: i64) -> Result<bool> {
        if self.sent.paint == Some(Paint::White(kelvin)) {
            return Ok(false);
        }
        self.space().await?;
        self.device().color_temp(kelvin).await?;
        self.mark();
        self.sent.paint = Some(Paint::White(kelvin));
        self.counts.frames_sent += 1;
        Ok(true)
    }

    /// Power the device on.
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
        Ok(true)
    }

    /// Arm the channel. The pass powers the device on first: arming a dark
    /// strip paints nothing.
    async fn arm(&mut self) -> Result<()> {
        let Some(options) = self.options.clone() else {
            return Ok(());
        };
        if self.stream.is_some() {
            return Ok(());
        }
        self.space().await?;
        let stream = self.device().open_stream(options).await?;
        self.mark();
        self.stream = Some(stream);
        Ok(())
    }

    /// The white over the whole device where the white channel carries a
    /// value, and the zones otherwise.
    ///
    /// A white command ends the armed channel, and the zones that follow
    /// paint nothing — see `docs/protocol/lan.md` 2.3. The white therefore
    /// goes out while the channel is armed, and the disarm follows it: a
    /// disarm first would show the color the device held before the stream.
    /// The white channel back at 0 arms the channel again.
    async fn zones_or_white(&mut self, look: &Look) -> Result<bool> {
        let Some(kelvin) = look.white_temp else {
            self.arm().await?;
            return self.zones(&look.zones).await;
        };
        if !self.white(kelvin).await? {
            return Ok(false);
        }
        if self.stream.is_some() {
            self.space().await?;
            self.close().await;
        }
        Ok(true)
    }

    /// The first pass takes the fixture dark and leaves it on. The power off
    /// follows one off delay later.
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
        if self.stream.is_none()
            || matches!(&self.sent.paint, Some(Paint::Zones(sent)) if sent == zones)
        {
            return Ok(false);
        }
        self.space().await?;
        if let Some(stream) = &self.stream {
            stream.set_all(zones)?;
        }
        // The buffer of the last zones is kept, so a repaint allocates nothing.
        match &mut self.sent.paint {
            Some(Paint::Zones(sent)) => {
                sent.clear();
                sent.extend_from_slice(zones);
            }
            paint => *paint = Some(Paint::Zones(zones.to_vec())),
        }
        Ok(true)
    }
}
