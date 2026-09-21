//! `identify`: light each patched fixture in turn — see `docs/dmx.md` 4.3.
//!
//! [`Chosen`] narrows the walk to a part of the rig. The blackout covers the
//! whole rig either way.
//!
//! [`Govee::identify_walk`] runs the walk itself, and [`resolve`] reports the
//! patched devices that enable no `lan` mode before it starts.

use std::path::Path;

use govee_toolkit::codec::Mode;
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{DeviceId, Govee, Walk, WalkObserver};
use govee_toolkit_dmx::patch::{Fixture, Patch, PortAddress, Rig};
use serde_json::json;

use super::patch as patcher;
use super::rig::{configure, resolve, scan};

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
    writer: Writer,
) -> Result<(), Failure> {
    let file = patcher::file(named);
    let patch = Patch::load(&file).map_err(|e| Failure::config(e.to_string()))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Failure::internal(e.to_string()))?;
    runtime.block_on(async {
        let govee = Govee::start(configure(config)?)
            .await
            .map_err(|e| Failure::config(e.to_string()))?;
        scan(&govee).await?;
        let outcome = async {
            let rig = resolve(&govee, &patch)?;
            let fixtures = lit(&govee, &rig, chosen)?;
            run(&govee, &rig, &fixtures, walk, writer).await
        }
        .await;
        govee
            .shutdown()
            .await
            .map_err(|e| Failure::internal(e.to_string()))?;
        outcome
    })
}

/// Light the chosen fixtures, then take the rig off.
async fn run(
    govee: &Govee,
    rig: &Rig,
    chosen: &[&Fixture],
    walk: Walk,
    writer: Writer,
) -> Result<(), Failure> {
    let blackout: Vec<DeviceId> = rig
        .fixtures()
        .iter()
        .map(|fixture| fixture.entry.device.clone())
        .collect();
    let spans = Spans { chosen, writer };
    let lit: Vec<DeviceId> = chosen
        .iter()
        .map(|fixture| fixture.entry.device.clone())
        .collect();
    let report = govee
        .identify_walk(&blackout, &lit, &walk, &spans)
        .await
        .map_err(|e| Failure::unreachable(e.to_string()))?;
    match report.summary("fixture") {
        Some(summary) => Err(Failure::unreachable(summary)),
        None => Ok(()),
    }
}

/// What the walk prints: the entry the operator reads, beside the fixture
/// that answers it.
struct Spans<'a> {
    chosen: &'a [&'a Fixture],
    writer: Writer,
}

impl WalkObserver for Spans<'_> {
    fn lighting(&self, id: &DeviceId) {
        let Some(fixture) = self.chosen.iter().find(|f| &f.entry.device == id) else {
            return;
        };
        let mut record = fixture.json();
        if let Some(object) = record.as_object_mut() {
            object.insert("event".to_owned(), json!("identify"));
        }
        self.writer
            .emit(&record, &format!("identify {id}  {}", fixture.span));
    }

    fn refused(&self, id: &DeviceId, reason: &str) {
        self.writer.emit(
            &json!({ "event": "failed", "device": id.to_string(), "reason": reason }),
            &format!("failed {id}  {reason}"),
        );
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
        return Err(Failure::config(
            "the patch enables no fixture, so there is nothing to light",
        ));
    }
    if chosen.is_empty() {
        return Ok(fixtures.iter().collect());
    }
    let mut named: Vec<&Fixture> = Vec::new();
    if !chosen.targets.is_empty() {
        let ids = govee
            .select(&chosen.targets, Some(Mode::Lan))
            .map_err(|e| Failure::config(e.to_string()))?;
        for id in &ids {
            let Some(fixture) = fixtures.iter().find(|f| &f.entry.device == id) else {
                return Err(Failure::config(format!(
                    "the patch drives no fixture for `{id}`"
                )));
            };
            named.push(fixture);
        }
    }
    if let Some(universe) = chosen.universe {
        let at = rig.at(universe, chosen.address);
        if at.is_empty() {
            return Err(Failure::config(at_nothing(universe, chosen.address)));
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
