//! When a fixture that failed is written to again.
//!
//! The wait doubles from [`FIRST`] to [`LONGEST`], and a write that lands
//! clears it — see `docs/dmx.md` 3.3.

use std::time::Duration;

use tokio::time::Instant;

/// How long a fixture waits after the first failed write.
const FIRST: Duration = Duration::from_millis(250);
/// The longest wait between two attempts. A device that comes back is picked
/// up inside this.
const LONGEST: Duration = Duration::from_secs(8);

/// The wait one fixture keeps between two attempts.
#[derive(Debug, Default)]
pub(super) struct Backoff {
    /// The current wait, and `None` where nothing has failed.
    wait: Option<Duration>,
    /// When the next attempt is due, and `None` where one is due now.
    ready: Option<Instant>,
}

impl Backoff {
    /// Whether the fixture must write nothing yet.
    pub(super) fn holds(&mut self, now: Instant) -> bool {
        match self.ready {
            Some(ready) if now < ready => true,
            _ => {
                self.ready = None;
                false
            }
        }
    }

    /// Take the next wait, after a write that did not land.
    pub(super) fn failed(&mut self, now: Instant) {
        let wait = self.wait.map_or(FIRST, |wait| (wait * 2).min(LONGEST));
        self.wait = Some(wait);
        self.ready = Some(now + wait);
    }

    /// Forget the wait, after a write that landed.
    pub(super) fn cleared(&mut self) {
        self.wait = None;
        self.ready = None;
    }

    /// When the next attempt is due, and `None` where one is due now.
    pub(super) fn ready(&self) -> Option<Instant> {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::Instant;

    use super::{Backoff, FIRST, LONGEST};

    #[tokio::test]
    async fn the_wait_doubles_to_the_cap() {
        let mut backoff = Backoff::default();
        let now = Instant::now();
        backoff.failed(now);
        assert_eq!(backoff.ready(), Some(now + FIRST));
        for _ in 0..10 {
            backoff.failed(now);
        }
        assert_eq!(backoff.ready(), Some(now + LONGEST));
    }

    #[tokio::test]
    async fn a_write_that_lands_clears_the_wait() {
        let mut backoff = Backoff::default();
        let now = Instant::now();
        backoff.failed(now);
        assert!(backoff.holds(now));
        backoff.cleared();
        assert!(!backoff.holds(now));
        backoff.failed(now);
        assert_eq!(backoff.ready(), Some(now + FIRST));
    }

    #[tokio::test]
    async fn the_wait_ends() {
        let mut backoff = Backoff::default();
        let now = Instant::now();
        backoff.failed(now);
        assert!(!backoff.holds(now + FIRST));
        assert_eq!(backoff.ready(), None);
    }
}
