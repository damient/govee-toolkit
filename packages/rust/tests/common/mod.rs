//! The loopback rig both integration suites are built on.
//!
//! Wiring a transport to a simulator means agreeing on four addresses. Keeping
//! that agreement in one place is what stops the two suites from drifting when
//! `Endpoints` changes shape.

#![allow(dead_code, clippy::expect_used)]

use std::net::{Ipv4Addr, SocketAddr};

use govee_toolkit::lan::{DeviceId, Endpoints};
use govee_toolkit_sim::Simulator;

pub(crate) const MAC: &str = "AA:BB:CC:DD:EE:FF";
pub(crate) const SKU: &str = "H61A0";

pub(crate) fn id() -> DeviceId {
    DeviceId::new(MAC)
}

/// One simulated device, on the loopback with ephemeral ports.
pub(crate) async fn simulator() -> Simulator {
    Simulator::start(govee_toolkit_sim::Options::loopback(MAC, SKU))
        .await
        .expect("the simulator binds")
}

/// Point a transport at that simulator, with no multicast involved.
pub(crate) fn endpoints(simulator: &Simulator) -> Endpoints {
    Endpoints {
        scan_target: simulator.scan_addr().expect("scan address"),
        reply_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        control_port: simulator.control_addr().expect("control address").port(),
        multicast_group: None,
    }
}
