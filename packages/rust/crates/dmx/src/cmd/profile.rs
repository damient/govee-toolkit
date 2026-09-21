//! `profile`: the channel table of one device, to patch a desk with.

use govee_toolkit::codec::{Catalog, Device};
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit_dmx::profile::{self, Personality, Profile};
use govee_toolkit_dmx::report;

use super::spellings;

pub(crate) fn run(sku: &str, personality: Option<&str>, writer: Writer) -> Result<(), Failure> {
    let catalog = Catalog::embedded().map_err(|error| Failure::usage(error.to_string()))?;
    let device = catalog
        .device(sku)
        .map_err(|error| Failure::usage(error.to_string()))?;
    let tables = tables(device, personality)?;
    writer.emit(
        &report::json(device, &tables),
        &report::text(device, &tables),
    );
    Ok(())
}

/// The tables to print: the one asked for, or every personality the device
/// serves. A personality wider than one universe stays in the list and
/// carries its error, because the operator has to see that it is the width
/// that refused it.
fn tables(
    device: &Device,
    personality: Option<&str>,
) -> Result<Vec<Result<Profile, profile::Error>>, Failure> {
    let Some(name) = personality else {
        return Ok(profile::served(device));
    };
    let personality = Personality::parse(name).ok_or_else(|| {
        Failure::usage(format!(
            "unknown personality `{name}`; expected {}",
            spellings()
        ))
    })?;
    let table = Profile::of(device, personality).map_err(|e| Failure::refused(e.to_string()))?;
    Ok(vec![Ok(table)])
}
