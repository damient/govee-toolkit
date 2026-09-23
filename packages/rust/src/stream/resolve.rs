//! What a stream needs, read off the device file.
//!
//! The file names both the commands and the arguments; `role:` is the only way
//! in, since neither name lives here. See `devices/schema.yaml`.

use crate::codec::frame::Token;
use crate::codec::{ArgRole, ArgSpec, Command, Device, Mode, Role, Spread};
use crate::error::{Error, Result};
use crate::stream::StreamOptions;
use crate::stream::zones::zone_count;

/// How the device file paints zones over the chosen mode.
#[derive(Debug, Clone)]
pub(crate) enum Painter {
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
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Whole { command, .. } | Self::Masked { command, .. } => command,
        }
    }
}

/// The entry that arms and disarms the channel, where the mode has one.
#[derive(Debug, Clone)]
pub(crate) struct Enable {
    /// The device file entry.
    pub(crate) command: String,
    /// The argument the arming flag goes in.
    pub(crate) arg: String,
}

/// The commands and the zone count a stream opens with.
#[derive(Debug)]
pub(crate) struct Plan {
    /// `None` where the file declares no arming command for this mode, which
    /// is what a mode whose zones are always addressable looks like.
    pub(crate) enable: Option<Enable>,
    /// The entry that sets zone interpolation, where the mode carries it in a
    /// frame of its own, and the value to send.
    pub(crate) gradient: Option<(Enable, i64)>,
    pub(crate) painter: Painter,
    /// What the caller paints.
    pub(crate) zones: usize,
    pub(crate) spread: Option<Spread>,
}

impl Plan {
    pub(crate) fn width(&self) -> usize {
        self.spread.map_or(self.zones, |spread| {
            usize::try_from(spread.pixels()).unwrap_or(usize::MAX)
        })
    }
}

/// Everything the device file has to say about a stream over `mode`.
pub(crate) fn plan(device: &Device, mode: Mode, options: &StreamOptions) -> Result<Plan> {
    // A mode that names no arming entry has nothing to arm: over one that
    // paints by mask, the zones are addressable as soon as the device is on.
    let enable = match device.command_for(mode, Role::SegmentEnable) {
        Some(command) => Some(Enable {
            arg: arg_named(device, mode, command, ArgRole::Enable)?.to_owned(),
            command: command.to_owned(),
        }),
        None => None,
    };
    let painter = painter(device, mode, options.gradient)?;
    // Where the painting frame has no room for the setting, the file names a
    // command that carries it alone.
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
    let (zones, spread) = zone_count(device, mode, &painter, options.resolution)?;
    // A mode that can carry the setting nowhere fails rather than paint
    // hard-edged zones under a caller that asked for interpolation.
    if options.gradient && gradient.is_none() && !carries_gradient(&painter) {
        return Err(Error::NoRoleCommand {
            sku: device.sku.clone(),
            mode,
            role: Role::SegmentGradient,
        });
    }
    Ok(Plan {
        enable,
        gradient,
        painter,
        zones,
        spread,
    })
}

/// Whether the painting frame itself carries the gradient setting.
fn carries_gradient(painter: &Painter) -> bool {
    matches!(
        painter,
        Painter::Whole {
            gradient: Some(_),
            ..
        }
    )
}

/// Whichever of the two painting roles the file declares for `mode`.
///
/// A whole-frame command wins where both are declared: it paints the same
/// zones in one write.
pub(crate) fn painter(device: &Device, mode: Mode, gradient: bool) -> Result<Painter> {
    if let Some(command) = device.command_for(mode, Role::SegmentColor) {
        return Ok(Painter::Whole {
            colors: arg_named(device, mode, command, ArgRole::Colors)?.to_owned(),
            // A device file that marks no gradient argument gets nothing
            // extra: the codec refuses an argument the command does not
            // declare.
            gradient: arg_named(device, mode, command, ArgRole::Gradient)
                .ok()
                .map(|arg| (arg.to_owned(), i64::from(gradient))),
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
/// The `count:` on the zone argument, or the width of the mask field where the
/// file declares none. `None` where it declares neither, and the stream then
/// refuses to open.
pub(crate) fn mask_limit(device: &Device, mode: Mode, command: &str) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::codec::Catalog;
    use crate::stream::Resolution;

    const MASKED: &str = include_str!("../../tests/fixtures/masked-zones.yaml");

    fn catalog() -> Catalog {
        Catalog::from_sources([("masked-zones.yaml", MASKED)]).expect("the device file parses")
    }

    fn planned(resolution: Resolution) -> Result<Plan> {
        let catalog = catalog();
        let device = catalog.device("HTEST3").expect("the SKU resolves");
        plan(
            device,
            Mode::Ble,
            &StreamOptions {
                resolution,
                ..StreamOptions::default()
            },
        )
    }

    #[test]
    fn a_file_declaring_only_the_masked_role_paints_by_mask() {
        let plan = planned(Resolution::App).unwrap();
        assert_eq!(plan.zones, 15);
        assert!(matches!(plan.painter, Painter::Masked { .. }));
        assert_eq!(plan.painter.command(), "paint");
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
      role: segment_enable
      frame: \"33 05 15 ${on} <pad:20> <xor>\"
      args:
        on: { type: int, range: [0, 1], role: enable }
    paint:
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
}
