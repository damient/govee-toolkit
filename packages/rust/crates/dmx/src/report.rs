//! The channel table, for a person and for a machine.
//!
//! The JSON form is the contract of `govee-dmx profile --json`. The text form
//! is for an operator at a desk, and its layout can change at any release.

use govee_toolkit::codec::Device;
use serde_json::{Value, json};

use crate::profile::{Channel, Error, Personality, Profile, Slot};

/// The tables of one device, as one JSON object.
///
/// A personality the device serves and cannot fit in one universe carries an
/// `error` instead of its channels.
#[must_use]
pub fn json(device: &Device, tables: &[Result<Profile, Error>]) -> Value {
    json!({
        "sku": device.sku,
        "name": device.name,
        "personalities": personalities(tables),
    })
}

/// The same tables, as the JSON array alone.
///
/// `dist/catalog.json` carries this beside each device, so the catalog, the
/// site and the node read one channel table.
#[must_use]
pub fn personalities(tables: &[Result<Profile, Error>]) -> Value {
    Value::Array(tables.iter().map(personality_json).collect())
}

/// The same tables, as the lines an operator reads.
#[must_use]
pub fn text(device: &Device, tables: &[Result<Profile, Error>]) -> String {
    let mut lines = vec![
        format!("{}  {}  family: {}", device.sku, device.name, device.family),
        format!("serves: {}", names(tables)),
    ];
    for table in tables {
        lines.push(String::new());
        match table {
            Ok(profile) => {
                lines.push(format!(
                    "{}  {} channels",
                    profile.personality(),
                    profile.width()
                ));
                lines.extend(profile.channels().iter().map(channel_text));
            }
            Err(error) => lines.push(format!("{}  {error}", error.personality())),
        }
    }
    lines.join("\n")
}

fn names(tables: &[Result<Profile, Error>]) -> String {
    if tables.is_empty() {
        return "-".to_owned();
    }
    tables
        .iter()
        .map(|table| personality_of(table).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn personality_of(table: &Result<Profile, Error>) -> Personality {
    table
        .as_ref()
        .map_or_else(Error::personality, Profile::personality)
}

fn personality_json(table: &Result<Profile, Error>) -> Value {
    let name = personality_of(table).as_str();
    match table {
        Ok(profile) => json!({
            "personality": name,
            "width": profile.width(),
            "channels": profile.channels().iter().map(channel_json).collect::<Vec<_>>(),
        }),
        Err(error) => json!({ "personality": name, "error": error.to_string() }),
    }
}

fn channel_json(channel: &Channel) -> Value {
    let mut record = json!({ "offset": channel.offset, "slot": kind(channel.slot) });
    if let (Some(object), Slot::Color(component) | Slot::Zone { component, .. }) =
        (record.as_object_mut(), channel.slot)
    {
        object.insert("component".to_owned(), json!(component.to_string()));
    }
    if let (Some(object), Slot::Zone { index, .. }) = (record.as_object_mut(), channel.slot) {
        object.insert("zone".to_owned(), json!(index));
    }
    if let (Some(object), Some(scale)) = (record.as_object_mut(), channel.scale) {
        let [min, max] = scale.range();
        object.insert("range".to_owned(), json!([min, max]));
        object.insert("steps".to_owned(), json!(scale.steps()));
    }
    if let (Some(object), true) = (record.as_object_mut(), unreached(channel)) {
        object.insert("unreached".to_owned(), json!(true));
    }
    record
}

/// Whether the channel holds its place in the table and drives nothing. The
/// white channel does that where `lan` reaches no white temperature.
fn unreached(channel: &Channel) -> bool {
    matches!(channel.slot, Slot::WhiteTemp) && channel.scale.is_none()
}

/// What the slot drives, as one word a machine matches on. The zone index and
/// the component travel beside it, so a reader never parses this name.
fn kind(slot: Slot) -> &'static str {
    match slot {
        Slot::Dimmer => "dimmer",
        Slot::Color(_) => "color",
        Slot::WhiteTemp => "white_temp",
        Slot::Mode => "mode",
        Slot::Zone { .. } => "zone",
    }
}

fn channel_text(channel: &Channel) -> String {
    let detail = channel.scale.map_or_else(
        || {
            if unreached(channel) {
                "unreached, drives nothing".to_owned()
            } else {
                String::new()
            }
        },
        |scale| {
            let [min, max] = scale.range();
            format!("{min} to {max}, {} steps", scale.steps())
        },
    );
    format!(
        "{:>4}  {:<22}{detail}",
        channel.offset,
        channel.slot.to_string()
    )
    .trim_end()
    .to_owned()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]

    use govee_toolkit::codec::Catalog;

    use super::{json, text};
    use crate::profile::{self, Personality, Profile};

    fn catalog() -> Catalog {
        Catalog::embedded().expect("the embedded catalog parses")
    }

    #[test]
    fn a_scaled_channel_carries_its_range_and_its_step_count() {
        let catalog = catalog();
        let device = catalog.device("H6008").expect("the SKU resolves");
        let table = Profile::of(device, Personality::Full).expect("a full personality");
        let record = json(device, &[Ok(table)]);
        let dimmer = &record["personalities"][0]["channels"][0];
        assert_eq!(dimmer["slot"], "dimmer");
        assert_eq!(dimmer["offset"], 1);
        assert_eq!(dimmer["steps"], 100);
        assert_eq!(dimmer["range"], serde_json::json!([1, 100]));
    }

    /// A zone channel says which zone and which component, so a reader never
    /// parses the text form.
    #[test]
    fn a_zone_channel_carries_its_index_and_its_component() {
        let catalog = catalog();
        let device = catalog.device("H61A0").expect("the SKU resolves");
        let table = Profile::of(device, Personality::Segment).expect("a segment personality");
        let record = json(device, &[Ok(table)]);
        let green = &record["personalities"][0]["channels"][3];
        assert_eq!(green["slot"], "zone");
        assert_eq!(green["zone"], 0);
        assert_eq!(green["component"], "green");
        assert_eq!(green["offset"], 4);
    }

    /// A personality the device serves and cannot fit answers its error, so
    /// the width is what the operator sees refused it.
    #[test]
    fn a_table_over_one_universe_reports_its_width() {
        let catalog = catalog();
        let device = catalog.device("H6008").expect("the SKU resolves");
        let error = profile::Error::TooWide {
            sku: device.sku.clone(),
            personality: Personality::Pixel,
            channels: 601,
        };
        let record = json(device, &[Err(error.clone())]);
        assert_eq!(record["personalities"][0]["personality"], "pixel");
        assert_eq!(record["personalities"][0]["error"], error.to_string());
        assert!(text(device, &[Err(error)]).contains("601"));
    }

    #[test]
    fn the_text_form_names_every_channel_and_its_offset() {
        let catalog = catalog();
        let device = catalog.device("H6008").expect("the SKU resolves");
        let table = Profile::of(device, Personality::Full).expect("a full personality");
        let width = usize::from(table.width());
        let printed = text(device, &[Ok(table)]);
        for line in [
            "dimmer",
            "red",
            "green",
            "blue",
            "white temperature",
            "mode",
        ] {
            assert!(printed.contains(line), "`{line}` is missing");
        }
        // Two header lines, one blank line, one personality line, then the
        // channels.
        assert_eq!(printed.lines().count(), 4 + width);
    }
}
