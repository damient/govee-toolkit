//! How often one fixture accepts a write.
//!
//! A zone personality streams, and the stream reads its rate from the device
//! file. A command carries no such measurement, so the bridge holds the
//! latest look and writes it at [`FALLBACK_HZ`] — see `docs/dmx.md` 3.

use std::time::Duration;

use tokio::time::Instant;

/// The rate a fixture takes commands at, where the patch states none. It is a
/// default and not a measurement: it is under the rate a desk sends at, and
/// smooth enough for a fade. Raise it with `max_hz`.
const FALLBACK_HZ: f64 = 30.0;

/// The wait one fixture keeps between two writes.
#[derive(Debug)]
pub(super) struct Pace {
    /// The shortest time between two writes.
    interval: Duration,
    /// When the next write is due, and `None` where one is due now.
    ready: Option<Instant>,
}

impl Pace {
    /// The pace for one fixture: what the patch states, else [`FALLBACK_HZ`].
    pub(super) fn new(max_hz: Option<f64>) -> Self {
        let hz = max_hz.filter(|hz| *hz > 0.0).unwrap_or(FALLBACK_HZ);
        Self {
            interval: Duration::from_secs_f64(1.0 / hz),
            ready: None,
        }
    }

    /// Whether the fixture must write nothing yet.
    pub(super) fn holds(&self, now: Instant) -> bool {
        self.ready.is_some_and(|ready| now < ready)
    }

    /// Take the next wait, after a write that went out.
    pub(super) fn wrote(&mut self, now: Instant) {
        self.ready = Some(now + self.interval);
    }

    /// When the next write is due, and `None` where one is due now.
    pub(super) fn ready(&self) -> Option<Instant> {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::Instant;

    use super::{FALLBACK_HZ, Pace};

    #[tokio::test]
    async fn the_first_write_waits_for_nothing() {
        let pace = Pace::new(None);
        assert!(!pace.holds(Instant::now()), "a rig writes at once");
    }

    #[tokio::test]
    async fn a_write_holds_the_next_one_for_the_interval() {
        let mut pace = Pace::new(Some(20.0));
        let now = Instant::now();
        pace.wrote(now);
        assert!(pace.holds(now + std::time::Duration::from_millis(49)));
        assert!(!pace.holds(now + std::time::Duration::from_millis(51)));
    }

    #[tokio::test]
    async fn a_patch_that_states_no_rate_takes_the_fallback() {
        let mut pace = Pace::new(None);
        let now = Instant::now();
        pace.wrote(now);
        let interval = std::time::Duration::from_secs_f64(1.0 / FALLBACK_HZ);
        assert_eq!(pace.ready(), Some(now + interval));
    }

    /// A rate of 0 divides by zero.
    #[tokio::test]
    async fn a_rate_of_zero_takes_the_fallback() {
        let mut pace = Pace::new(Some(0.0));
        let now = Instant::now();
        pace.wrote(now);
        let interval = std::time::Duration::from_secs_f64(1.0 / FALLBACK_HZ);
        assert_eq!(pace.ready(), Some(now + interval));
    }
}
