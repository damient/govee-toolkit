//! The one-line summary a binding prints for a value the core reports.
//!
//! [`Style`] carries the literals a host language writes, so one body answers
//! every language.

use crate::transport::{Health, Reply};
use crate::{Device, DeviceStatus, Served};

/// How a host language writes a boolean and a value the device did not
/// report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Style {
    /// `true`, `false` and `null`.
    Javascript,
    /// `True`, `False` and `None`.
    Python,
}

impl Style {
    /// `value` as this language writes it.
    #[must_use]
    pub const fn boolean(self, value: bool) -> &'static str {
        match (self, value) {
            (Self::Javascript, true) => "true",
            (Self::Javascript, false) => "false",
            (Self::Python, true) => "True",
            (Self::Python, false) => "False",
        }
    }

    /// What this language writes where the device reported nothing.
    #[must_use]
    pub const fn absent(self) -> &'static str {
        match self {
            Self::Javascript => "null",
            Self::Python => "None",
        }
    }

    /// An optional boolean, or [`Style::absent`] where there is none.
    #[must_use]
    pub const fn optional_boolean(self, value: Option<bool>) -> &'static str {
        match value {
            Some(value) => self.boolean(value),
            None => self.absent(),
        }
    }
}

/// What a binding prints for one value: the type's name, and the fields that
/// tell two values apart.
pub trait Summary {
    /// This value in one line, with the literals `style` writes.
    fn summary(&self, style: Style) -> String;
}

impl Summary for Health {
    fn summary(&self, style: Style) -> String {
        format!(
            "Health(state='{}', failures={}, available={})",
            self.state,
            self.failures,
            style.boolean(self.available)
        )
    }
}

impl Summary for Device {
    fn summary(&self, _style: Style) -> String {
        format!("Device(id='{}', sku='{}')", self.id, self.sku)
    }
}

impl Summary for Served {
    fn summary(&self, _style: Style) -> String {
        format!(
            "Served(id='{}', mode='{}', command='{}')",
            self.id, self.mode, self.command
        )
    }
}

impl Summary for DeviceStatus {
    fn summary(&self, style: Style) -> String {
        format!(
            "DeviceStatus(id='{}', on={})",
            self.id,
            style.optional_boolean(self.on)
        )
    }
}

impl Summary for Reply {
    fn summary(&self, _style: Style) -> String {
        format!("Reply(id='{}')", self.id)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::transport::{DeviceId, State};

    fn id() -> DeviceId {
        DeviceId::new("aa:bb:cc:dd:ee:ff")
    }

    fn status(on: Option<bool>) -> DeviceStatus {
        DeviceStatus {
            id: id(),
            on,
            brightness: None,
            color: None,
            color_temp_kelvin: None,
            raw: Value::Null,
        }
    }

    #[test]
    fn each_language_writes_its_own_literals() {
        assert_eq!(
            status(Some(true)).summary(Style::Javascript),
            "DeviceStatus(id='AA:BB:CC:DD:EE:FF', on=true)"
        );
        assert_eq!(
            status(Some(true)).summary(Style::Python),
            "DeviceStatus(id='AA:BB:CC:DD:EE:FF', on=True)"
        );
        assert_eq!(
            status(None).summary(Style::Javascript),
            "DeviceStatus(id='AA:BB:CC:DD:EE:FF', on=null)"
        );
        assert_eq!(
            status(None).summary(Style::Python),
            "DeviceStatus(id='AA:BB:CC:DD:EE:FF', on=None)"
        );
    }

    #[test]
    fn a_health_summary_carries_the_three_fields() {
        let health = Health {
            state: State::Ok,
            failures: 0,
            available: true,
        };

        assert_eq!(
            health.summary(Style::Python),
            "Health(state='ok', failures=0, available=True)"
        );
    }
}
