//! `identify`: light each patched fixture in turn.
//!
//! The walk is what an operator reads the rig with: the terminal names the
//! entry, and one fixture in the room answers. Every fixture goes off first,
//! and the walk then lights the enabled entries one at a time, in the order
//! the patch lists them, over `lan` alone. A fixture that fails stops no
//! other one.
//!
//! [`Chosen`] narrows the walk to a part of the rig, by device or by the
//! channels a fixture answers to. The blackout covers the whole rig either
//! way: the patch declares the room, so one lit fixture in a dark room is
//! what the operator reads.
//!
//! [`Govee::identify_walk`] runs the walk itself. It drives a device the
//! configuration enables `lan` for, and [`resolve`] reports the patched
//! devices that enable no `lan` mode before the walk starts.

use std::path::Path;

use govee_toolkit::{DeviceId, Govee, Walk, WalkObserver};
use govee_toolkit_dmx::patch::{Fixture, Patch, PortAddress, Rig};
use serde_json::json;

use super::rig::{configure, resolve, scan};
use super::{CONFIG, Failure, INTERNAL, UNREACHABLE, patch as writer};

/// Which fixtures one walk lights. Every enabled fixture where it names
/// none.
#[derive(Debug, Clone, Default)]
pub(crate) struct Chosen {
    /// The devices the command line named, in the grammar
    /// [`govee_toolkit::Selector`] reads.
    pub(crate) targets: Vec<String>,
    /// The port-address to light.
    pub(crate) universe: Option<PortAddress>,
    /// The channel inside that port-address. The whole universe where it
    /// names none.
    pub(crate) address: Option<u16>,
}

impl Chosen {
    /// Whether the command line narrowed the walk at all.
    fn is_empty(&self) -> bool {
        self.targets.is_empty() && self.universe.is_none()
    }
}

pub(crate) fn start(
    named: Option<&Path>,
    chosen: &Chosen,
    walk: Walk,
    config: Option<&Path>,
    as_json: bool,
) -> Result<(), Failure> {
    let file = writer::file(named);
    let patch = Patch::load(&file).map_err(|e| Failure::new(e.to_string(), CONFIG))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
    runtime.block_on(async {
        let govee = Govee::start(configure(config)?)
            .await
            .map_err(|e| Failure::new(e.to_string(), CONFIG))?;
        scan(&govee).await?;
        let outcome = async {
            let rig = resolve(&govee, &patch)?;
            let fixtures = lit(&govee, &rig, chosen)?;
            run(&govee, &rig, &fixtures, walk, as_json).await
        }
        .await;
        govee
            .shutdown()
            .await
            .map_err(|e| Failure::new(e.to_string(), INTERNAL))?;
        outcome
    })
}

/// Light the chosen fixtures, then take the rig off.
async fn run(
    govee: &Govee,
    rig: &Rig,
    chosen: &[&Fixture],
    walk: Walk,
    as_json: bool,
) -> Result<(), Failure> {
    let blackout: Vec<DeviceId> = rig
        .fixtures()
        .iter()
        .map(|fixture| fixture.entry.device.clone())
        .collect();
    let spans = Spans { chosen, as_json };
    let lit: Vec<DeviceId> = chosen
        .iter()
        .map(|fixture| fixture.entry.device.clone())
        .collect();
    let report = govee
        .identify_walk(&blackout, &lit, &walk, &spans)
        .await
        .map_err(|e| Failure::new(e.to_string(), UNREACHABLE))?;
    match report.summary("fixture") {
        Some(summary) => Err(Failure::new(summary, UNREACHABLE)),
        None => Ok(()),
    }
}

/// What the walk prints: the entry the operator reads, beside the fixture
/// that answers it.
struct Spans<'a> {
    chosen: &'a [&'a Fixture],
    as_json: bool,
}

impl WalkObserver for Spans<'_> {
    fn lighting(&self, id: &DeviceId) {
        let Some(fixture) = self.chosen.iter().find(|f| &f.entry.device == id) else {
            return;
        };
        if self.as_json {
            let mut record = fixture.json();
            if let Some(object) = record.as_object_mut() {
                object.insert("event".to_owned(), json!("identify"));
            }
            println!("{record}");
            return;
        }
        println!("identify {id}  {}", fixture.span);
    }

    fn refused(&self, id: &DeviceId, reason: &str) {
        if self.as_json {
            println!(
                "{}",
                json!({ "event": "failed", "device": id.to_string(), "reason": reason })
            );
            return;
        }
        println!("failed {id}  {reason}");
    }
}

/// The fixtures the command line chose, in the order the patch lists them.
///
/// A target and an address name fixtures separately, and the walk lights
/// every fixture either one names. A target or an address that names no
/// driven fixture is a fault: an operator who typed one means to see it
/// light.
fn lit<'a>(govee: &Govee, rig: &'a Rig, chosen: &Chosen) -> Result<Vec<&'a Fixture>, Failure> {
    let fixtures = rig.fixtures();
    if fixtures.is_empty() {
        return Err(Failure::new(
            "the patch enables no fixture, so there is nothing to light",
            CONFIG,
        ));
    }
    if chosen.is_empty() {
        return Ok(fixtures.iter().collect());
    }
    let mut named: Vec<&Fixture> = Vec::new();
    if !chosen.targets.is_empty() {
        let ids = govee
            .select(&chosen.targets)
            .map_err(|e| Failure::new(e.to_string(), CONFIG))?;
        for id in &ids {
            let Some(fixture) = fixtures.iter().find(|f| &f.entry.device == id) else {
                return Err(Failure::new(
                    format!("the patch drives no fixture for `{id}`"),
                    CONFIG,
                ));
            };
            named.push(fixture);
        }
    }
    if let Some(universe) = chosen.universe {
        let at = rig.at(universe, chosen.address);
        if at.is_empty() {
            return Err(Failure::new(at_nothing(universe, chosen.address), CONFIG));
        }
        named.extend(at);
    }
    Ok(in_patch_order(fixtures, &named))
}

/// What to say where a port-address holds no driven fixture.
fn at_nothing(universe: PortAddress, address: Option<u16>) -> String {
    match address {
        Some(channel) => {
            format!("no driven fixture answers to channel {channel} of universe {universe}")
        }
        None => format!("no driven fixture sits on universe {universe}"),
    }
}

/// The chosen fixtures, each one once and in the order the patch lists them.
fn in_patch_order<'a>(fixtures: &'a [Fixture], named: &[&'a Fixture]) -> Vec<&'a Fixture> {
    fixtures
        .iter()
        .filter(|fixture| {
            named
                .iter()
                .any(|chosen| chosen.entry.device == fixture.entry.device)
        })
        .collect()
}
