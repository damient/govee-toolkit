//! Transport tuning for `cloud`, and where the API key comes from. Read
//! whatever transports the build carries, so one configuration file works
//! against all of them.

use std::path::PathBuf;

use serde::Deserialize;

use crate::env::Env;
use crate::transport::breaker::Policy;

/// The environment variable the key is read from.
pub const KEY_ENV: &str = "GOVEE_API_KEY";

/// Transport tuning for `cloud`. The key itself is **not** a field here:
/// `config.yaml` ends up in bug reports. See `docs/security.md`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CloudConfig {
    /// A file holding the API key alone. Read when [`KEY_ENV`] carries
    /// nothing.
    pub key_file: Option<PathBuf>,
    /// Unset uses the documented base URL.
    pub base_url: Option<String>,
    /// How long a request waits for its answer.
    pub request_timeout_ms: u64,
    /// The shortest interval between two requests about one device.
    pub min_interval_ms: u64,
    /// How long a command may wait for its slot before it fails.
    pub max_wait_ms: u64,
    /// Consecutive failures that degrade the mode.
    pub degrade_after: u32,
    /// Consecutive failures that take it down.
    pub down_after: u32,
    /// Consecutive answers that bring it back.
    pub recover_after: u32,
    /// How long a degraded mode waits before it lets a probe through.
    pub cooldown_seconds: u64,
    /// The same, for a mode that is down.
    pub down_cooldown_seconds: u64,
}

impl Default for CloudConfig {
    fn default() -> Self {
        let breaker = Policy::default();
        Self {
            key_file: None,
            base_url: None,
            request_timeout_ms: 10_000,
            min_interval_ms: 6_000,
            max_wait_ms: 15_000,
            degrade_after: breaker.degrade_after,
            down_after: breaker.down_after,
            recover_after: breaker.recover_after,
            cooldown_seconds: breaker.cooldown.as_secs(),
            down_cooldown_seconds: breaker.down_cooldown.as_secs(),
        }
    }
}

impl CloudConfig {
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

    /// The API key, from [`KEY_ENV`] or from the file the configuration
    /// names, in that order. `None` where neither carries one, which is not
    /// an error: this mode is opt-in.
    ///
    /// `env` is [`Config::env`](crate::Config::env), so the process
    /// environment and `.env` both reach here.
    ///
    /// # Errors
    ///
    /// [`Error::LocalDevices`](crate::Error::LocalDevices) on a file that
    /// cannot be read.
    pub fn key(&self, env: &Env) -> crate::error::Result<Option<String>> {
        if let Some(key) = env.var(KEY_ENV)
            && !key.trim().is_empty()
        {
            return Ok(Some(key.trim().to_owned()));
        }
        let Some(path) = &self.key_file else {
            return Ok(None);
        };
        let text =
            std::fs::read_to_string(path).map_err(|e| crate::error::Error::LocalDevices {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
        let key = text.trim().to_owned();
        Ok((!key.is_empty()).then_some(key))
    }

    /// What to set when no key is available, or `None` when one is. A key
    /// file that cannot be read answers as a missing key.
    pub(crate) fn missing_key(&self, env: &Env) -> Option<&'static str> {
        match self.key(env) {
            Ok(Some(_)) => None,
            Ok(None) | Err(_) => {
                Some("set GOVEE_API_KEY, put it in `.env`, or set `cloud.key_file`")
            }
        }
    }

    /// The transport options this configuration asks for, or `None` where no
    /// key is available.
    ///
    /// # Errors
    ///
    /// As for [`CloudConfig::key`].
    #[cfg(feature = "cloud")]
    pub fn transport_options(
        &self,
        env: &Env,
    ) -> crate::error::Result<Option<crate::cloud::Options>> {
        let Some(key) = self.key(env)? else {
            return Ok(None);
        };
        Ok(Some(crate::cloud::Options {
            key,
            base_url: self
                .base_url
                .clone()
                .unwrap_or_else(|| crate::cloud::BASE_URL.to_owned()),
            policy: self.policy(),
            request_timeout: std::time::Duration::from_millis(self.request_timeout_ms),
            min_interval: std::time::Duration::from_millis(self.min_interval_ms),
            max_wait: std::time::Duration::from_millis(self.max_wait_ms),
        }))
    }
}

#[cfg(all(test, feature = "cloud"))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::transport::millis;

    #[test]
    fn a_configuration_with_no_key_names_what_to_set() {
        let cloud = CloudConfig::default();
        let remedy = cloud
            .missing_key(&Env::default())
            .expect("no key, so a remedy");
        assert!(
            remedy.contains(KEY_ENV),
            "the remedy names {KEY_ENV}: {remedy}"
        );
    }

    #[test]
    fn the_environment_supplies_the_key() {
        let env = Env::from_pairs([(KEY_ENV, "  a-key\n")]);
        let cloud = CloudConfig::default();
        assert_eq!(cloud.key(&env).expect("a key"), Some("a-key".to_owned()));
        assert_eq!(cloud.missing_key(&env), None);
    }

    #[test]
    fn a_key_file_supplies_the_key() {
        let mut file = std::env::temp_dir();
        file.push("govee-toolkit-test-key");
        std::fs::write(&file, "  a-key\n").expect("the temporary file is writable");
        let cloud = CloudConfig {
            key_file: Some(file.clone()),
            ..CloudConfig::default()
        };
        let env = Env::default();
        assert_eq!(
            cloud.key(&env).expect("the file reads"),
            Some("a-key".to_owned())
        );
        assert_eq!(cloud.missing_key(&env), None);
        std::fs::remove_file(&file).expect("the temporary file is removable");
    }

    #[test]
    fn the_environment_wins_over_the_key_file() {
        let cloud = CloudConfig {
            key_file: Some(PathBuf::from("/nonexistent/govee/api-key")),
            ..CloudConfig::default()
        };
        let env = Env::from_pairs([(KEY_ENV, "from-the-environment")]);
        assert_eq!(
            cloud.key(&env).expect("the file is never read"),
            Some("from-the-environment".to_owned())
        );
    }

    #[test]
    fn the_defaults_match_the_transport_they_configure() {
        let cloud = CloudConfig::default();
        let transport = crate::cloud::Options::default();
        assert_eq!(cloud.request_timeout_ms, millis(transport.request_timeout));
        assert_eq!(cloud.min_interval_ms, millis(transport.min_interval));
        assert_eq!(cloud.max_wait_ms, millis(transport.max_wait));
        assert_eq!(cloud.base_url, None);
    }
}
