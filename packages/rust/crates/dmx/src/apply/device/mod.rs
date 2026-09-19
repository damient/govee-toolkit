//! One fixture's send path.
//!
//! One task per fixture: a failing device therefore stops no other one. The
//! task holds what it last sent and writes only what changed, and it sends the
//! current values again after the refresh interval — see `docs/dmx.md`.
//!
//! The task keeps two other waits: what the patch shows after the sender goes
//! quiet, and the backoff that a device which stopped answering is retried
//! on. [`send`] holds what one pass puts on the wire.

mod send;

use govee_toolkit::stream::{Rate, Resolution, StreamOptions};
use govee_toolkit::{DeviceHandle, DeviceId, Error, Govee, Mode, SegmentStream};
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use self::send::{Dark, Sent};
use super::backoff::Backoff;
use super::look::Look;
use super::rate::Pace;
use super::{Counts, Failure, Timing};
use crate::patch::{Fixture, SignalLoss};
use crate::profile::Personality;

/// One fixture, and the device it drives.
#[derive(Debug)]
pub(super) struct Feeder {
    govee: Govee,
    id: DeviceId,
    /// `None` where the personality carries no zone: those devices take the
    /// `color` and `color_temp` roles instead.
    options: Option<StreamOptions>,
    stream: Option<SegmentStream>,
    timing: Timing,
    /// What the fixture shows once the sender goes quiet.
    loss: SignalLoss,
    backoff: Backoff,
    /// The rate the fixture takes writes at.
    pace: Pace,
    /// A look the pace held, which the next due write carries.
    held: bool,
    /// What the fixture does at dimmer 0.
    dark: Dark,
    sent: Sent,
    /// When the last datagram went out to this device. The next one waits
    /// behind it.
    last_sent: Option<Instant>,
    /// The last look received, which the refresh sends again.
    last: Option<Look>,
    failures: mpsc::Sender<Failure>,
    counts: Counts,
}

impl Feeder {
    /// The device, over `lan` alone. The bridge substitutes no mode where the
    /// device stops answering: it reports the device unreachable instead —
    /// `docs/dmx.md`.
    fn device(&self) -> DeviceHandle<'_> {
        self.govee.device_on(&self.id, Mode::Lan)
    }

    pub(super) fn new(
        govee: &Govee,
        fixture: &Fixture,
        timing: Timing,
        failures: mpsc::Sender<Failure>,
    ) -> Self {
        let id = fixture.entry.device.clone();
        Self {
            govee: govee.clone(),
            id: id.clone(),
            options: options(fixture),
            stream: None,
            timing,
            loss: fixture.entry.on_signal_loss,
            backoff: Backoff::default(),
            pace: Pace::new(fixture.entry.max_hz),
            held: false,
            dark: Dark::default(),
            sent: Sent::default(),
            last_sent: None,
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
///
/// Three waits end a pass: a look that arrived, the write that a refresh or a
/// backoff makes due, and the silence that the patch answers.
pub(super) async fn run(mut feeder: Feeder, mut looks: watch::Receiver<(u64, Look)>) -> Counts {
    let mut taken = 0u64;
    let mut due = Instant::now() + feeder.timing.refresh;
    let mut quiet = Instant::now();
    // The first look arms it: a rig waits dark for the desk rather than going
    // to the signal loss answer at start.
    let mut armed = false;
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
                let wrote = feeder.apply(&look).await;
                due = feeder.due(wrote, due);
                quiet = Instant::now() + feeder.timing.silence;
                armed = feeder.loss != SignalLoss::Hold;
            }
            () = tokio::time::sleep_until(due) => {
                let wrote = feeder.resend().await;
                due = feeder.due(wrote, Instant::now() + feeder.timing.refresh);
            }
            () = tokio::time::sleep_until(quiet), if armed => {
                let wrote = feeder.quiet().await;
                due = feeder.due(wrote, due);
                // What the patch asks for is applied once, and the next look
                // arms the wait again.
                armed = false;
            }
        }
    }
    feeder.finish().await
}

impl Feeder {
    /// Write what changed. Answers whether anything went out.
    ///
    /// A device that failed writes nothing until its backoff is over, and a
    /// device written to a moment ago writes nothing until its pace is over.
    /// The look is kept either way, so the attempt that follows carries the
    /// current one and not a stale one.
    async fn apply(&mut self, look: &Look) -> bool {
        if look.resend {
            self.sent = Sent::default();
        }
        self.last = Some(look.clone());
        let now = Instant::now();
        if self.backoff.holds(now) {
            return false;
        }
        if self.pace.holds(now) {
            // The desk sends above what the device takes, which is the count
            // the operator reads to see it.
            self.counts.frames_superseded += 1;
            self.held = true;
            return false;
        }
        self.held = false;
        match self.write(look).await {
            Ok(wrote) => {
                self.backoff.cleared();
                if wrote {
                    self.pace.wrote(Instant::now());
                }
                wrote
            }
            Err(error) => {
                self.failed(&error);
                // A failed write leaves the device on an unknown look, so the
                // next one must not be compared against what this one meant to
                // send.
                self.sent = Sent::default();
                self.discard();
                self.backoff.failed(Instant::now());
                true
            }
        }
    }

    /// When the next write is due: the backoff decides it where a device
    /// failed, the pace where a look waits for it, and `refresh` otherwise.
    fn due(&self, wrote: bool, refresh: Instant) -> Instant {
        if let Some(retry) = self.backoff.ready() {
            return retry;
        }
        let next = if wrote {
            Instant::now() + self.timing.refresh
        } else {
            refresh
        };
        let next = match self.pace.ready() {
            Some(ready) if self.held => ready.min(next),
            _ => next,
        };
        // A fixture that waits to power off writes nothing until it does, so
        // no other wait would carry the pass that ends the wait.
        match self.off_due() {
            Some(off) => off.min(next),
            None => next,
        }
    }

    /// Send the current values once. Nothing acknowledges a LAN frame, so a
    /// lost one would otherwise hold a stale look until the desk changes it.
    async fn resend(&mut self) -> bool {
        let Some(look) = self.last.clone() else {
            return false;
        };
        // A look the pace held is the current one, and the device holds the
        // values around it. Writing it again is not a refresh.
        if !self.held {
            self.sent = Sent::default();
        }
        self.apply(&look).await
    }

    /// Apply what the patch asks for once the sender has gone quiet.
    ///
    /// `off` powers the fixture off on this pass and waits no off delay: a
    /// sender that went away is not a dimmer that dipped.
    async fn quiet(&mut self) -> bool {
        let Some(look) = self.last.as_ref().and_then(|last| last.quiet(self.loss)) else {
            return false;
        };
        if self.loss == SignalLoss::Off {
            self.dark = Dark::Due;
        }
        self.apply(&look).await
    }

    /// Forget a stream whose device stopped answering. Its counters are kept
    /// and the next write opens a new one: a disarming frame has nothing to
    /// reach.
    fn discard(&mut self) {
        let Some(stream) = self.stream.take() else {
            return;
        };
        self.counts.frames_sent += stream.frames_sent();
        self.counts.frames_superseded += stream.frames_superseded();
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
        self.mark();
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
        Personality::Full => return None,
        Personality::Segment => Resolution::App,
        Personality::Pixel => Resolution::Native,
    };
    Some(StreamOptions {
        resolution,
        rate: fixture.entry.max_hz.map_or(Rate::Measured, Rate::Fixed),
        gradient: false,
    })
}
