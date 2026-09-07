//! The write budget.
//!
//! A firmware written to faster than it keeps up does not answer with an
//! error: it stops answering, and the caller sees a device that has gone away.
//!
//! The rate is a measurement, so it lives in the device file.
//! [`Budgets::from_catalog`] reads `measurements.ble.write_budget_hz` per SKU;
//! a file recording none falls back to
//! [`Options::writes_per_second`](super::Options::writes_per_second). The
//! burst never comes from the file: `burst_frames_before_stall` is the count
//! that broke a unit, not one that is safe.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::codec::Catalog;
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
    /// burst is zero. Either is a budget that never releases a write, which on
    /// this wire looks exactly like a device that stopped answering.
    pub fn new(per_second: f64, burst: u32) -> Result<Self> {
        check_rate(per_second)?;
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

    /// The same burst allowance, at another sustained rate.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] if the rate is not finite and positive.
    pub fn at_rate(self, per_second: f64) -> Result<Self> {
        check_rate(per_second)?;
        Ok(Self { per_second, ..self })
    }
}

/// Refuse a rate no write could go out under.
fn check_rate(per_second: f64) -> Result<()> {
    if !per_second.is_finite() || per_second <= 0.0 {
        return Err(Error::Option {
            field: "writes_per_second".to_owned(),
            reason: format!("expected a finite rate above zero, got {per_second}"),
        });
    }
    Ok(())
}

/// The sustained write rate each device file records, in writes per second.
///
/// Keyed by uppercased SKU, verified aliases included.
#[derive(Debug, Clone, Default)]
pub struct Budgets(BTreeMap<String, f64>);

impl Budgets {
    /// Every `measurements.ble.write_budget_hz` a catalog carries.
    ///
    /// A file recording none is absent here rather than defaulted.
    #[must_use]
    pub fn from_catalog(catalog: &Catalog) -> Self {
        let mut rates = BTreeMap::new();
        for sku in catalog.skus() {
            if let Ok(device) = catalog.device(sku)
                && let Some(hz) = device.measurements.ble.write_budget_hz
            {
                rates.insert(sku.to_owned(), hz);
            }
        }
        Self(rates)
    }

    /// The rate recorded for `sku`, or `None` if its file records none.
    #[must_use]
    pub fn rate(&self, sku: &str) -> Option<f64> {
        self.0
            .get(sku)
            .or_else(|| self.0.get(&sku.to_uppercase()))
            .copied()
    }

    /// One budget per SKU: the rate its file records, at `fallback`'s burst.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] if a device file records a rate no write could go
    /// out under. `crate::codec::validate` refuses such a file, so this
    /// catches a local one that never went through it.
    pub fn checked(&self, fallback: Budget) -> Result<BTreeMap<String, Budget>> {
        self.0
            .iter()
            .map(|(sku, hz)| Ok((sku.clone(), fallback.at_rate(*hz)?)))
            .collect()
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
    /// A pacer that spends `budget`. It starts with a full burst.
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

    /// Claim one write, and return how long to wait before making it. The
    /// claim is taken immediately, so callers are served in order.
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

    const MEASURED: &str = "
schema_version: 1
sku: \"HTEST1\"
family: \"test\"
name: \"A unit somebody measured\"
aliases: [\"HTEST1A\"]
capabilities:
  power: {}
measurements:
  ble:
    sustained_writes_hz: 60
    write_budget_hz: 40
";

    const UNMEASURED: &str = "
schema_version: 1
sku: \"HTEST2\"
family: \"test\"
name: \"A unit nobody measured\"
capabilities:
  power: {}
";

    fn budgets(sources: &[(&str, &str)]) -> Budgets {
        let catalog = Catalog::from_sources(sources.iter().copied()).expect("the files parse");
        Budgets::from_catalog(&catalog)
    }

    #[test]
    fn the_rate_comes_off_the_device_file_and_covers_its_verified_aliases() {
        let budgets = budgets(&[("measured.yaml", MEASURED), ("unmeasured.yaml", UNMEASURED)]);
        assert_eq!(budgets.rate("HTEST1"), Some(40.0));
        assert_eq!(budgets.rate("htest1"), Some(40.0));
        assert_eq!(budgets.rate("HTEST1A"), Some(40.0));
    }

    #[test]
    fn a_unit_nobody_measured_gets_no_rate_from_another_one() {
        let budgets = budgets(&[("measured.yaml", MEASURED), ("unmeasured.yaml", UNMEASURED)]);
        assert_eq!(budgets.rate("HTEST2"), None);
        assert_eq!(budgets.rate("H0000"), None);
    }

    #[test]
    fn a_measured_rate_keeps_the_burst_it_is_checked_against() {
        let fallback = Budget::new(100.0, 8).expect("a usable budget");
        let checked = budgets(&[("measured.yaml", MEASURED)])
            .checked(fallback)
            .expect("40 writes a second is a budget");
        let measured = checked.get("HTEST1").copied().expect("it is recorded");
        assert!((measured.per_second - 40.0).abs() < 1e-6, "{measured:?}");
        assert!(
            (measured.burst - fallback.burst).abs() < 1e-6,
            "the burst comes from the fallback, got {measured:?}"
        );
    }

    #[test]
    fn a_device_file_recording_a_rate_nothing_could_be_sent_under_is_refused() {
        let broken = MEASURED.replace("write_budget_hz: 40", "write_budget_hz: 0");
        let error = budgets(&[("broken.yaml", broken.as_str())])
            .checked(Budget::new(100.0, 8).expect("a usable budget"))
            .expect_err("a rate of zero is not a budget");
        assert_eq!(error.code(), "out_of_range");
    }

    #[test]
    fn a_budget_that_could_never_release_a_write_is_refused() {
        for (rate, burst) in [(0.0, 4), (-1.0, 4), (f64::NAN, 4), (100.0, 0)] {
            let error = Budget::new(rate, burst).expect_err("{rate}/{burst} is not a budget");
            assert_eq!(error.code(), "out_of_range");
        }
    }
}
