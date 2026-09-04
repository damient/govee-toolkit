//! Multicast discovery: the `scan` request and the replies it draws.
//!
//! Addresses and payload are the documented ones — `docs/protocol/lan.md` §1.
//! Nothing here touches a socket; [`crate::socket`] does that.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::DeviceId;

/// The multicast group discovery requests go to.
pub const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
/// The port discovery requests go to.
pub const DISCOVERY_PORT: u16 = 4001;
/// The port every reply — `scan` and `devStatus` alike — comes back on.
pub const REPLY_PORT: u16 = 4002;
/// The port commands go to, on the device's own address.
pub const CONTROL_PORT: u16 = 4003;

/// Where the transport sends and listens.
///
/// The constants above are the protocol. These are here because a test and the
/// simulator need ephemeral ports on the loopback: the defaults are what talks
/// to real hardware, and nothing else should change them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoints {
    /// Where the `scan` request is sent.
    pub scan_target: SocketAddr,
    /// Where replies are received.
    pub reply_bind: SocketAddr,
    /// The port on the device's own address that accepts commands.
    pub control_port: u16,
    /// The group to join on the receiving socket, if any.
    pub multicast_group: Option<Ipv4Addr>,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            scan_target: SocketAddr::from((MULTICAST_GROUP, DISCOVERY_PORT)),
            reply_bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, REPLY_PORT)),
            control_port: CONTROL_PORT,
            multicast_group: Some(MULTICAST_GROUP),
        }
    }
}

/// The discovery request, ready to send.
#[must_use]
pub fn scan_request() -> Vec<u8> {
    // Written out rather than built through serde_json: it is a constant of the
    // protocol, and `account_topic` is not something a caller supplies.
    br#"{"msg":{"cmd":"scan","data":{"account_topic":"reserve"}}}"#.to_vec()
}

/// One device that answered a `scan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    /// The `device` field of the reply: the MAC, and the identity everything
    /// else keys on. An IP address changes across a DHCP lease; this does not.
    pub id: DeviceId,
    /// Where to send commands.
    pub ip: IpAddr,
    /// The SKU the device reports. Not necessarily one the catalog knows.
    pub sku: String,
    /// `bleVersionHard`.
    pub ble_hardware: String,
    /// `bleVersionSoft`.
    pub ble_software: String,
    /// `wifiVersionHard`.
    pub wifi_hardware: String,
    /// `wifiVersionSoft`.
    pub wifi_software: String,
}

impl DiscoveredDevice {
    /// Read one out of a `scan` reply's `msg.data`.
    ///
    /// A reply missing `device`, `sku` or a parseable `ip` is not a device;
    /// firmware versions are reported empty by some units and are optional.
    #[must_use]
    pub fn from_data(data: &serde_json::Value) -> Option<Self> {
        let string = |key: &str| data.get(key).and_then(serde_json::Value::as_str);
        let owned = |key: &str| string(key).unwrap_or_default().to_owned();

        let id = DeviceId::new(string("device")?);
        if id.as_str().is_empty() {
            return None;
        }
        Some(Self {
            id,
            ip: string("ip")?.parse().ok()?,
            sku: string("sku")?.to_owned(),
            ble_hardware: owned("bleVersionHard"),
            ble_software: owned("bleVersionSoft"),
            wifi_hardware: owned("wifiVersionHard"),
            wifi_software: owned("wifiVersionSoft"),
        })
    }

    /// The address commands are sent to.
    #[must_use]
    pub fn control_address(&self, endpoints: &Endpoints) -> SocketAddr {
        SocketAddr::new(self.ip, endpoints.control_port)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn data(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("valid json")
    }

    #[test]
    fn the_request_is_the_documented_one() {
        let sent: serde_json::Value = serde_json::from_slice(&scan_request()).expect("valid json");
        assert_eq!(sent["msg"]["cmd"], "scan");
        assert_eq!(sent["msg"]["data"]["account_topic"], "reserve");
    }

    #[test]
    fn reads_a_reply_from_the_protocol_documentation() {
        let device = DiscoveredDevice::from_data(&data(
            r#"{"ip":"192.168.1.42","device":"aa:bb:cc:dd:ee:ff","sku":"H61A0",
                "bleVersionHard":"","bleVersionSoft":"","wifiVersionHard":"1.00.10",
                "wifiVersionSoft":"2.05.08"}"#,
        ))
        .expect("a well-formed reply");

        assert_eq!(device.id.as_str(), "AA:BB:CC:DD:EE:FF");
        assert_eq!(device.ip, IpAddr::from([192, 168, 1, 42]));
        assert_eq!(device.sku, "H61A0");
        assert_eq!(device.wifi_software, "2.05.08");
        assert!(device.ble_software.is_empty());
    }

    #[test]
    fn a_reply_without_an_identity_is_not_a_device() {
        assert!(
            DiscoveredDevice::from_data(&data(r#"{"ip":"192.168.1.42","sku":"H61A0"}"#)).is_none()
        );
        assert!(
            DiscoveredDevice::from_data(&data(r#"{"device":"","ip":"192.168.1.42","sku":"H"}"#))
                .is_none()
        );
        assert!(
            DiscoveredDevice::from_data(&data(r#"{"device":"aa","sku":"H","ip":"nope"}"#))
                .is_none()
        );
    }
}
