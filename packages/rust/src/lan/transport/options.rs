//! How the transport is configured, and what to do after a command is written.

use std::time::Duration;

use crate::codec::Encoded;
use crate::lan::breaker::Policy;
use crate::lan::cache::Cache;
use crate::lan::discovery::Endpoints;

/// How the transport is set up.
#[derive(Debug, Clone)]
pub struct Options {
    /// Where to send and listen. The defaults are the protocol's own ports.
    pub endpoints: Endpoints,
    /// Circuit breaker thresholds.
    pub policy: Policy,
    /// The device cache. [`Cache::in_memory`] by default — a caller that wants
    /// discovery to survive a restart passes [`Cache::load`].
    pub cache: Cache,
    /// How long a scan collects replies before returning.
    pub scan_window: Duration,
    /// How often to rescan in the background. `None` scans only when asked.
    pub refresh_interval: Option<Duration>,
    /// How long a status request waits for its answer.
    pub status_timeout: Duration,
    /// The shortest interval between two verifications of the same device.
    /// `None` disables verification: the breaker then learns nothing, which is
    /// what a caller streaming frames wants.
    pub verify_interval: Option<Duration>,
    /// Drop a cached device that has not answered a scan for this long.
    pub forget_after: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            endpoints: Endpoints::default(),
            policy: Policy::default(),
            cache: Cache::in_memory(),
            scan_window: Duration::from_secs(2),
            refresh_interval: Some(Duration::from_secs(60)),
            // The idle round-trip measured on real hardware is tens of
            // milliseconds (`devices/H61A0.yaml`); half a second is silence,
            // not slowness.
            status_timeout: Duration::from_millis(500),
            verify_interval: Some(Duration::from_secs(1)),
            forget_after: Duration::from_secs(7 * 24 * 3600),
        }
    }
}

/// What to do about a command once it has been written to the socket.
#[derive(Debug, Clone, Copy)]
pub enum Verify<'a> {
    /// Nothing. The breaker learns nothing from this command — right for a
    /// stream of frames, where the verification traffic would compete with the
    /// frames themselves.
    None,
    /// Ask the device for its status afterwards, and record the answer, or its
    /// absence, against the breaker. The request is supplied by the caller
    /// because building it means reading the device file, which is the codec's
    /// job and not this crate's.
    With(&'a Encoded),
}
