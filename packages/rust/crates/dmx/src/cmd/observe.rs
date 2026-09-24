//! What a run prints.
//!
//! A dry run prints every packet and every look. A live run prints the rig it
//! took, the failures, and the counters at the end: a line per frame at 44
//! frames per second would bury them.

use std::collections::BTreeMap;
use std::net::SocketAddr;

use govee_toolkit::DeviceId;
use govee_toolkit::exit::Writer;
use govee_toolkit_dmx::apply::{Counts, Look};
use govee_toolkit_dmx::input::UniverseFrame;
use govee_toolkit_dmx::node::Observer;
use govee_toolkit_dmx::patch::{Fixture, Label, Patch, Rig};
use serde_json::{Value, json};

/// The lines, or the records, one run writes.
#[derive(Debug)]
pub(crate) struct Printer {
    writer: Writer,
    /// A dry run writes to no device, and prints every packet instead.
    dry_run: bool,
    /// Whether the run prints the answered polls. A desk polls every few
    /// seconds, and the lines bury the counters of a long run.
    debug: bool,
    /// Every patched fixture, as a message names it. [`Printer::started`]
    /// fills it.
    labels: BTreeMap<DeviceId, Label>,
}

impl Printer {
    pub(crate) const fn new(writer: Writer, dry_run: bool, debug: bool) -> Self {
        Self {
            writer,
            dry_run,
            debug,
            labels: BTreeMap::new(),
        }
    }

    /// `id`, with the name the patch gives it.
    fn label(&self, id: &DeviceId) -> String {
        self.labels
            .get(id)
            .map_or_else(|| id.to_string(), ToString::to_string)
    }

    /// `record`, with the name the patch gives `id`.
    fn named(&self, mut record: Value, id: &DeviceId) -> Value {
        let name = self.labels.get(id).and_then(|label| label.name.as_deref());
        if let (Some(name), Some(fields)) = (name, record.as_object_mut()) {
            fields.insert("name".to_owned(), json!(name));
        }
        record
    }

    /// The socket, the node name and the rig, before the first frame.
    pub(crate) fn started(&mut self, bound: Option<SocketAddr>, patch: &Patch, rig: &Rig) {
        self.labels = rig
            .fixtures()
            .iter()
            .chain(rig.reserved())
            .map(|fixture| (fixture.entry.device.clone(), fixture.entry.label()))
            .collect();
        let address = bound.map_or_else(|| patch.node.bind.to_string(), |a| a.to_string());
        let fixtures: Vec<Value> = rig.fixtures().iter().map(Fixture::json).collect();
        self.writer.emit(
            &json!({
                "event": "listening",
                "address": address,
                "node": patch.node.name,
                "dry_run": self.dry_run,
                "fixtures": fixtures,
                "reserved": rig
                    .reserved()
                    .iter()
                    .map(|fixture| fixture.entry.device.to_string())
                    .collect::<Vec<_>>(),
            }),
            &self.start_text(&address, patch, rig),
        );
    }

    fn start_text(&self, address: &str, patch: &Patch, rig: &Rig) -> String {
        let mut lines = vec![format!(
            "listening on {address}  node {}{}",
            patch.node.name,
            if self.dry_run { "  dry run" } else { "" }
        )];
        lines.extend(rig.fixtures().iter().map(|fixture| {
            format!(
                "{}  {}  {}",
                fixture.entry.label(),
                fixture.span,
                fixture.profile.personality()
            )
        }));
        lines.extend(rig.reserved().iter().map(|fixture| {
            format!(
                "{}  {}  reserved, not driven",
                fixture.entry.label(),
                fixture.span
            )
        }));
        lines.join("\n")
    }

    /// What each device did, once the node has stopped.
    pub(crate) fn ended(&self, counts: &[Counts]) {
        for count in counts {
            let record = json!({
                "event": "counts",
                "device": count.id.to_string(),
                "frames_sent": count.frames_sent,
                "frames_superseded": count.frames_superseded,
            });
            self.writer.emit(
                &self.named(record, &count.id),
                &format!(
                    "{}  {} frames sent  {} superseded",
                    self.label(&count.id),
                    count.frames_sent,
                    count.frames_superseded
                ),
            );
        }
    }
}

impl Observer for Printer {
    fn received(&mut self, frame: &UniverseFrame) {
        if !self.dry_run {
            return;
        }
        self.writer.emit(
            &json!({
                "event": "received",
                "universe": frame.universe,
                "source": frame.source.to_string(),
                "slots": frame.len(),
            }),
            &format!(
                "universe {}  from {}  {} slots",
                frame.universe,
                frame.source,
                frame.len()
            ),
        );
    }

    fn resolved(&mut self, id: &DeviceId, look: &Look) {
        if !self.dry_run {
            return;
        }
        let mut record = look_json(look);
        if let Some(fields) = record.as_object_mut() {
            fields.insert("event".to_owned(), json!("resolved"));
            fields.insert("device".to_owned(), json!(id.to_string()));
        }
        self.writer.emit(
            &self.named(record, id),
            &format!("  {}  {}", self.label(id), look_text(look)),
        );
    }

    fn refused(&mut self, source: SocketAddr, reason: &str) {
        if !self.dry_run {
            return;
        }
        self.writer.emit(
            &json!({ "event": "refused", "source": source.to_string(), "reason": reason }),
            &format!("refused  {source}  {reason}"),
        );
    }

    fn ignored(&mut self, source: SocketAddr, opcode: u16) {
        if !self.dry_run {
            return;
        }
        self.writer.emit(
            &json!({ "event": "ignored", "source": source.to_string(), "opcode": opcode }),
            &format!("ignored  {source}  opcode 0x{opcode:04x}"),
        );
    }

    /// Reported under `--debug` and in a dry run alone. An operator who
    /// cannot find the node in a desk's node list turns the flag on to see
    /// whether the node answered.
    fn polled(&mut self, source: SocketAddr, replies: usize) {
        if !self.dry_run && !self.debug {
            return;
        }
        self.writer.emit(
            &json!({ "event": "polled", "source": source.to_string(), "replies": replies }),
            &format!("polled  {source}  {replies} replies"),
        );
    }

    /// Always reported, and on stderr: a run that drives 39 devices and drops
    /// one must say so.
    fn failed(&mut self, id: &DeviceId, reason: &str) {
        let record = json!({ "event": "failed", "device": id.to_string(), "reason": reason });
        self.writer.warn(
            &self.named(record, id),
            &format!("failed  {}  {reason}", self.label(id)),
        );
    }
}

fn look_json(look: &Look) -> Value {
    json!({
        "on": look.on,
        "brightness": look.brightness,
        "color": look.color,
        "white_temp": look.white_temp,
        "zones": look.zones,
        "resend": look.resend,
    })
}

fn look_text(look: &Look) -> String {
    if !look.on {
        return "off".to_owned();
    }
    let mut parts = vec!["on".to_owned()];
    if let Some(level) = look.brightness {
        parts.push(format!("brightness {level}"));
    }
    if let Some([red, green, blue]) = look.color {
        parts.push(format!("color {red},{green},{blue}"));
    }
    if let Some(kelvin) = look.white_temp {
        parts.push(format!("white {kelvin}K"));
    }
    if !look.zones.is_empty() {
        parts.push(format!("zones {}", look.zones.len()));
    }
    if look.resend {
        parts.push("resend".to_owned());
    }
    parts.join("  ")
}
