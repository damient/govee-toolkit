//! Where a device the scan found goes in the patch.
//!
//! The plan adds entries and moves none: an address an operator already set
//! on a desk is the one thing the bridge must not change — see
//! `docs/dmx.md` 4.1.

use std::collections::{BTreeMap, BTreeSet};

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Device;

use super::Patch;
use super::address::{MAX as MAX_PORT_ADDRESS, PortAddress};
use super::error::Error;
use crate::profile::{self, Personality, Profile, UNIVERSE};

/// Which personality a new entry takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// This personality alone. A device that serves it through nothing is
    /// skipped, and the plan says so.
    Fixed(Personality),
    /// The widest personality the device serves. One strip can then take a
    /// whole universe.
    Widest,
}

/// A device the scan found, with the file that states what it answers to.
#[derive(Debug, Clone, Copy)]
pub struct Candidate<'a> {
    /// The identity the scan reported.
    pub id: &'a DeviceId,
    /// The device file it is encoded under.
    pub device: &'a Device,
    /// The name the configuration gives the device.
    pub name: Option<&'a str>,
    /// The groups the configuration puts the device in.
    pub groups: &'a [String],
}

/// One entry the plan adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The identity the entry names.
    pub device: DeviceId,
    /// The SKU, which sizes the entry where the device does not answer.
    pub sku: String,
    /// The model name, for the operator at the desk.
    pub model: String,
    /// The name of the fixture, where the configuration gives the device one.
    pub name: Option<String>,
    /// The groups of the fixture.
    pub groups: Vec<String>,
    /// The layout the entry takes.
    pub personality: Personality,
    /// The port-address it lands on.
    pub universe: PortAddress,
    /// The start address, 1 to 512.
    pub address: u16,
    /// How many channels it takes from `address`.
    pub width: u16,
}

/// A device the plan adds no entry for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    /// The identity the scan reported.
    pub device: DeviceId,
    /// The SKU it reported.
    pub sku: String,
    /// What the plan could not do.
    pub reason: String,
}

/// What one scan adds to one patch.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// The entries to append, in the order they were placed.
    pub added: Vec<Placement>,
    /// The devices the plan placed nothing for.
    pub skipped: Vec<Skip>,
    /// How many entries the file already carries.
    pub kept: usize,
}

impl Plan {
    /// Whether the plan changes the file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
    }
}

impl Patch {
    /// The entries `candidates` adds to this patch.
    ///
    /// Each new fixture takes the lowest free channels of the lowest universe
    /// from `first`, so a second scan fills the gaps a disabled fixture left.
    /// `known` answers the device file of a SKU, and sizes an entry whose
    /// device did not answer.
    ///
    /// # Errors
    ///
    /// [`Error::Unsized`] where an entry states no width: its channels would
    /// otherwise go to a new fixture. [`Error::Unserved`] where an entry names
    /// a personality its device serves through nothing.
    pub fn plan<'a>(
        &self,
        candidates: &[Candidate<'a>],
        layout: Layout,
        first: PortAddress,
        known: impl Fn(&str) -> Option<&'a Device>,
    ) -> Result<Plan, Vec<Error>> {
        let found: BTreeMap<&DeviceId, &Device> = candidates
            .iter()
            .map(|candidate| (candidate.id, candidate.device))
            .collect();
        let mut taken = self.taken(&found, &known)?;
        let patched: BTreeSet<&DeviceId> = self.patch.iter().map(|entry| &entry.device).collect();
        let mut plan = Plan {
            kept: self.patch.len(),
            ..Plan::default()
        };
        for candidate in candidates {
            if patched.contains(candidate.id) {
                continue;
            }
            match place(candidate, layout, first, &mut taken) {
                Ok(placement) => plan.added.push(placement),
                Err(reason) => plan.skipped.push(Skip {
                    device: candidate.id.clone(),
                    sku: candidate.device.sku.clone(),
                    reason,
                }),
            }
        }
        Ok(plan)
    }

    fn taken<'a>(
        &self,
        found: &BTreeMap<&DeviceId, &Device>,
        known: &impl Fn(&str) -> Option<&'a Device>,
    ) -> Result<Taken, Vec<Error>> {
        let mut taken = Taken::default();
        let mut errors = Vec::new();
        for entry in &self.patch {
            let device = found
                .get(&entry.device)
                .copied()
                .or_else(|| entry.sku.as_deref().and_then(known));
            let Some(device) = device else {
                errors.push(Error::Unsized {
                    device: entry.label(),
                });
                continue;
            };
            match (entry.port_address(), Profile::of(device, entry.personality)) {
                (Ok(universe), Ok(profile)) => taken.hold(universe, entry.address, profile.width()),
                (Err(error), _) => errors.push(error),
                (_, Err(source)) => errors.push(Error::Unserved {
                    device: entry.label(),
                    source,
                }),
            }
        }
        if errors.is_empty() {
            Ok(taken)
        } else {
            Err(errors)
        }
    }
}

/// The channels each universe already holds.
#[derive(Debug, Default)]
struct Taken {
    per_universe: BTreeMap<PortAddress, Vec<(u16, u16)>>,
}

impl Taken {
    /// Hold `width` channels from `address`. A range past the end of the
    /// universe is held as it stands: the loader is what refuses it, and this
    /// pass must not hand those channels to a second fixture.
    fn hold(&mut self, universe: PortAddress, address: u16, width: u16) {
        let last = address.saturating_add(width.saturating_sub(1));
        self.per_universe
            .entry(universe)
            .or_default()
            .push((address, last));
    }

    /// The lowest start address that leaves `width` free channels in
    /// `universe`, or `None` where the universe holds no such run.
    fn free(&self, universe: PortAddress, width: u16) -> Option<u16> {
        let mut ranges = self
            .per_universe
            .get(&universe)
            .cloned()
            .unwrap_or_default();
        ranges.sort_unstable();
        let mut cursor = 1_u16;
        for (first, last) in ranges {
            if first > cursor && first - cursor >= width {
                return Some(cursor);
            }
            cursor = cursor.max(last.saturating_add(1));
        }
        (cursor.saturating_add(width.saturating_sub(1)) <= UNIVERSE).then_some(cursor)
    }
}

fn place(
    candidate: &Candidate<'_>,
    layout: Layout,
    first: PortAddress,
    taken: &mut Taken,
) -> Result<Placement, String> {
    let (personality, profile) = table(candidate.device, layout)?;
    let width = profile.width();
    for universe in (first.get()..=MAX_PORT_ADDRESS).filter_map(PortAddress::new) {
        let Some(address) = taken.free(universe, width) else {
            continue;
        };
        taken.hold(universe, address, width);
        return Ok(Placement {
            device: candidate.id.clone(),
            sku: candidate.device.sku.clone(),
            model: candidate.device.name.clone(),
            name: candidate.name.map(ToOwned::to_owned),
            groups: candidate.groups.to_vec(),
            personality,
            universe,
            address,
            width,
        });
    }
    Err(format!(
        "no universe from {first} holds the {width} channels of `{personality}`"
    ))
}

fn table(device: &Device, layout: Layout) -> Result<(Personality, Profile), String> {
    match layout {
        Layout::Fixed(personality) => Profile::of(device, personality)
            .map(|profile| (personality, profile))
            .map_err(|error| error.to_string()),
        Layout::Widest => profile::served(device)
            .into_iter()
            .flatten()
            .map(|profile| (profile.personality(), profile))
            .max_by_key(|(_, profile)| profile.width())
            .ok_or_else(|| format!("{} serves no channel table over `lan`", device.sku)),
    }
}

#[cfg(test)]
mod tests;
