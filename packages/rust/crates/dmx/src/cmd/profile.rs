//! `profile`: the channel table of one device, to patch a desk with.

use govee_toolkit::codec::{Catalog, Device};
use govee_toolkit_dmx::profile::{self, Personality, Profile};
use govee_toolkit_dmx::report;

use super::{Failure, REFUSED, USAGE, spellings};

pub(crate) fn run(sku: &str, personality: Option<&str>, as_json: bool) -> Result<(), Failure> {
    let catalog = Catalog::embedded().map_err(|error| Failure::new(error.to_string(), USAGE))?;
    let device = catalog
        .device(sku)
        .map_err(|error| Failure::new(error.to_string(), USAGE))?;
    let tables = tables(device, personality)?;
    if as_json {
        println!("{}", report::json(device, &tables));
    } else {
        println!("{}", report::text(device, &tables));
    }
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
        Failure::new(
            format!("unknown personality `{name}`; expected {}", spellings()),
            USAGE,
        )
    })?;
    let table =
        Profile::of(device, personality).map_err(|e| Failure::new(e.to_string(), REFUSED))?;
    Ok(vec![Ok(table)])
}
