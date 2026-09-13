//! How the `ble` transport is configured.

use std::time::Duration;

use crate::transport::breaker::Policy;

/// Configuration for [`Transport`](super::Transport).
#[derive(Debug, Clone)]
pub struct Options {
    /// Circuit breaker thresholds.
    pub policy: Policy,
    /// How long a scan listens when the caller names no window.
    pub scan_window: Duration,
    /// How long the second scan pass listens when the first heard nothing. A
    /// device that has just dropped a connection takes seconds to advertise
    /// again.
    pub rescan_window: Duration,
    /// How long a connection and its service discovery can take.
    pub connect_timeout: Duration,
    /// How long a status request waits for its answer.
    pub status_timeout: Duration,
    /// How long each step of the handshake that opens an encoded link
    /// waits for the device's answer. See `docs/protocol/ble.md` 9.
    pub handshake_timeout: Duration,
    /// The shortest interval between two verifications of the same device.
    /// `None` disables verification: the breaker then learns nothing.
    pub verify_interval: Option<Duration>,
    /// Sustained write budget for a device whose file records none, in frames
    /// per second. A file that records one wins over this value. Must be
    /// finite and above zero.
    /// [`Transport::start`](super::Transport::start) refuses any other value;
    /// it never clamps.
    pub writes_per_second: f64,
    /// How long [`Transport::close`](super::Transport::close) holds a link
    /// open, so that the last frame written on it goes out. A device file that
    /// records `measurements.ble.write_drain_ms` wins over this value.
    pub write_drain: Duration,
    /// How long the send path waits between two frames of one chunked
    /// command. It applies to no single-frame command, so it costs the
    /// ordinary send path nothing. See `docs/protocol/ble.md` 4.
    pub chunk_gap: Duration,
    /// How many frames may go out back to back before the budget applies. Must
    /// be at least one. It applies to every device: the device files record
    /// the count that stalled a unit, which is not a safe burst.
    pub burst: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            policy: Policy::default(),
            // Nobody measured how long a Govee device takes to advertise, so
            // the first pass listens as long as the second. A window too short
            // reports a device that is there as absent, which costs more than
            // the wait.
            scan_window: Duration::from_secs(5),
            rescan_window: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(10),
            status_timeout: Duration::from_secs(1),
            // One unit answered each step in about 60 ms; the margin is for a
            // busy radio.
            handshake_timeout: Duration::from_secs(2),
            verify_interval: Some(Duration::from_secs(1)),
            // A starting point for a unit nobody measured. One close pays it
            // once.
            write_drain: Duration::from_millis(50),
            // Measured on one H61A0, the only unit anybody measured, and a
            // starting point for any other. See `crate::ble::pace`.
            writes_per_second: 100.0,
            // The pace the phone controller keeps between two frames of a
            // chunked transfer, for every device. One unit dropped a transfer
            // paced at the write budget and accepted the same transfer at this
            // pace.
            chunk_gap: Duration::from_millis(300),
            burst: 16,
        }
    }
}
