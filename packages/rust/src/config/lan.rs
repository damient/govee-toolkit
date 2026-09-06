//! Transport tuning for `lan`.
//!
//! The section is read whatever transports the build carries, so that one
//! configuration file works against all of them. What turns it into
//! `crate::lan::Options` is behind the `lan` feature, and the test at the
//! bottom is what keeps the numbers here and the transport's own defaults from
//! drifting apart.

use std::path::PathBuf;

use serde::Deserialize;

#[cfg(feature = "lan")]
use crate::error::Result;
use crate::transport::breaker::Policy;

/// Transport tuning for `lan`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LanConfig {
    /// Where discovery results are cached. Unset uses the default path;
    /// `false` in YAML disables the cache, which then lives in memory only.
    pub cache: Option<PathBuf>,
    /// Keep the cache in memory rather than on disk.
    pub cache_disabled: bool,
    /// How long a scan collects replies.
    pub scan_window_ms: u64,
    /// How often to rescan in the background. `null` scans only when asked.
    pub refresh_interval_seconds: Option<u64>,
    /// How long a status request waits for an answer.
    pub status_timeout_ms: u64,
    /// Shortest interval between two verifications of one device. `null`
    /// disables fire-and-verify, and with it everything the breaker learns.
    pub verify_interval_ms: Option<u64>,
    /// Forget a cached device that has not answered a scan for this long.
    pub forget_after_days: u64,
    /// Consecutive unanswered verifications that degrade the mode.
    pub degrade_after: u32,
    /// Consecutive unanswered verifications that take it down.
    pub down_after: u32,
    /// Consecutive answers that bring it back.
    pub recover_after: u32,
    /// How long a degraded mode waits before letting a probe through.
    pub cooldown_seconds: u64,
    /// How long a mode that is down waits.
    pub down_cooldown_seconds: u64,
}

impl Default for LanConfig {
    fn default() -> Self {
        let breaker = Policy::default();
        Self {
            cache: None,
            cache_disabled: false,
            scan_window_ms: 2_000,
            refresh_interval_seconds: Some(60),
            status_timeout_ms: 500,
            verify_interval_ms: Some(1_000),
            forget_after_days: 7,
            degrade_after: breaker.degrade_after,
            down_after: breaker.down_after,
            recover_after: breaker.recover_after,
            cooldown_seconds: breaker.cooldown.as_secs(),
            down_cooldown_seconds: breaker.down_cooldown.as_secs(),
        }
    }
}

impl LanConfig {
    /// The breaker thresholds this configuration asks for.
    #[must_use]
    pub fn policy(&self) -> Policy {
        Policy {
            degrade_after: self.degrade_after,
            down_after: self.down_after,
            recover_after: self.recover_after,
            cooldown: std::time::Duration::from_secs(self.cooldown_seconds),
            down_cooldown: std::time::Duration::from_secs(self.down_cooldown_seconds),
        }
    }

    /// The transport options this configuration asks for.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`](crate::Error::Transport) if the cache file cannot
    /// be read.
    #[cfg(feature = "lan")]
    pub fn transport_options(&self) -> Result<crate::lan::Options> {
        let cache = match self.cache_path() {
            Some(path) => crate::lan::Cache::load(path)?,
            None => crate::lan::Cache::in_memory(),
        };
        Ok(crate::lan::Options {
            policy: self.policy(),
            cache,
            scan_window: std::time::Duration::from_millis(self.scan_window_ms),
            refresh_interval: self
                .refresh_interval_seconds
                .map(std::time::Duration::from_secs),
            status_timeout: std::time::Duration::from_millis(self.status_timeout_ms),
            verify_interval: self
                .verify_interval_ms
                .map(std::time::Duration::from_millis),
            forget_after: std::time::Duration::from_secs(
                self.forget_after_days.saturating_mul(86_400),
            ),
            ..crate::lan::Options::default()
        })
    }

    /// Where the device cache belongs, or `None` to keep it in memory.
    #[cfg(feature = "lan")]
    #[must_use]
    pub fn cache_path(&self) -> Option<PathBuf> {
        if self.cache_disabled {
            return None;
        }
        Some(
            self.cache
                .clone()
                .unwrap_or_else(crate::paths::device_cache_file),
        )
    }
}

#[cfg(all(test, feature = "lan"))]
mod tests {
    use super::*;
    use crate::transport::millis;

    #[test]
    fn the_defaults_match_the_transport_they_configure() {
        let lan = LanConfig::default();
        let transport = crate::lan::Options::default();
        assert_eq!(lan.scan_window_ms, millis(transport.scan_window));
        assert_eq!(
            lan.refresh_interval_seconds,
            transport.refresh_interval.map(|d| d.as_secs())
        );
        assert_eq!(lan.status_timeout_ms, millis(transport.status_timeout));
        assert_eq!(
            lan.verify_interval_ms,
            transport.verify_interval.map(millis)
        );
        assert_eq!(
            lan.forget_after_days,
            transport.forget_after.as_secs() / 86_400
        );
        assert_eq!(lan.policy(), Policy::default());
    }
}
