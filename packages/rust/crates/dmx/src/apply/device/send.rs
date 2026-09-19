//! What one pass puts on the wire.
//!
//! The pass writes what changed of one look, and nothing else: a desk that
//! holds a static look puts no traffic on the device. It writes the power
//! first, then the brightness, then the colors.

use std::time::Duration;

use govee_toolkit::Result;
use tokio::time::Instant;

use super::Feeder;
use crate::apply::look::Look;

/// How long one command waits behind the one before it.
///
/// A device drops a datagram that arrives directly behind two others — see
/// `docs/protocol/lan.md` 1, "Consecutive commands". Nothing acknowledges a
/// LAN frame, so the drop is silent, and a device that reads a power off and
/// a power on back to back can apply them in the other order. The wait
/// therefore spans two passes: a dimmer taken to 0 and straight back up
/// writes the two from two passes. The gap is wider than the smallest one
/// measured, for a unit slower than the one that was measured. A pass that
/// changes one slot, behind a quiet device, waits not at all, so a color
/// chase pays nothing.
const GAP: Duration = Duration::from_millis(5);

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
    /// Wait until `GAP` has passed since the last datagram to this device.
    pub(super) async fn space(&self) {
        let Some(last) = self.last_sent else {
            return;
        };
        if let Some(left) = GAP.checked_sub(last.elapsed()) {
            tokio::time::sleep(left).await;
        }
    }

    /// Note the datagram that just went out, which the next one waits behind.
    pub(super) fn mark(&mut self) {
        self.last_sent = Some(Instant::now());
    }

    pub(super) async fn write(&mut self, look: &Look) -> Result<bool> {
        if !look.on {
            return self.power_off().await;
        }
        let mut wrote = self.power_on().await?;
        if let Some(level) = look.brightness
            && self.sent.brightness != Some(level)
        {
            self.space().await;
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
            self.space().await;
            return Ok(self.zones(&look.zones)? || wrote);
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
                self.space().await;
                self.device().color(rgb).await?;
                self.mark();
                self.sent.color = Some(rgb);
                self.counts.frames_sent += 1;
                wrote = true;
            }
        }
        if let Some(kelvin) = white {
            self.space().await;
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
        self.space().await;
        self.device().power(true).await?;
        self.mark();
        self.sent.on = Some(true);
        self.counts.frames_sent += 1;
        if let Some(options) = self.options.clone()
            && self.stream.is_none()
        {
            self.space().await;
            let stream = self.device().open_stream(options).await?;
            self.mark();
            self.stream = Some(stream);
        }
        Ok(true)
    }

    /// The dimmer at 0. The channel is disarmed first: it holds the colors
    /// only while it is armed, and a device that comes back on arms it again.
    async fn power_off(&mut self) -> Result<bool> {
        if self.sent.on == Some(false) {
            return Ok(false);
        }
        self.close().await;
        self.space().await;
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
    fn zones(&mut self, zones: &[[u8; 3]]) -> Result<bool> {
        let Some(stream) = &self.stream else {
            return Ok(false);
        };
        if self.sent.zones == zones {
            return Ok(false);
        }
        stream.set_all(zones)?;
        self.sent.zones = zones.to_vec();
        Ok(true)
    }
}
