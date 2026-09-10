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
    let reads = request.request.as_ref().map(|request| &request.reads);

    let mut raw = serde_json::Map::new();
    let mut by_role: Vec<(ArgRole, serde_json::Value)> = Vec::new();
    for capability in answered {
        let Some(value) = capability.value() else {
            continue;
        };
        raw.insert(capability.instance.clone(), value.clone());
        if let Some(arg) = reads.and_then(|reads| reads.get(&capability.instance))
            && let Some(role) = request.roles.get(arg)
        {
            by_role.push((*role, value.clone()));
        }
    }

    DeviceStatus::from_roles(id, serde_json::Value::Object(raw), |role| {
        by_role
            .iter()
            .find(|(claimed, _)| *claimed == role)
            .and_then(|(_, value)| value.as_i64())
    })
}
