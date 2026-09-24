//! The patch file, as text.
//!
//! The writer edits the lines a scan owns and rewrites nothing else: a round
//! trip through the parser would drop every comment.
//!
//! A scan owns three things: the entries it appends, the `enabled:` of each
//! entry, and the `scanned:` stamp. An entry written in the flow form —
//! `- { device: A, address: 1 }` — carries no `enabled:` line to edit, so the
//! writer leaves it as it stands.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use govee_toolkit::DeviceId;

use super::plan::Placement;

/// The file a first scan starts from.
const SKELETON: &str = "\
# The rig `govee-dmx` drives.
#
# `govee-dmx patch` appends to this file and moves no entry that is already in
# it. It writes `enabled:` from the scan: an entry whose device did not answer
# is `false`, and it keeps its channels, so every other address stays where the
# desk has it. Set `hold: true` on an entry to keep `enabled:` as you wrote it,
# whatever the scan finds. Delete the entry to hand its channels back.
node:
  bind: 0.0.0.0
  name: govee-toolkit
  refresh_secs: 10
  signal_loss_secs: 4
  off_delay_secs: 5
patch:
";

/// The key the entries hang under, at the start of a line.
const KEY: &str = "patch:";
const STAMP: &str = "scanned:";
const ENABLED: &str = "enabled:";
const DEVICE: &str = "- device:";

/// `text`, as the scan leaves it.
///
/// `added` are the entries to append, `states` the `enabled:` of every entry
/// the file carries, by identity, and `scanned` the stamp. An empty `text`
/// takes the skeleton first, so a first scan writes a whole file.
#[must_use]
pub fn update(
    text: &str,
    added: &[Placement],
    states: &BTreeMap<DeviceId, bool>,
    scanned: &str,
) -> String {
    let mut out = if text.trim().is_empty() {
        SKELETON.to_owned()
    } else {
        opened(text)
    };
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out = stamped(&out, scanned);
    out = enabled(&out, states);
    let tail = out.split_off(insertion(&out));
    for placement in added {
        out.push_str(&entry(placement));
    }
    out.push_str(&tail);
    out
}

/// `text`, with the `patch:` list ready to take an entry.
///
/// A file with no `patch:` key takes one at the end. A `patch: []` takes the
/// block form, because an entry cannot go in the flow form.
fn opened(text: &str) -> String {
    let mut out = text.to_owned();
    match text.lines().find(|line| line.starts_with(KEY)) {
        None => {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(KEY);
            out.push('\n');
        }
        Some(line) if line.trim_end() == "patch: []" => {
            out = out.replacen(line, KEY, 1);
        }
        Some(_) => {}
    }
    out
}

/// `text`, carrying `scanned` as its stamp.
///
/// The stamp goes above the first key, so a reader meets it before the rig.
fn stamped(text: &str, scanned: &str) -> String {
    let line = format!("{STAMP} {scanned}");
    let mut out = String::with_capacity(text.len() + line.len() + 1);
    let mut written = false;
    for source in text.split_inclusive('\n') {
        if source.starts_with(STAMP) {
            out.push_str(&line);
            out.push('\n');
            written = true;
            continue;
        }
        if !written && !source.starts_with(['#', ' ', '\t']) && !source.trim().is_empty() {
            out.push_str(&line);
            out.push('\n');
            written = true;
        }
        out.push_str(source);
    }
    if !written {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// `text`, with the `enabled:` of each entry `states` names.
///
/// The key goes under the `device:` of its entry. An entry the states do not
/// name keeps every line it has.
fn enabled(text: &str, states: &BTreeMap<DeviceId, bool>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut current: Option<bool> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with(DEVICE) {
            let indent = &line[..line.len() - trimmed.len()];
            out.push_str(line);
            current = identity(trimmed).and_then(|id| states.get(&id).copied());
            if let Some(state) = current {
                let _ = writeln!(out, "{indent}  {ENABLED} {state}");
            }
            continue;
        }
        if trimmed.starts_with(ENABLED) && current.is_some() {
            continue;
        }
        if !line.starts_with([' ', '\t', '#']) {
            current = None;
        }
        out.push_str(line);
    }
    out
}

fn identity(line: &str) -> Option<DeviceId> {
    let value = line.strip_prefix(DEVICE)?.trim();
    let value = value.trim_matches(['"', '\''].as_slice()).trim();
    if value.is_empty() {
        return None;
    }
    Some(DeviceId::new(value))
}

/// Where an entry goes: after the last line of the `patch:` list.
///
/// The list runs to the first line that starts at column 0 after the key, so
/// a `node:` block under the list keeps its place.
fn insertion(text: &str) -> usize {
    let mut offset = 0;
    let mut end = text.len();
    let mut inside = false;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        if !inside {
            if line.starts_with(KEY) {
                inside = true;
                end = offset;
            }
            continue;
        }
        let blank = line.trim().is_empty();
        if !blank && !line.starts_with([' ', '\t', '#']) {
            return start;
        }
        if !blank {
            end = offset;
        }
    }
    end
}

fn entry(placement: &Placement) -> String {
    let mut text = format!(
        "  # {}  {}  {} channels\n",
        placement.sku, placement.model, placement.width
    );
    let _ = writeln!(text, "  - device: \"{}\"", placement.device);
    if let Some(name) = &placement.name {
        // A JSON string is a YAML scalar in double quotes, whatever the name
        // holds.
        let _ = writeln!(text, "    name: {}", serde_json::Value::from(name.as_str()));
    }
    if !placement.groups.is_empty() {
        // A JSON array is a YAML flow sequence.
        let _ = writeln!(
            text,
            "    groups: {}",
            serde_json::Value::from(placement.groups.clone())
        );
    }
    let _ = write!(
        text,
        "    enabled: true\n    sku: {}\n    universe: {}\n    address: {}\n    personality: {}\n",
        placement.sku, placement.universe, placement.address, placement.personality
    );
    text
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::patch::{Patch, PortAddress};
    use crate::profile::Personality;

    const NOW: &str = "2026-09-19T20:00:00Z";

    fn placement(id: &str, address: u16) -> Placement {
        Placement {
            device: DeviceId::new(id),
            sku: "H6199".to_owned(),
            model: "Strip".to_owned(),
            name: None,
            groups: Vec::new(),
            personality: Personality::Full,
            universe: PortAddress::new(0).unwrap_or_else(|| unreachable!()),
            address,
            width: 6,
        }
    }

    fn states(pairs: &[(&str, bool)]) -> BTreeMap<DeviceId, bool> {
        pairs
            .iter()
            .map(|(id, state)| (DeviceId::new(*id), *state))
            .collect()
    }

    fn write(text: &str, added: &[Placement], pairs: &[(&str, bool)]) -> String {
        update(text, added, &states(pairs), NOW)
    }

    /// How many lines carry `key`. A comment that names the key does not.
    fn keys(text: &str, key: &str) -> usize {
        text.lines()
            .filter(|line| line.trim_start().starts_with(key))
            .count()
    }

    #[test]
    fn a_first_scan_writes_a_whole_file() {
        let text = write("", &[placement("AA:BB:CC:DD:EE:01", 1)], &[]);
        let patch = Patch::parse(&text, "test").expect("the skeleton parses");
        assert_eq!(patch.patch.len(), 1);
        assert_eq!(patch.patch[0].address, 1);
        assert!(patch.patch[0].enabled);
        assert_eq!(patch.scanned.as_deref(), Some(NOW));
        assert_eq!(patch.node.signal_loss_secs, 4);
    }

    #[test]
    fn an_entry_goes_after_the_last_one_and_keeps_the_comments() {
        let first = write("", &[placement("AA:BB:CC:DD:EE:01", 1)], &[]);
        let text = write(
            &first,
            &[placement("AA:BB:CC:DD:EE:02", 7)],
            &[("AA:BB:CC:DD:EE:01", true)],
        );
        assert!(text.starts_with("# The rig"));
        let patch = Patch::parse(&text, "test").expect("the file parses");
        assert_eq!(patch.patch.len(), 2);
        assert_eq!(patch.patch[1].address, 7);
    }

    /// The scan owns `enabled:`: a device that did not answer takes `false`,
    /// and it takes `true` again once it answers.
    #[test]
    fn the_scan_writes_the_state_of_each_entry_both_ways() {
        let first = write("", &[placement("AA:BB:CC:DD:EE:01", 1)], &[]);
        let gone = write(&first, &[], &[("AA:BB:CC:DD:EE:01", false)]);
        let patch = Patch::parse(&gone, "test").expect("the file parses");
        assert!(!patch.patch[0].enabled);
        assert_eq!(keys(&gone, ENABLED), 1, "{gone}");

        let back = write(&gone, &[], &[("AA:BB:CC:DD:EE:01", true)]);
        let patch = Patch::parse(&back, "test").expect("the file parses");
        assert!(patch.patch[0].enabled);
        assert_eq!(keys(&back, ENABLED), 1, "{back}");
    }

    /// The stamp is written once, whatever the number of scans.
    #[test]
    fn the_stamp_is_replaced_and_never_repeated() {
        let first = write("", &[placement("AA:BB:CC:DD:EE:01", 1)], &[]);
        let later = update(&first, &[], &states(&[]), "2026-09-20T09:00:00Z");
        assert_eq!(keys(&later, STAMP), 1, "{later}");
        let patch = Patch::parse(&later, "test").expect("the file parses");
        assert_eq!(patch.scanned.as_deref(), Some("2026-09-20T09:00:00Z"));
    }

    /// An entry the states do not name keeps every line it has.
    #[test]
    fn an_entry_outside_the_scan_is_untouched() {
        let text = "patch:\n  - device: \"AA:BB:CC:DD:EE:01\"\n    enabled: false\n    address: 1\n    personality: full\n";
        let out = write(text, &[], &[]);
        let patch = Patch::parse(&out, "test").expect("the file parses");
        assert!(!patch.patch[0].enabled);
    }

    #[test]
    fn a_node_block_under_the_list_keeps_its_place() {
        let text = "patch:\n  - device: \"AA:BB:CC:DD:EE:01\"\n    address: 1\n    personality: full\nnode:\n  name: desk\n";
        let out = write(text, &[placement("AA:BB:CC:DD:EE:02", 7)], &[]);
        let patch = Patch::parse(&out, "test").expect("the file parses");
        assert_eq!(patch.node.name, "desk");
        assert_eq!(patch.patch.len(), 2);
        assert_eq!(patch.patch[1].device.as_str(), "AA:BB:CC:DD:EE:02");
    }

    /// A new entry takes the name the configuration gives, in quotes, so a
    /// name that YAML would read as another type stays a name.
    #[test]
    fn a_new_entry_takes_the_name_the_configuration_gives() {
        let mut named = placement("AA:BB:CC:DD:EE:01", 1);
        named.name = Some("kitchen: left".to_owned());
        let text = write("", &[named, placement("AA:BB:CC:DD:EE:02", 7)], &[]);
        let patch = Patch::parse(&text, "test").expect("the file parses");
        assert_eq!(patch.patch[0].name.as_deref(), Some("kitchen: left"));
        assert_eq!(patch.patch[1].name, None);
    }

    #[test]
    fn a_new_entry_takes_the_groups_the_configuration_gives() {
        let mut grouped = placement("AA:BB:CC:DD:EE:01", 1);
        grouped.groups = vec!["bar".to_owned(), "yes: no".to_owned()];
        let text = write("", &[grouped, placement("AA:BB:CC:DD:EE:02", 7)], &[]);
        let patch = Patch::parse(&text, "test").expect("the file parses");
        assert_eq!(patch.patch[0].groups, ["bar", "yes: no"]);
        assert!(patch.patch[1].groups.is_empty());
        assert!(!text.contains("groups: []"), "{text}");
    }

    #[test]
    fn an_empty_list_takes_the_block_form() {
        let out = write(
            "node:\n  name: desk\npatch: []\n",
            &[placement("AA:BB:CC:DD:EE:01", 1)],
            &[],
        );
        let patch = Patch::parse(&out, "test").expect("the file parses");
        assert_eq!(patch.patch.len(), 1);
    }

    #[test]
    fn a_file_with_no_list_takes_one() {
        let out = write(
            "node:\n  name: desk\n",
            &[placement("AA:BB:CC:DD:EE:01", 1)],
            &[],
        );
        let patch = Patch::parse(&out, "test").expect("the file parses");
        assert_eq!(patch.node.name, "desk");
        assert_eq!(patch.patch.len(), 1);
    }
}
