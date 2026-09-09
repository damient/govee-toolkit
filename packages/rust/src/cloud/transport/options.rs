//! How the `cloud` transport is configured.

use std::time::Duration;

use crate::transport::breaker::Policy;

/// The settings [`Transport::start`](super::Transport::start) takes.
///
/// The API key has no default: this mode does nothing without one, and it is
/// never read from the configuration file. See `docs/security.md`.
#[derive(Debug, Clone)]
pub struct Options {
    /// The account's API key, for the `Govee-API-Key` header.
    pub key: String,
    /// Where the API lives. [`crate::cloud::BASE_URL`] unless a caller points
    /// this somewhere else.
    pub base_url: String,
    /// Circuit breaker thresholds.
    pub policy: Policy,
    /// How long a request waits for its answer.
    pub request_timeout: Duration,
    /// The shortest interval between two requests about one device. A command
    /// inside it waits for its slot, which is what throttling this mode means
    /// — see `docs/protocol/cloud.md`.
    pub min_interval: Duration,
    /// How long a command may wait for that slot. Past it the command fails
    /// rather than sits in a queue nobody can see.
    pub max_wait: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            key: String::new(),
            base_url: crate::cloud::BASE_URL.to_owned(),
            policy: Policy::default(),
            request_timeout: Duration::from_secs(10),
            // Govee documents ten requests a minute for one device. Nobody
            // confirmed that against a live account here, so the default is
            // the documented rate and not a measured one.
            min_interval: Duration::from_secs(6),
            max_wait: Duration::from_secs(15),
        }
    }
}
