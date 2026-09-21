//! `docs/dmx-profiles.md`: the DMX channel table of every device file.
//!
//! The table is derived by `govee_toolkit::profile`, so this page and the node
//! cannot disagree. Nothing here reads a SKU name.

use govee_toolkit::codec::{Catalog, Device};
use govee_toolkit::profile::{self, Personality, Profile, Scale, Slot, report};
use serde_json::{Value, json};

/// The channel tables of one device, for `dist/catalog.json`.
///
/// It holds what `govee-dmx profile --json` holds, so the devices page of the
/// site and the node read one table. A personality the device serves through
/// nothing is left out, and one wider than a universe carries its error.
pub(crate) fn catalog_entry(device: &Device) -> Value {
    let tables = profile::served(device);
    json!({ "personalities": report::personalities(&tables) })
}

/// How many channels `personality` takes on `device`, as a table cell.
///
/// `—` where the device serves it through nothing, and the count with a note
/// where the table is wider than one universe.
fn width(device: &Device, personality: Personality) -> String {
    match Profile::of(device, personality) {
        Ok(profile) => profile.width().to_string(),
        Err(profile::Error::TooWide { channels, .. }) => {
            format!("{channels}, over one universe")
        }
        Err(_) => "—".to_owned(),
    }
}

pub(crate) fn personality_table(catalog: &Catalog) -> String {
    let columns = Personality::ALL.map(|p| format!("`{p}`"));
    let mut out = format!("| SKU | Name | {} |\n", columns.join(" | "));
    out.push_str(&format!(
        "| --- | ---- | {} |\n",
        columns
            .iter()
            .map(|column| "-".repeat(column.len()))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for device in catalog.devices() {
        let cells: Vec<String> = Personality::ALL
            .into_iter()
            .map(|personality| width(device, personality))
            .collect();
        out.push_str(&format!(
            "| [{}](../devices/{}.yaml) | {} | {} |\n",
            device.sku,
            device.sku,
            device.name,
            cells.join(" | ")
        ));
    }
    out
}

/// The scale of the dimmer and of the white channel, per device.
///
/// A step count under 255 is what the operator needs: the desk sends 255
/// values and the device applies fewer, so a slow fade lands stepped.
pub(crate) fn scale_table(catalog: &Catalog) -> String {
    let mut out = String::from(
        "| SKU | Dimmer | Dimmer steps | White temperature | White steps |\n\
         | --- | ------ | ------------ | ----------------- | ----------- |\n",
    );
    for device in catalog.devices() {
        let dimmer = scale_of(device, Personality::Full, Slot::Dimmer);
        let white = scale_of(device, Personality::Full, Slot::WhiteTemp);
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            device.sku,
            range(dimmer),
            steps(dimmer),
            range(white),
            steps(white),
        ));
    }
    out
}

/// The scale the device puts on `slot`, read off the personality that carries
/// it. `None` where the device serves that personality through nothing.
fn scale_of(device: &Device, personality: Personality, slot: Slot) -> Option<Scale> {
    let profile = Profile::of(device, personality).ok()?;
    profile
        .channels()
        .iter()
        .find(|channel| channel.slot == slot)
        .and_then(|channel| channel.scale)
}

fn range(scale: Option<Scale>) -> String {
    scale.map_or_else(
        || "—".to_owned(),
        |scale| {
            let [min, max] = scale.range();
            format!("{min} to {max}")
        },
    )
}

fn steps(scale: Option<Scale>) -> String {
    scale.map_or_else(|| "—".to_owned(), |scale| scale.steps().to_string())
}
