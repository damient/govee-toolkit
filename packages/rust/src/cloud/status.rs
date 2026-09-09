//! What a state answer reports, in the shape every mode reports it.
//!
//! The device file says which capability answers into which argument, and
//! that argument's `role:` says which field it fills. No capability name
//! reaches this code.

use crate::cloud::api::CapabilityState;
use crate::codec::{ArgRole, Encoded};
use crate::transport::{DeviceId, DeviceStatus};

/// Read a status out of what the API answered.
///
/// `request` carries the instance-to-argument map the device file declares,
/// and `roles` what each argument is. A capability nothing claims still
/// reaches the caller, under [`DeviceStatus::raw`].
pub(crate) fn read(id: DeviceId, request: &Encoded, answered: &[CapabilityState]) -> DeviceStatus {
    let reads = request
        .request
        .as_ref()
        .map(|request| request.reads.clone())
        .unwrap_or_default();

    let mut raw = serde_json::Map::new();
    let mut by_role: Vec<(ArgRole, serde_json::Value)> = Vec::new();
    for capability in answered {
        let Some(value) = capability.value() else {
            continue;
        };
        raw.insert(capability.instance.clone(), value.clone());
        if let Some(arg) = reads.get(&capability.instance)
            && let Some(role) = request.roles.get(arg)
        {
            by_role.push((*role, value.clone()));
        }
    }

    let int = |role: ArgRole| {
        by_role
            .iter()
            .find(|(claimed, _)| *claimed == role)
            .and_then(|(_, value)| value.as_i64())
    };
    let channel = |packed: i64, shift: u32| u8::try_from((packed >> shift) & 0xff).unwrap_or(0);
    let packed = int(ArgRole::Color);
    DeviceStatus {
        id,
        on: int(ArgRole::On).map(|v| v != 0),
        brightness: int(ArgRole::Brightness),
        color: packed.map(|value| [channel(value, 16), channel(value, 8), channel(value, 0)]),
        color_temp_kelvin: int(ArgRole::ColorTemp),
        raw: serde_json::Value::Object(raw),
    }
}
