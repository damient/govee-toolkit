//! One fixture's send path.
//!
//! One task per fixture: a failing device therefore stops no other one. The
//! task holds what it last sent and writes only what changed, and it sends the
//! current values again after the refresh interval — see `docs/dmx.md`.

use std::time::Duration;

use govee_toolkit::stream::{Rate, Resolution, StreamOptions};
use govee_toolkit::{DeviceId, Error, Govee, Result, SegmentStream};
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use super::look::Look;
use super::{Counts, Failure};
use crate::patch::Fixture;
use crate::profile::Personality;

/// What one fixture last received. Every field is compared before a write, so
/// a desk that holds a static look puts no traffic on the device.
#[derive(Debug, Default)]
struct Sent {
    on: Option<bool>,
    brightness: Option<i64>,
    color: Option<[u8; 3]>,
    white_temp: Option<i64>,
    zones: Vec<[u8; 3]>,
}

/// One fixture, and the device it drives.
#[derive(Debug)]
pub(super) struct Feeder {
    govee: Govee,
    id: DeviceId,
    /// `None` where the personality carries no zone: those devices take the
    /// `color` and `color_temp` roles instead.
    options: Option<StreamOptions>,
    stream: Option<SegmentStream>,
    refresh: Duration,
    sent: Sent,
    /// The last look received, which the refresh sends again.
    last: Option<Look>,
    failures: mpsc::Sender<Failure>,
    counts: Counts,
}

impl Feeder {
    pub(super) fn new(
        govee: &Govee,
        fixture: &Fixture,
        refresh: Duration,
        failures: mpsc::Sender<Failure>,
    ) -> Self {
        let id = fixture.entry.device.clone();
        Self {
            govee: govee.clone(),
            id: id.clone(),
            options: options(fixture),
            stream: None,
            refresh,
            sent: Sent::default(),
            last: None,
            failures,
            counts: Counts {
                id,
                frames_sent: 0,
                frames_superseded: 0,
            },
        }
    }
}

/// Take looks until the node stops, then disarm and answer the counters.
pub(super) async fn run(mut feeder: Feeder, mut looks: watch::Receiver<(u64, Look)>) -> Counts {
    let mut taken = 0u64;
    let mut due = Instant::now() + feeder.refresh;
    loop {
        tokio::select! {
            arrived = looks.changed() => {
                if arrived.is_err() {
                    break;
                }
                let (generation, look) = looks.borrow_and_update().clone();
                // Every generation between the last one taken and this one was
                // replaced before a write carried it.
                feeder.counts.frames_superseded += generation.saturating_sub(taken + 1);
                taken = generation;
                if feeder.apply(&look).await {
                    due = Instant::now() + feeder.refresh;
                }
            }
            () = tokio::time::sleep_until(due) => {
                feeder.resend().await;
                due = Instant::now() + feeder.refresh;
            }
        }
    }
    feeder.finish().await
}

impl Feeder {
    /// Write what changed. Answers whether anything went out.
    async fn apply(&mut self, look: &Look) -> bool {
        if look.resend {
            self.sent = Sent::default();
        }
        self.last = Some(look.clone());
        match self.write(look).await {
            Ok(wrote) => wrote,
            Err(error) => {
                self.failed(&error);
                // A failed write leaves the device on an unknown look, so the
                // next one must not be compared against what this one meant to
                // send.
                self.sent = Sent::default();
                true
            }
        }
    }

    /// Send the current values once. Nothing acknowledges a LAN frame, so a
    /// lost one would otherwise hold a stale look until the desk changes it.
    async fn resend(&mut self) {
        let Some(look) = self.last.clone() else {
            return;
        };
        self.sent = Sent::default();
        self.apply(&look).await;
    }

    async fn write(&mut self, look: &Look) -> Result<bool> {
        if !look.on {
            return self.power_off().await;
        }
        let mut wrote = self.power_on().await?;
        if let Some(level) = look.brightness
            && self.sent.brightness != Some(level)
        {
            self.govee.device(&self.id).brightness(level).await?;
            self.sent.brightness = Some(level);
            self.counts.frames_sent += 1;
            wrote = true;
        }
        if self.options.is_some() {
            return Ok(self.zones(&look.zones)? || wrote);
        }
        if let Some(rgb) = look.color
            && self.sent.color != Some(rgb)
        {
            self.govee.device(&self.id).color(rgb).await?;
            self.sent.color = Some(rgb);
            self.counts.frames_sent += 1;
            wrote = true;
        }
        if let Some(kelvin) = look.white_temp
            && self.sent.white_temp != Some(kelvin)
        {
            self.govee.device(&self.id).color_temp(kelvin).await?;
            self.sent.white_temp = Some(kelvin);
            self.counts.frames_sent += 1;
            wrote = true;
        }
        Ok(wrote)
    }

    /// Power the device on, and arm the channel where the personality paints
    /// zones. Arming a dark strip paints nothing, so the order is fixed.
    async fn power_on(&mut self) -> Result<bool> {
        if self.sent.on == Some(true) {
            return Ok(false);
        }
        self.govee.device(&self.id).power(true).await?;
        self.sent.on = Some(true);
        self.counts.frames_sent += 1;
        if let Some(options) = self.options.clone()
            && self.stream.is_none()
        {
            let stream = self.govee.device(&self.id).open_stream(options).await?;
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
        self.govee.device(&self.id).power(false).await?;
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

    /// Disarm the channel, and carry what it sent into the counters.
    async fn close(&mut self) {
        let Some(stream) = self.stream.take() else {
            return;
        };
        self.counts.frames_sent += stream.frames_sent();
        self.counts.frames_superseded += stream.frames_superseded();
        if let Err(error) = stream.close().await {
            self.failed(&error);
        }
    }

    async fn finish(mut self) -> Counts {
        self.close().await;
        self.counts
    }

    /// Report a failure, and keep the fixture running: a show does not stop
    /// because one device dropped.
    fn failed(&self, error: &Error) {
        let failure = Failure {
            id: self.id.clone(),
            reason: error.to_string(),
        };
        // A full channel means the reader is behind. Dropping the report keeps
        // the send path free, which is what the report is about.
        drop(self.failures.try_send(failure));
    }
}

/// How the stream opens for this fixture, and `None` where the personality
/// carries no zone.
fn options(fixture: &Fixture) -> Option<StreamOptions> {
    let resolution = match fixture.profile.personality() {
        Personality::Basic | Personality::Full => return None,
        Personality::Pixel => Resolution::App,
        Personality::PixelNative => Resolution::Native,
    };
    Some(StreamOptions {
        resolution,
        rate: fixture.entry.max_hz.map_or(Rate::Measured, Rate::Fixed),
        gradient: false,
    })
}
