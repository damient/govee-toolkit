//! What a stream needs, read off the device file.
//!
//! The file names both the commands and the arguments; `role:` is the only way
//! in, since neither name lives here. See `devices/schema.yaml`.

use crate::codec::frame::Token;
use crate::codec::{ArgRole, ArgSpec, Command, Device, Mode, Role};
use crate::error::{Error, Result};
use crate::stream::{Rate, StreamOptions, Zones};

/// How the device file paints zones over the chosen mode.
#[derive(Debug, Clone)]
pub(super) enum Painter {
    /// One frame carries every zone.
    Whole {
        /// The device file entry.
        command: String,
        /// The argument the colors go in.
        colors: String,
        /// The gradient argument and the value to send, where one is declared.
        gradient: Option<(String, i64)>,
    },
    /// One frame carries one color and the zones it applies to, so a repaint
    /// costs one write per distinct color.
    Masked {
        /// The device file entry.
        command: String,
        /// The argument the single color goes in.
        colors: String,
        /// The argument the zone indices go in.
        zones: String,
        /// How many zones the mask can name: the `count:` on the zone
        /// argument, or the width of the mask field where the file declares
        /// none.
        limit: usize,
    },
}

impl Painter {
    pub(super) fn command(&self) -> &str {
        match self {
            Self::Whole { command, .. } | Self::Masked { command, .. } => command,
        }
    }
}

/// The entry that arms and disarms the channel, where the mode has one.
#[derive(Debug, Clone)]
pub(super) struct Enable {
    /// The device file entry.
    pub(super) command: String,
    /// The argument the arming flag goes in.
    pub(super) arg: String,
}

/// The commands and the zone count a stream opens with.
#[derive(Debug)]
pub(super) struct Plan {
    /// `None` where the file declares no arming command for this mode, which
    /// is what a mode whose zones are always addressable looks like.
    pub(super) enable: Option<Enable>,
    /// The entry that sets zone interpolation, where the mode carries it in a
    /// frame of its own, and the value to send.
    pub(super) gradient: Option<(Enable, i64)>,
    pub(super) painter: Painter,
    pub(super) zones: usize,
}

/// Everything the device file has to say about a stream over `mode`.
pub(super) fn plan(device: &Device, mode: Mode, options: &StreamOptions) -> Result<Plan> {
    // A mode that names no arming entry has nothing to arm: over one that
    // paints by mask, the zones are addressable as soon as the device is on.
    // Do not invent a frame here — only the device file says what a device
    // does.
    let enable = match device.command_for(mode, Role::SegmentEnable) {
        Some(command) => Some(Enable {
            arg: arg_named(device, mode, command, ArgRole::Enable)?.to_owned(),
            command: command.to_owned(),
        }),
        None => None,
    };
    let painter = painter(device, mode, options.gradient)?;
    // Where the painting frame has no room for the setting, the file names a
    // command that carries it alone. Without that command the option encodes
    // into nothing, and the caller never gets the gradient it asked for.
    let gradient = match device.command_for(mode, Role::SegmentGradient) {
        Some(command) => Some((
            Enable {
                arg: arg_named(device, mode, command, ArgRole::Gradient)?.to_owned(),
                command: command.to_owned(),
            },
            i64::from(options.gradient),
        )),
        None => None,
    };
    let zones = zone_count(device, mode, &painter, options.zones)?;
    Ok(Plan {
        enable,
        gradient,
        painter,
        zones,
    })
}

/// Whichever of the two painting roles the file declares for `mode`.
///
/// A whole-frame command wins where both are declared: it paints the same
/// zones in one write.
fn painter(device: &Device, mode: Mode, gradient: bool) -> Result<Painter> {
    if let Some(command) = device.command_for(mode, Role::SegmentColor) {
        return Ok(Painter::Whole {
            colors: arg_named(device, mode, command, ArgRole::Colors)?.to_owned(),
            gradient: gradient_arg(device, mode, command, gradient),
            command: command.to_owned(),
        });
    }
    // A file claiming neither is reported against the whole-frame role:
    // `NoRoleCommand` names one role, and both point at the same file.
    let command = device
        .command_for(mode, Role::SegmentColorMasked)
        .ok_or_else(|| Error::NoRoleCommand {
            sku: device.sku.clone(),
            mode,
            role: Role::SegmentColor,
        })?;
    Ok(Painter::Masked {
        colors: arg_named(device, mode, command, ArgRole::Colors)?.to_owned(),
        zones: arg_named(device, mode, command, ArgRole::Zones)?.to_owned(),
        limit: mask_limit(device, mode, command).ok_or_else(|| Error::ZoneMaskUnbounded {
            sku: device.sku.clone(),
            mode,
            command: command.to_owned(),
        })?,
        command: command.to_owned(),
    })
}

/// How many zones the mask carries, where the file bounds it.
///
/// The `count:` on the zone argument is the bound; where the file declares
/// none, the width of the mask field the layout writes it into is, since a bit
/// past that field reaches no zone. `None` where the file says neither, and
/// the stream then refuses to open.
fn mask_limit(device: &Device, mode: Mode, command: &str) -> Option<usize> {
    let spec = device.commands.get(mode).get(command)?;
    let name = spec.arg_for(ArgRole::Zones)?;
    let declared = match spec.args.get(name)? {
        ArgSpec::Zones { count, .. } => *count,
        _ => None,
    };
    declared.or_else(|| mask_bits(command, spec, name))
}

/// How many zones the mask field reading `arg` can name, from its width.
///
/// `None` for a layout that does not parse: the same command fails to encode,
/// and `crate::codec::validate` reports the file.
fn mask_bits(command: &str, spec: &Command, arg: &str) -> Option<usize> {
    let exchanges =
        crate::codec::exchange::exchanges(command, spec, &spec.parsed_exchanges).ok()??;
    exchanges
        .sends()
        .flat_map(crate::codec::Frame::tokens)
        .find_map(|token| match token {
            Token::Mask { name, width } if name == arg => Some(width * 8),
            _ => None,
        })
}

/// The zone count the stream carries, refused where the mode cannot address
/// it.
///
/// Zero means nobody recorded the count — for either capability, and for a
/// caller who asked for none. A stream armed on it would send frames the codec
/// refuses, and nothing reads that refusal.
fn zone_count(device: &Device, mode: Mode, painter: &Painter, zones: Zones) -> Result<usize> {
    if let (Painter::Masked { .. }, Zones::Native) = (painter, zones) {
        return Err(Error::NativeZonesUnreachable {
            sku: device.sku.clone(),
            mode,
        });
    }
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
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if let Painter::Masked { limit, .. } = painter
        && count > *limit
    {
        return Err(Error::ZoneCountUnsupported {
            sku: device.sku.clone(),
            mode,
            zones: count,
            limit: *limit,
        });
    }
    Ok(count)
}

/// The rate to send at, and a warning when nothing was measured for this mode.
pub(super) fn rate_hz(
    device: &Device,
    sku: &str,
    mode: Mode,
    zones: usize,
    rate: Rate,
    fallback: f64,
) -> f64 {
    match rate {
        Rate::Fixed(hz) => hz,
        Rate::Measured => device
            .measurements
            .clean_hz(mode, u32::try_from(zones).unwrap_or(u32::MAX))
            .unwrap_or_else(|| {
                tracing::warn!(
                    %sku,
                    %mode,
                    fallback_hz = fallback,
                    "no `measurements.frame_rate` for this unit on this mode; streaming at the fallback rate"
                );
                fallback
            }),
    }
}

/// The argument of `command` marked `role`.
fn arg_named<'a>(device: &'a Device, mode: Mode, command: &str, role: ArgRole) -> Result<&'a str> {
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
fn gradient_arg(
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::codec::Catalog;

    const MASKED: &str = include_str!("../../tests/fixtures/masked-zones.yaml");

    fn catalog() -> Catalog {
        Catalog::from_sources([("masked-zones.yaml", MASKED)]).expect("the device file parses")
    }

    fn planned(zones: Zones) -> Result<Plan> {
        let catalog = catalog();
        let device = catalog.device("HTEST3").expect("the SKU resolves");
        plan(
            device,
            Mode::Ble,
            &StreamOptions {
                zones,
                ..StreamOptions::default()
            },
        )
    }

    #[test]
    fn a_file_declaring_only_the_masked_role_paints_by_mask() {
        let plan = planned(Zones::App).unwrap();
        assert_eq!(plan.zones, 15);
        assert!(matches!(plan.painter, Painter::Masked { .. }));
        assert_eq!(plan.painter.command(), "paint");
    }

    #[test]
    fn native_resolution_is_refused_rather_than_masked() {
        // 42 pixels behind 15 zones: a mask names zones, and the firmware drops
        // the bits past the last one in silence.
        let error = planned(Zones::Native).expect_err("a mask reaches no pixel");
        assert_eq!(error.code(), "native_zones_unreachable");
    }

    #[test]
    fn more_zones_than_the_mask_names_are_refused() {
        let error = planned(Zones::Exact(20)).expect_err("the mask names 15");
        assert_eq!(error.code(), "zone_count_unsupported");
        assert_eq!(planned(Zones::Exact(15)).unwrap().zones, 15);
    }

    /// The same file with the mask bounded by nothing: no `count:` on the zone
    /// argument, and a layout that writes no mask field.
    const UNBOUNDED: &str = "
schema_version: 1
sku: \"HTEST4\"
family: \"test\"
name: \"Masked segment device, unbounded mask\"
capabilities:
  segments: { count: 15 }
modes:
  ble: { support: partial, capabilities: [\"segments\"] }
commands:
  ble:
    arm:
      documented: true
      role: segment_enable
      frame: \"33 05 15 ${on} <pad:20> <xor>\"
      args:
        on: { type: int, range: [0, 1], role: enable }
    paint:
      documented: true
      role: segment_color_masked
      frame: \"33 05 15 01 (${color}:rgb)×${n} <pad:20> <xor>\"
      args:
        n: { type: int, range: [1, 1] }
        color: { type: rgb_list, max_len: 1, role: colors }
        mask: { type: zones, role: zones }
";

    #[test]
    fn a_mask_the_file_bounds_by_nothing_refuses_to_open() {
        let catalog =
            Catalog::from_sources([("unbounded.yaml", UNBOUNDED)]).expect("the device file parses");
        let device = catalog.device("HTEST4").expect("the SKU resolves");
        let error = plan(device, Mode::Ble, &StreamOptions::default())
            .expect_err("nothing says how far the mask reaches");
        assert_eq!(error.code(), "zone_mask_unbounded");
    }

    #[test]
    fn the_rate_comes_from_the_row_measured_over_this_mode() {
        let catalog = catalog();
        let device = catalog.device("HTEST3").expect("the SKU resolves");
        let hz = rate_hz(device, "HTEST3", Mode::Ble, 15, Rate::Measured, 10.0);
        assert!((hz - 8.0).abs() < f64::EPSILON);

        // Nothing was measured over `lan`, and a `ble` row does not stand in
        // for it.
        let hz = rate_hz(device, "HTEST3", Mode::Lan, 15, Rate::Measured, 10.0);
        assert!((hz - 10.0).abs() < f64::EPSILON);
    }
}
