//! A fake Govee device on UDP.
//!
//! It exists so the transport can be tested in CI, where there is no hardware:
//! it answers `scan` on the discovery port and answers status requests on the
//! control port, and it can be told to go silent, to answer late, or to drop
//! replies — which is what the circuit breaker's transitions need to be
//! exercised end to end.
//!
//! **It plays the wire, not the firmware.** It does not interpret writes: a
//! command that is not a request for status is recorded and acknowledged with
//! nothing, which is exactly what a real device does
//! (`docs/protocol/lan.md` §2.1). Modelling what each write means would be
//! re-implementing per-SKU semantics in Rust, which is the one thing this
//! project keeps in `devices/*.yaml`. Tests assert on [`Simulator::received`]
//! instead, and the status a device reports is set explicitly with
//! [`Simulator::set_status`].
//!
//! ```no_run
//! # async fn example() -> std::io::Result<()> {
//! let sim = govee_toolkit_sim::Simulator::start(govee_toolkit_sim::Options::loopback(
//!     "AA:BB:CC:DD:EE:FF",
//!     "H61A0",
//! ))
//! .await?;
//! println!(
//!     "scan on {}, control on {}",
//!     sim.scan_addr()?,
//!     sim.control_addr()?
//! );
//! sim.set_silent(true); // the device stops answering; the breaker degrades
//!
//! # Ok(())
//! # }
//! ```

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::UdpSocket;

use crate::serve::{Listen, bind};

/// The documented ports, repeated here so a simulator can be started without
/// depending on the transport crate. See `docs/protocol/lan.md` §1.
pub const DISCOVERY_PORT: u16 = 4001;
/// The port replies go to on a real network.
pub const REPLY_PORT: u16 = 4002;
/// The port commands arrive on.
pub const CONTROL_PORT: u16 = 4003;
/// The discovery multicast group.
pub const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// Ways to make the device behave badly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Faults {
    /// Answer nothing at all. A device that is off, or off the network.
    pub silent: bool,
    /// Drop one reply in every `n`. `Some(1)` drops all of them, which is
    /// [`Faults::silent`] with the requests still recorded.
    pub drop_one_in: Option<u32>,
    /// Wait this long before answering.
    pub latency: Duration,
}

/// How the simulator is set up.
#[derive(Debug, Clone)]
pub struct Options {
    /// The MAC it reports as its identity.
    pub id: String,
    /// The SKU it reports.
    pub sku: String,
    /// The address it advertises. Commands are sent there, so on loopback it
    /// must be the address the control socket is actually reachable at.
    pub advertised_ip: IpAddr,
    /// Where discovery requests are received.
    pub scan_bind: SocketAddr,
    /// Where commands are received.
    pub control_bind: SocketAddr,
    /// The group to join, so multicast discovery reaches it. `None` on
    /// loopback, where the client sends its request unicast.
    pub multicast_group: Option<Ipv4Addr>,
    /// The port replies are sent to. `None` answers whichever port the request
    /// came from, which is what a test on ephemeral ports needs; a real device
    /// always answers [`REPLY_PORT`].
    pub reply_port: Option<u16>,
    /// Firmware versions reported in a scan reply.
    pub firmware: Firmware,
    /// How it misbehaves.
    pub faults: Faults,
}

/// The four version strings a scan reply carries.
#[derive(Debug, Clone, Default)]
pub struct Firmware {
    /// `bleVersionHard`.
    pub ble_hardware: String,
    /// `bleVersionSoft`.
    pub ble_software: String,
    /// `wifiVersionHard`.
    pub wifi_hardware: String,
    /// `wifiVersionSoft`.
    pub wifi_software: String,
}

impl Options {
    /// A device on the loopback interface with ephemeral ports, answering the
    /// port each request came from. The shape every test wants.
    #[must_use]
    pub fn loopback(id: impl Into<String>, sku: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            sku: sku.into(),
            advertised_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            scan_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            control_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            multicast_group: None,
            reply_port: None,
            firmware: Firmware::default(),
            faults: Faults::default(),
        }
    }

    /// A device on the real ports, reachable from anything on the network.
    #[must_use]
    pub fn on_the_network(id: impl Into<String>, sku: impl Into<String>, ip: IpAddr) -> Self {
        Self {
            id: id.into(),
            sku: sku.into(),
            advertised_ip: ip,
            scan_bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT)),
            control_bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, CONTROL_PORT)),
            multicast_group: Some(MULTICAST_GROUP),
            reply_port: Some(REPLY_PORT),
            firmware: Firmware::default(),
            faults: Faults::default(),
        }
    }
}

/// One datagram the device was sent.
#[derive(Debug, Clone, PartialEq)]
pub struct Received {
    /// Who sent it.
    pub from: SocketAddr,
    /// `msg.cmd`.
    pub cmd: String,
    /// `msg.data`.
    pub data: serde_json::Value,
}

impl Received {
    /// Whether this reads as a request for status: an envelope carrying no
    /// arguments. Every documented write carries at least one
    /// (`docs/protocol/lan.md` §1), so the distinction needs no command names.
    #[must_use]
    pub fn is_status_request(&self) -> bool {
        match &self.data {
            serde_json::Value::Object(map) => map.is_empty(),
            serde_json::Value::Null => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct State {
    pub(crate) faults: Faults,
    pub(crate) status: serde_json::Value,
    pub(crate) received: Vec<Received>,
    pub(crate) replies: u32,
}

/// A running fake device.
#[derive(Debug, Clone)]
pub struct Simulator {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) options: Options,
    pub(crate) scan: UdpSocket,
    pub(crate) control: UdpSocket,
    pub(crate) state: Mutex<State>,
}

impl Simulator {
    /// Bind both sockets and start answering.
    ///
    /// The tasks stop when the last clone of the returned handle is dropped.
    ///
    /// # Errors
    ///
    /// Whatever binding the sockets or joining the group returns.
    // Nothing here awaits, but the listeners are spawned onto the current
    // runtime: `async` is what states that there has to be one.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    pub async fn start(options: Options) -> std::io::Result<Self> {
        let scan = bind(options.scan_bind, options.multicast_group)?;
        let control = bind(options.control_bind, None)?;

        let inner = Arc::new(Inner {
            options,
            scan,
            control,
            state: Mutex::new(State {
                faults: Faults::default(),
                status: serde_json::json!({}),
                received: Vec::new(),
                replies: 0,
            }),
        });
        if let Ok(mut state) = inner.state.lock() {
            state.faults = inner.options.faults;
        }

        for socket in [Listen::Scan, Listen::Control] {
            let inner = Arc::clone(&inner);
            tokio::spawn(async move { inner.serve(socket).await });
        }

        Ok(Self { inner })
    }

    /// Where discovery requests are accepted.
    ///
    /// # Errors
    ///
    /// Whatever the socket reports.
    pub fn scan_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.scan.local_addr()
    }

    /// Where commands are accepted.
    ///
    /// # Errors
    ///
    /// Whatever the socket reports.
    pub fn control_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.control.local_addr()
    }

    /// Stop answering, or start again. The requests are still recorded.
    pub fn set_silent(&self, silent: bool) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.faults.silent = silent;
        }
    }

    /// Replace the faults wholesale.
    pub fn set_faults(&self, faults: Faults) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.faults = faults;
        }
    }

    /// Set what the device reports when asked for its status.
    ///
    /// This is `msg.data` verbatim, so a test decides exactly what a firmware
    /// answers — including a partial or unexpected shape.
    pub fn set_status(&self, status: serde_json::Value) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.status = status;
        }
    }

    /// Every datagram received so far, oldest first.
    #[must_use]
    pub fn received(&self) -> Vec<Received> {
        self.inner
            .state
            .lock()
            .map(|s| s.received.clone())
            .unwrap_or_default()
    }

    /// How many datagrams have been received.
    #[must_use]
    pub fn received_count(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|s| s.received.len())
            .unwrap_or_default()
    }

    /// Forget what has been received.
    pub fn clear(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.received.clear();
        }
    }
}

mod serve;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn a_write_is_not_a_status_request() {
        let write = Received {
            from: SocketAddr::from(([127, 0, 0, 1], 1)),
            cmd: "turn".to_owned(),
            data: serde_json::json!({ "value": 1 }),
        };
        assert!(!write.is_status_request());

        let read = Received {
            data: serde_json::json!({}),
            ..write.clone()
        };
        assert!(read.is_status_request());
    }
}
