//! The per-device, per-mode circuit breaker.
//!
//! Every method takes the current instant instead of a clock read, so the
//! transitions in `docs/modes.md` are testable without a socket and without a
//! wait. The breaker answers from the last recorded result, never from the
//! network, so a mode costs no round-trip to choose.

use std::time::{Duration, Instant};

/// How healthy a mode is for one device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// Answering. Every command goes out.
    Ok,
    /// Consecutive failures crossed [`Policy::degrade_after`]. After a
    /// cooldown, the breaker allows one command as a probe.
    Degraded,
    /// Still failing after [`Policy::down_after`]. As `Degraded`, with a
    /// longer cooldown.
    Down,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Down => "down",
        })
    }
}

/// The thresholds behind the transitions.
///
/// The defaults follow `docs/modes.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Consecutive failures that take a healthy mode to `Degraded`.
    pub degrade_after: u32,
    /// Consecutive failures that take a degraded mode to `Down`.
    pub down_after: u32,
    /// Consecutive successes that bring a degraded mode back to `Ok`.
    pub recover_after: u32,
    /// How long `Degraded` waits before letting a probe through.
    pub cooldown: Duration,
    /// How long `Down` waits before letting a probe through.
    pub down_cooldown: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            degrade_after: 3,
            down_after: 6,
            recover_after: 2,
            cooldown: Duration::from_secs(30),
            down_cooldown: Duration::from_secs(300),
        }
    }
}

/// What a call to the breaker changed, if anything.
///
/// `docs/modes.md` requires every transition to be observable, so each call
/// returns one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// State before the call.
    pub from: State,
    /// State after it.
    pub to: State,
}

impl Transition {
    /// Whether the state moved.
    #[must_use]
    pub fn changed(self) -> bool {
        self.from != self.to
    }
}

/// One device's health in one mode.
#[derive(Debug, Clone)]
pub struct Breaker {
    policy: Policy,
    state: State,
    failures: u32,
    successes: u32,
    /// When a non-`Ok` breaker allows one probe.
    retry_at: Option<Instant>,
}

impl Default for Breaker {
    fn default() -> Self {
        Self::new(Policy::default())
    }
}

impl Breaker {
    /// A healthy breaker.
    #[must_use]
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            state: State::Ok,
            failures: 0,
            successes: 0,
            retry_at: None,
        }
    }

    /// The current state, without regard to the cooldown.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// Consecutive failures recorded so far.
    #[must_use]
    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// When this breaker allows the next probe, if it is not `Ok`.
    #[must_use]
    pub fn retry_at(&self) -> Option<Instant> {
        self.retry_at
    }

    /// Whether a command may be sent over this mode right now.
    ///
    /// `Ok` always passes. `Degraded` and `Down` pass one command after their
    /// cooldown, and refuse until then. That probe is the only route back to
    /// `Ok`. No network round-trip is involved.
    #[must_use]
    pub fn allows(&self, now: Instant) -> bool {
        match self.state {
            State::Ok => true,
            State::Degraded | State::Down => self.retry_at.is_none_or(|at| now >= at),
        }
    }

    /// Record a command the device answered.
    pub fn record_success(&mut self, now: Instant) -> Transition {
        let from = self.state;
        self.failures = 0;
        match self.state {
            State::Ok => {}
            State::Degraded | State::Down => {
                self.successes += 1;
                // A single answer is not recovery: a device that flaps would
                // read as healthy between two dropped commands.
                if self.successes >= self.policy.recover_after {
                    self.state = State::Ok;
                    self.successes = 0;
                    self.retry_at = None;
                } else {
                    // The next command is the second half of the same probe.
                    self.retry_at = Some(now);
                    if self.state == State::Down {
                        self.state = State::Degraded;
                    }
                }
            }
        }
        Transition {
            from,
            to: self.state,
        }
    }

    /// Record a command the device did not answer.
    pub fn record_failure(&mut self, now: Instant) -> Transition {
        let from = self.state;
        self.successes = 0;
        self.failures = self.failures.saturating_add(1);

        if self.failures >= self.policy.down_after {
            self.state = State::Down;
            self.retry_at = Some(now + self.policy.down_cooldown);
        } else if self.failures >= self.policy.degrade_after {
            self.state = State::Degraded;
            self.retry_at = Some(now + self.policy.cooldown);
        }

        Transition {
            from,
            to: self.state,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn healthy_until_the_threshold() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        for _ in 0..2 {
            assert!(!b.record_failure(now).changed());
            assert_eq!(b.state(), State::Ok);
            assert!(b.allows(now));
        }
        let t = b.record_failure(now);
        assert_eq!((t.from, t.to), (State::Ok, State::Degraded));
    }

    #[test]
    fn a_success_resets_the_run() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        b.record_failure(now);
        b.record_failure(now);
        b.record_success(now);
        assert_eq!(b.failures(), 0);
        b.record_failure(now);
        b.record_failure(now);
        assert_eq!(b.state(), State::Ok);
    }

    #[test]
    fn degraded_refuses_until_the_cooldown_elapses() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        for _ in 0..3 {
            b.record_failure(now);
        }
        assert_eq!(b.state(), State::Degraded);
        assert!(!b.allows(now));
        assert!(!b.allows(now + Duration::from_secs(29)));
        assert!(b.allows(now + Duration::from_secs(30)));
    }

    #[test]
    fn a_single_answer_is_not_recovery() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        for _ in 0..3 {
            b.record_failure(now);
        }
        let probe = now + Duration::from_secs(30);
        assert!(!b.record_success(probe).changed());
        assert_eq!(b.state(), State::Degraded);
        assert!(b.allows(probe));
        let t = b.record_success(probe);
        assert_eq!((t.from, t.to), (State::Degraded, State::Ok));
    }

    #[test]
    fn a_failed_probe_reaches_down() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        for _ in 0..6 {
            b.record_failure(now);
        }
        assert_eq!(b.state(), State::Down);
        assert!(!b.allows(now + Duration::from_secs(299)));
        assert!(b.allows(now + Duration::from_secs(300)));
    }

    #[test]
    fn down_climbs_back_through_degraded() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        for _ in 0..6 {
            b.record_failure(now);
        }
        let probe = now + Duration::from_secs(300);
        let t = b.record_success(probe);
        assert_eq!((t.from, t.to), (State::Down, State::Degraded));
        let t = b.record_success(probe);
        assert_eq!((t.from, t.to), (State::Degraded, State::Ok));
        assert_eq!(b.retry_at(), None);
    }

    #[test]
    fn failures_do_not_wrap() {
        let now = Instant::now();
        let mut b = Breaker::new(Policy::default());
        for _ in 0..1000 {
            b.record_failure(now);
        }
        assert_eq!(b.state(), State::Down);
        assert_eq!(b.failures(), 1000);
    }
}
