//! What the facade reports: its event stream, and the two shapes an
//! application reads devices through.
//!
//! [`Event::to_json`] is the record every surface prints, and [`Event`]'s
//! [`fmt::Display`] is the one line a text surface prints. Both live here, so
//! a new variant fails the build until both render it.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Value, json};

use crate::codec::Mode;
use crate::transport::{DeviceId, Event as TransportEvent, Health};

/// One stream for the whole SDK: what the transports report, plus what only
/// the facade can see.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// Something a transport reported. Each one carries the mode it is
    /// about.
    Transport(crate::transport::Event),
    /// A device answered with a SKU the catalog does not know. Pin a known
    /// SKU in the configuration, or add a device file — `devices/README.md`.
    UnknownSku {
        /// The device.
        id: DeviceId,
        /// What it reported.
        sku: String,
    },
}

impl Event {
    /// This event as the record every surface prints. `event` names which one
    /// it is.
    #[must_use]
    pub fn to_json(&self) -> Value {
        match self {
            Self::UnknownSku { id, sku } => json!({
                "event": "unknown_sku",
                "id": id.to_string(),
                "sku": sku,
            }),
            Self::Transport(TransportEvent::Discovered {
                mode,
                device,
                change,
            }) => json!({
                "event": "discovered",
                "mode": mode.to_string(),
                "id": device.id.to_string(),
                "sku": device.sku,
                "endpoint": device.endpoint,
                "firmware": device.firmware,
                "change": change.to_string(),
            }),
            Self::Transport(TransportEvent::Forgotten { mode, id }) => json!({
                "event": "forgotten",
                "mode": mode.to_string(),
                "id": id.to_string(),
            }),
            Self::Transport(TransportEvent::Sent(sent)) => json!({
                "event": "sent",
                "mode": sent.mode.to_string(),
                "id": sent.id.to_string(),
                "cmd": sent.cmd,
                "endpoint": sent.endpoint,
            }),
            Self::Transport(TransportEvent::Status { mode, status }) => json!({
                "event": "status",
                "mode": mode.to_string(),
                "id": status.id.to_string(),
                "on": status.on,
                "brightness": status.brightness,
            }),
            Self::Transport(TransportEvent::HealthChanged {
                id,
                mode,
                transition,
            }) => json!({
                "event": "health_changed",
                "mode": mode.to_string(),
                "id": id.to_string(),
                "from": transition.from.to_string(),
                "to": transition.to.to_string(),
            }),
        }
    }

    /// The record for a subscription that fell behind and lost `missed`
    /// events. The stream drops the oldest, so a slow reader loses events
    /// rather than block the SDK. The gap is reported, never hidden.
    #[must_use]
    pub fn lagged(missed: u64) -> Value {
        json!({ "event": "lagged", "missed": missed })
    }

    /// The mode this event is about. `None` where it is about the device
    /// itself, whatever the mode.
    #[must_use]
    pub fn mode(&self) -> Option<Mode> {
        match self {
            Self::Transport(
                TransportEvent::Discovered { mode, .. }
                | TransportEvent::Forgotten { mode, .. }
                | TransportEvent::Status { mode, .. }
                | TransportEvent::HealthChanged { mode, .. },
            ) => Some(*mode),
            Self::Transport(TransportEvent::Sent(sent)) => Some(sent.mode),
            Self::UnknownSku { .. } => None,
        }
    }
}

impl fmt::Display for Event {
    /// One line for this event: the identity first, then the mode, then what
    /// happened. A field the device did not report is `?`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSku { id, sku } => write!(f, "{id}  unknown sku  {sku}"),
            Self::Transport(TransportEvent::Discovered {
                mode,
                device,
                change,
            }) => write!(
                f,
                "{}  {mode}  discovered  {change}  {}  {}",
                device.id, device.sku, device.endpoint
            ),
            Self::Transport(TransportEvent::Forgotten { mode, id }) => {
                write!(f, "{id}  {mode}  forgotten")
            }
            Self::Transport(TransportEvent::Sent(sent)) => write!(
                f,
                "{}  {}  sent  {}  {}",
                sent.id, sent.mode, sent.cmd, sent.endpoint
            ),
            Self::Transport(TransportEvent::Status { mode, status }) => write!(
                f,
                "{}  {mode}  status  on={}  brightness={}",
                status.id,
                reported(status.on),
                reported(status.brightness),
            ),
            Self::Transport(TransportEvent::HealthChanged {
                id,
                mode,
                transition,
            }) => write!(
                f,
                "{id}  {mode}  health  {} -> {}",
                transition.from, transition.to
            ),
        }
    }
}

/// A value the device reported, or `?` where it reported none.
fn reported(value: Option<impl fmt::Display>) -> String {
    value.map_or_else(|| "?".to_owned(), |value| value.to_string())
}

/// A command that was served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Served {
    /// The device it went to.
    pub id: DeviceId,
    /// The mode that served it.
    pub mode: Mode,
    /// The device file entry that was sent.
    pub command: String,
    /// The `msg.cmd` over `lan`, or the device file's entry name where the
    /// wire carries no name.
    pub cmd: String,
}

/// A device the facade knows about.
#[derive(Debug, Clone)]
pub struct Device {
    /// Its identity.
    pub id: DeviceId,
    /// The SKU it will be encoded under: the one the configuration pins, or the
    /// one it reports.
    pub sku: String,
    /// The name the configuration gives it, if any.
    pub name: Option<String>,
    /// The enabled modes, in preference order.
    pub modes: Vec<Mode>,
    /// Its health per enabled mode. A mode is absent when this build carries
    /// no transport for it, or when that transport has heard nothing.
    pub health: BTreeMap<Mode, Health>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::DeviceStatus;

    fn id() -> DeviceId {
        DeviceId::new("aa:bb:cc:dd:ee:ff")
    }

    #[test]
    fn a_status_line_marks_what_the_device_did_not_report() {
        let event = Event::Transport(TransportEvent::Status {
            mode: Mode::Lan,
            status: DeviceStatus {
                id: id(),
                on: Some(true),
                brightness: None,
                color: None,
                color_temp_kelvin: None,
                raw: Value::Null,
            },
        });

        assert_eq!(
            event.to_string(),
            "AA:BB:CC:DD:EE:FF  lan  status  on=true  brightness=?"
        );
    }

    #[test]
    fn an_unknown_sku_line_names_the_device_and_what_it_reported() {
        let event = Event::UnknownSku {
            id: id(),
            sku: "H6199".to_owned(),
        };

        assert_eq!(event.to_string(), "AA:BB:CC:DD:EE:FF  unknown sku  H6199");
    }
}
