//! The write budget.
//!
//! The transport paces itself rather than trusting a caller: a firmware that is
//! written to faster than it can keep up does not answer with an error, it
//! stops answering, and the caller sees a device that has gone away.
//!
//! What rate a device tolerates is a measurement. One unit has been measured —
//! `devices/H61A0.yaml` records about 130 writes per second sustained, and a
//! burst of roughly a hundred frames leaving the firmware unresponsive for
//! twenty seconds — and the default in [`Options`](super::Options) is that
//! unit's budget.
//!
//! TODO: read the budget out of the device file for the device being written
//! to, so that a unit tolerating less is not written to at another unit's
//! rate.
//!
//! A token bucket, with the tokens allowed to go negative: a caller that finds
//! the bucket empty is told how long to wait, and concurrent callers queue
//! behind each other instead of all waking at once.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::transport::error::{Error, Result};

/// A write budget that has been checked: a sustained rate in writes per second,
/// and how many of them may go out back to back.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    per_second: f64,
    burst: f64,
}

impl Budget {
    /// Check a budget.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] if the rate is not finite and positive, or if the
    /// burst is zero. Either would describe a budget that never releases a
    /// write, which on this wire is indistinguishable from a device that has
    /// stopped answering — so it is refused here rather than raised to
    /// something the caller did not ask for.
    pub fn new(per_second: f64, burst: u32) -> Result<Self> {
        if !per_second.is_finite() || per_second <= 0.0 {
            return Err(Error::Option {
                field: "writes_per_second".to_owned(),
                reason: format!("expected a finite rate above zero, got {per_second}"),
            });
        }
        if burst == 0 {
            return Err(Error::Option {
                field: "burst".to_owned(),
                reason: "expected at least one write, got 0".to_owned(),
            });
        }
        Ok(Self {
            per_second,
            burst: f64::from(burst),
        })
    }
}

/// A write budget, as the send path spends it.
#[derive(Debug)]
pub struct Pacer {
    budget: Budget,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    tokens: f64,
    last: Instant,
}

impl Pacer {
    /// A pacer spending `budget`, starting with a full burst.
    #[must_use]
    pub fn new(budget: Budget) -> Self {
        Self {
            budget,
            state: Mutex::new(State {
                tokens: budget.burst,
                last: Instant::now(),
            }),
        }
    }

    /// Claim one write, and return how long to wait before making it.
    ///
    /// The claim is taken immediately whatever the answer, so callers are
    /// served in the order they asked.
    #[must_use]
    pub fn claim(&self, now: Instant) -> Duration {
        let Ok(mut state) = self.state.lock() else {
            return Duration::ZERO;
        };
        let elapsed = now.saturating_duration_since(state.last).as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.budget.per_second).min(self.budget.burst);
        state.last = now;
        state.tokens -= 1.0;
        if state.tokens >= 0.0 {
            return Duration::ZERO;
        }
        Duration::try_from_secs_f64(-state.tokens / self.budget.per_second)
            .unwrap_or(Duration::ZERO)
    }

    /// Claim one write and wait for it to be due.
    pub async fn acquire(&self) {
        let wait = self.claim(Instant::now());
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn pacer(per_second: f64, burst: u32) -> Pacer {
        Pacer::new(Budget::new(per_second, burst).expect("a usable budget"))
    }

    #[test]
    fn the_burst_goes_out_at_once_and_the_next_write_waits() {
        let pacer = pacer(100.0, 4);
        let now = Instant::now();
        for _ in 0..4 {
            assert_eq!(pacer.claim(now), Duration::ZERO);
        }
        let wait = pacer.claim(now);
        assert!(
            (wait.as_secs_f64() - 0.01).abs() < 1e-6,
            "one write over the burst waits one interval, got {wait:?}"
        );
    }

    #[test]
    fn callers_past_the_budget_queue_rather_than_waking_together() {
        let pacer = pacer(100.0, 1);
        let now = Instant::now();
        assert_eq!(pacer.claim(now), Duration::ZERO);
        let first = pacer.claim(now);
        let second = pacer.claim(now);
        assert!(second > first, "{second:?} should be later than {first:?}");
    }

    #[test]
    fn an_idle_pacer_refills_no_further_than_the_burst() {
        let pacer = pacer(100.0, 3);
        let now = Instant::now();
        let later = now + Duration::from_secs(60);
        for _ in 0..3 {
            assert_eq!(pacer.claim(later), Duration::ZERO);
        }
        assert!(pacer.claim(later) > Duration::ZERO);
    }

    #[test]
    fn a_budget_that_could_never_release_a_write_is_refused() {
        for (rate, burst) in [(0.0, 4), (-1.0, 4), (f64::NAN, 4), (100.0, 0)] {
            let error = Budget::new(rate, burst).expect_err("{rate}/{burst} is not a budget");
            assert_eq!(error.code(), "out_of_range");
        }
    }
}
