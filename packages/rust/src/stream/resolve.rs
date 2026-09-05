//! What a stream needs, read off the device file.
//!
//! The file names both the commands and the arguments; `role:` is the only way
//! in, since neither name lives here. See `devices/schema.yaml`.

use crate::codec::{ArgRole, Device, Mode, Role};
use crate::error::{Error, Result};
use crate::stream::{Rate, Zones};

/// The device file entry claiming `role`.
pub(super) fn named(device: &Device, mode: Mode, role: Role) -> Result<&str> {
    device
        .command_for(mode, role)
        .ok_or_else(|| Error::NoRoleCommand {
            sku: device.sku.clone(),
            mode,
            role,
        })
}

/// Zero means nobody recorded the count — for either capability, and for a
/// caller who asked for none. A stream armed on it would send frames the codec
/// refuses, and the refusal would land where nothing is looking.
pub(super) fn zone_count(device: &Device, zones: Zones) -> Result<usize> {
    let count = match zones {
        Zones::App => device.capabilities.segment_count().unwrap_or(0),
        Zones::Native => device.capabilities.native_pixels().unwrap_or(0),
        Zones::Exact(n) => u32::from(n),
    };
    if count == 0 {
        return Err(Error::ZoneCountUnknown {
            sku: device.sku.clone(),
        });
    }
    Ok(count.try_into().unwrap_or(usize::MAX))
}

/// The rate to send at, and a warning when nothing was measured.
pub(super) fn rate_hz(device: &Device, sku: &str, zones: usize, rate: Rate, fallback: f64) -> f64 {
    match rate {
        Rate::Fixed(hz) => hz,
        Rate::Measured => device
            .measurements
            .clean_hz(u32::try_from(zones).unwrap_or(u32::MAX))
            .unwrap_or_else(|| {
                tracing::warn!(
                    %sku,
                    fallback_hz = fallback,
                    "no `measurements.frame_rate` for this unit; streaming at the fallback rate"
                );
                fallback
            }),
    }
}

/// The argument of `command` marked `role`.
pub(super) fn arg_named<'a>(
    device: &'a Device,
    mode: Mode,
    command: &str,
    role: ArgRole,
) -> Result<&'a str> {
    device
        .commands
        .get(mode)
        .get(command)
        .and_then(|spec| spec.arg_for(role))
        .ok_or_else(|| Error::NoRoleArg {
            sku: device.sku.clone(),
            mode,
            command: command.to_owned(),
            arg_role: role,
        })
}

/// The argument marked [`ArgRole::Gradient`] and the value to send, if the
/// command declares one.
///
/// A device file that marks none gets nothing extra: the codec refuses an
/// argument the command does not declare.
pub(super) fn gradient_arg(
    device: &Device,
    mode: Mode,
    command: &str,
    gradient: bool,
) -> Option<(String, i64)> {
    let arg = device
        .commands
        .get(mode)
        .get(command)?
        .arg_for(ArgRole::Gradient)?;
    Some((arg.to_owned(), i64::from(gradient)))
}
