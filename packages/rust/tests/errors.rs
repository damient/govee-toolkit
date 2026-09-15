//! The family every failure reports.
//!
//! A binding maps [`Category`] to the error class it raises, so a variant that
//! falls in the wrong family changes what a caller catches.

use govee_toolkit::codec::{Mode, Role};
use govee_toolkit::{Category, DeviceId, Error};

fn id() -> DeviceId {
    DeviceId::new("AA:BB:CC:DD:EE:FF")
}

#[test]
fn a_command_that_was_never_sent_is_a_codec_failure() {
    let errors = [
        Error::NoRoleCommand {
            sku: "H6199".to_owned(),
            mode: Mode::Lan,
            role: Role::Status,
        },
        Error::ZoneOutOfRange { index: 9, zones: 4 },
        Error::StreamRateOutOfRange { hz: 0.0 },
        Error::ZoneCountUnknown {
            sku: "H6199".to_owned(),
        },
    ];
    for error in &errors {
        assert_eq!(error.category(), Category::Codec, "{error}");
    }
}

#[test]
fn a_link_that_failed_is_a_transport_failure() {
    let errors = [
        Error::NoModeAvailable {
            id: id(),
            modes: vec![Mode::Lan],
        },
        Error::ProvisionRefused {
            id: id(),
            status: 1,
        },
    ];
    for error in &errors {
        assert_eq!(error.category(), Category::Transport, "{error}");
    }
}

#[test]
fn a_setting_that_cannot_work_is_a_config_failure() {
    let errors = [
        Error::Config {
            path: "govee.toml".to_owned(),
            reason: "unreadable".to_owned(),
        },
        Error::ModeNotImplemented {
            id: id(),
            mode: Mode::Cloud,
        },
        Error::MissingCredential {
            id: id(),
            mode: Mode::Cloud,
            remedy: "set GOVEE_API_KEY",
        },
        Error::Env {
            path: ".env".to_owned(),
            reason: "unreadable".to_owned(),
        },
    ];
    for error in &errors {
        assert_eq!(error.category(), Category::Config, "{error}");
    }
}

#[test]
fn a_family_names_itself() {
    assert_eq!(Category::Codec.as_str(), "codec");
    assert_eq!(Category::Transport.to_string(), "transport");
}
