//! `send`: one device file entry, named as the file names it. The mode is
//! resolved before the values are read, because the same entry name can
//! declare different arguments on two modes.

use std::collections::BTreeMap;

use govee_toolkit::codec::{ArgSpec, Args, Mode};
use govee_toolkit::{DeviceId, Govee};

use crate::output::{Failure, Writer};
use crate::run::args;
use crate::run::verbs::report;

pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    id: &DeviceId,
    command: &str,
    pairs: &[String],
) -> Result<(), Failure> {
    let handle = govee.device(id);
    let mode = handle.serving_mode()?;
    let device = handle.spec()?;

    // An entry this mode does not carry goes out with no arguments, so that
    // the codec reports the unknown command rather than an unknown argument
    // of it.
    let values = match device.commands.get(mode).get(command) {
        Some(spec) => read(&spec.args, mode, command, pairs)?,
        None => Args::new(),
    };

    let served = handle.send(command, &values).await?;
    report(writer, &served);
    Ok(())
}

fn read(
    declared: &BTreeMap<String, ArgSpec>,
    mode: Mode,
    command: &str,
    pairs: &[String],
) -> Result<Args, Failure> {
    let mut values = Args::new();
    for pair in pairs {
        let (name, text) = pair
            .split_once('=')
            .ok_or_else(|| Failure::usage(format!("`{pair}` is not `name=value`")))?;
        let spec = declared.get(name).ok_or_else(|| {
            Failure::usage(format!(
                "`commands.{mode}.{command}` declares no argument `{name}`; it declares {}",
                names(declared)
            ))
        })?;
        values.insert(name, args::parse(name, spec, text)?);
    }
    Ok(values)
}

fn names(declared: &BTreeMap<String, ArgSpec>) -> String {
    if declared.is_empty() {
        return "none".to_owned();
    }
    declared
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
