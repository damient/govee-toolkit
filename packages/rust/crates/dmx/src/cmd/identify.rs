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
//! way: one lit fixture in a dark room is what the operator reads.

use std::path::Path;
use std::time::Duration;

use govee_toolkit::{DeviceId, Govee, Identify, Mode};
use govee_toolkit_dmx::patch::{Fixture, Patch, PortAddress, Rig};
use serde_json::json;

use super::rig::{configure, resolve, scan};
use super::{CONFIG, Failure, INTERNAL, UNREACHABLE, patch as writer};

/// What one walk does.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Walk {
    /// What each fixture shows.
    pub(crate) pass: Identify,
    /// How long the walk waits between two steps: after the rig goes off,
    /// and after each fixture lights.
    pub(crate) wait: Duration,
    /// How long the last fixture holds the color before every fixture goes
    /// off.
    pub(crate) hold: Duration,
    /// Leave every fixture lit, and send no power command at the end.
    pub(crate) keep: bool,
}

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
        let outcome = match resolve(&govee, &patch) {
            Ok(rig) => match lit(&govee, &rig, chosen) {
                Ok(fixtures) => run(&govee, &rig, &fixtures, walk, as_json).await,
                Err(failure) => Err(failure),
            },
            Err(failure) => Err(failure),
        };
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
    // The whole rig goes dark first. One lit fixture in a dark room is the
    // answer the operator reads. A fixture that refuses the blackout leaves
    // the walk here, and is reported once.
    let mut failed = off(govee, rig.fixtures(), as_json).await;
    let lit: Vec<&Fixture> = chosen
        .iter()
        .copied()
        .filter(|fixture| !failed.contains(&fixture.entry.device))
        .collect();
    tokio::time::sleep(walk.wait).await;
    for fixture in &lit {
        announce(fixture, as_json);
        let id = &fixture.entry.device;
        if let Err(error) = govee.device_on(id, Mode::Lan).identify(&walk.pass).await {
            report(id, &error.to_string(), as_json);
            failed.push(id.clone());
        }
        tokio::time::sleep(walk.wait).await;
    }
    if !walk.keep {
        tokio::time::sleep(walk.hold).await;
        drop(off(govee, rig.fixtures(), as_json).await);
    }
    if failed.is_empty() {
        return Ok(());
    }
    let unlit: Vec<String> = failed.iter().map(ToString::to_string).collect();
    Err(Failure::new(
        format!("these fixtures did not take the pass: {}", unlit.join(", ")),
        UNREACHABLE,
    ))
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

/// Take every fixture off at once, and answer the ones that refused.
///
/// One task per fixture: a rig goes dark together, and a fixture that answers
/// slowly holds up no other one.
async fn off(govee: &Govee, fixtures: &[Fixture], as_json: bool) -> Vec<DeviceId> {
    let mut passes = Vec::with_capacity(fixtures.len());
    for fixture in fixtures {
        let (govee, id) = (govee.clone(), fixture.entry.device.clone());
        passes.push(tokio::spawn(async move {
            let outcome = govee.device_on(&id, Mode::Lan).power(false).await;
            (id, outcome)
        }));
    }
    let mut refused = Vec::new();
    for pass in passes {
        let Ok((id, outcome)) = pass.await else {
            continue;
        };
        if let Err(error) = outcome {
            report(&id, &error.to_string(), as_json);
            refused.push(id);
        }
    }
    refused
}

/// Name the fixture before it lights, so the operator reads the line and the
/// room at the same time.
fn announce(fixture: &Fixture, as_json: bool) {
    let id = &fixture.entry.device;
    if as_json {
        println!(
            "{}",
            json!({
                "event": "identify",
                "device": id.to_string(),
                "universe": fixture.universe.get(),
                "first": fixture.span.first,
                "last": fixture.span.last,
                "personality": fixture.profile.personality().as_str(),
            })
        );
        return;
    }
    println!("identify {id}  {}", fixture.span);
}

fn report(id: &DeviceId, reason: &str, as_json: bool) {
    if as_json {
        println!(
            "{}",
            json!({ "event": "failed", "device": id.to_string(), "reason": reason })
        );
        return;
    }
    println!("failed {id}  {reason}");
}
