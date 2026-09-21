//! The patch, joined to what each device is.
//!
//! The file names a device by identity, and the identity alone says nothing
//! about the channels the device answers to. The caller hands the devices it
//! found, and this is where the channel table, the width and the overlaps
//! come out.

use std::collections::{BTreeMap, BTreeSet};

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Device;
use serde_json::{Value, json};

use super::Patch;
use super::address::PortAddress;
use super::entry::Entry;
use super::error::{Error, Span};
use crate::profile::{Profile, UNIVERSE};

/// One patched fixture: the entry, the device it drives and the channels it
/// answers to.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The line the patch carries.
    pub entry: Entry,
    /// The port-address the entry spells.
    pub universe: PortAddress,
    /// The channel table of the personality the entry asks for.
    pub profile: Profile,
    /// The channels it takes, inside `universe`.
    pub span: Span,
}

impl Fixture {
    /// The fixture, as every `--json` form of the node names it.
    #[must_use]
    pub fn json(&self) -> Value {
        json!({
            "device": self.entry.device.to_string(),
            "universe": self.universe.get(),
            "first": self.span.first,
            "last": self.span.last,
            "personality": self.profile.personality().as_str(),
        })
    }
}

/// Every fixture of a patch, checked.
///
/// No two driven fixtures contend for a channel, every fixture fits its
/// universe, and every device serves the personality the patch asks of it.
#[derive(Debug, Clone)]
pub struct Rig {
    fixtures: Vec<Fixture>,
    reserved: Vec<Fixture>,
}

impl Rig {
    /// Every driven fixture, in the order the patch lists them.
    #[must_use]
    pub fn fixtures(&self) -> &[Fixture] {
        &self.fixtures
    }

    /// Every disabled entry that still holds its channels. The bridge sends
    /// it nothing, and the patch command hands those channels to no new
    /// fixture. A driven fixture the operator patches over them is allowed.
    #[must_use]
    pub fn reserved(&self) -> &[Fixture] {
        &self.reserved
    }

    /// Every driven fixture on one port-address, in patch order.
    ///
    /// `address` keeps the fixtures that answer to that channel, which is the
    /// channel a desk shows and not only a start address. Without it, the
    /// whole universe answers.
    #[must_use]
    pub fn at(&self, universe: PortAddress, address: Option<u16>) -> Vec<&Fixture> {
        self.fixtures
            .iter()
            .filter(|fixture| fixture.universe == universe)
            .filter(|fixture| {
                address.is_none_or(|channel| {
                    fixture.span.first <= channel && channel <= fixture.span.last
                })
            })
            .collect()
    }

    /// The port-addresses the rig answers on, each one once and in order.
    /// `ArtPollReply` carries 4 of them per reply.
    #[must_use]
    pub fn universes(&self) -> Vec<PortAddress> {
        let mut addresses: Vec<PortAddress> = self
            .fixtures
            .iter()
            .map(|fixture| fixture.universe)
            .collect();
        addresses.sort_unstable();
        addresses.dedup();
        addresses
    }
}

impl Patch {
    /// The patch, joined to the devices `found` answers with.
    ///
    /// `known` answers the device file of a SKU, and sizes a disabled entry
    /// whose device did not answer: those channels stay reserved.
    ///
    /// Every entry is checked, and every fault is reported: an operator at a
    /// desk corrects the whole patch once, not one line per run.
    ///
    /// # Errors
    ///
    /// Every [`Error`] the entries carry: an address outside a universe, a
    /// fixture past the end of one, two driven fixtures on one channel under
    /// different tables, one device patched twice, an enabled entry `found`
    /// does not answer, and a personality the device serves through nothing.
    pub fn resolve<'a>(
        &self,
        found: impl Fn(&DeviceId) -> Option<&'a Device>,
        known: impl Fn(&str) -> Option<&'a Device>,
    ) -> Result<Rig, Vec<Error>> {
        let mut placed = Vec::with_capacity(self.patch.len());
        let mut errors = Vec::new();
        let mut seen: BTreeSet<&DeviceId> = BTreeSet::new();
        for entry in &self.patch {
            if entry.enabled && !seen.insert(&entry.device) {
                errors.push(Error::Twice {
                    device: entry.device.clone(),
                });
                continue;
            }
            match fixture(entry, &found, &known) {
                Ok(Some(fixture)) => placed.push(fixture),
                // A disabled entry that nothing sizes holds no channel. The
                // patch command is what refuses it, because it is the one that
                // hands the channels to another fixture.
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
        errors.extend(overlaps(&placed));
        if !errors.is_empty() {
            return Err(errors);
        }
        let (fixtures, reserved) = placed.into_iter().partition(|f| f.entry.enabled);
        Ok(Rig { fixtures, reserved })
    }
}

/// One entry, joined to its device.
///
/// A disabled entry takes the device file its `sku:` names where the device
/// itself did not answer, and answers `None` where neither states a width.
fn fixture<'a>(
    entry: &Entry,
    found: &impl Fn(&DeviceId) -> Option<&'a Device>,
    known: &impl Fn(&str) -> Option<&'a Device>,
) -> Result<Option<Fixture>, Error> {
    let universe = entry.port_address()?;
    let Some(device) = sized(entry, found, known)? else {
        return Ok(None);
    };
    let profile = Profile::of(device, entry.personality).map_err(|source| Error::Unserved {
        device: entry.device.clone(),
        source,
    })?;
    let span = span(entry, universe, profile.width())?;
    Ok(Some(Fixture {
        entry: entry.clone(),
        universe,
        profile,
        span,
    }))
}

/// The device file that states what the entry answers to.
///
/// An enabled entry needs the device this run found: the bridge drives what
/// answered, and a fixture nothing reached is a fault the operator must see.
fn sized<'a>(
    entry: &Entry,
    found: &impl Fn(&DeviceId) -> Option<&'a Device>,
    known: &impl Fn(&str) -> Option<&'a Device>,
) -> Result<Option<&'a Device>, Error> {
    if let Some(device) = found(&entry.device) {
        return Ok(Some(device));
    }
    if !entry.enabled {
        return Ok(entry.sku.as_deref().and_then(known));
    }
    Err(Error::Unknown {
        device: entry.device.clone(),
    })
}

/// The channels the fixture takes, from its start address.
fn span(entry: &Entry, universe: PortAddress, width: u16) -> Result<Span, Error> {
    if entry.address == 0 || entry.address > UNIVERSE {
        return Err(Error::StartAddress {
            device: entry.device.clone(),
            address: entry.address,
        });
    }
    let last = entry.address.saturating_add(width.saturating_sub(1));
    let span = Span {
        universe,
        first: entry.address,
        last,
    };
    if last > UNIVERSE {
        return Err(Error::PastUniverse {
            device: entry.device.clone(),
            personality: entry.personality,
            span,
        });
    }
    Ok(span)
}

/// Every pair of driven fixtures that answers to one channel.
///
/// A disabled entry is out of the pass. It takes no frame, so nothing
/// contends for the channels it holds, and a driven fixture can cover them.
///
/// A clone is out of the pass too: two fixtures on one span under one
/// personality answer to one channel table, and the desk drives both from one
/// set of values.
///
/// The fixtures are sorted per port-address, and the pass carries the fixture
/// that reaches furthest. A wide fixture therefore reports the narrow ones it
/// covers, and not the first alone.
fn overlaps(fixtures: &[Fixture]) -> Vec<Error> {
    let mut per_universe: BTreeMap<PortAddress, Vec<&Fixture>> = BTreeMap::new();
    for fixture in fixtures.iter().filter(|fixture| fixture.entry.enabled) {
        per_universe
            .entry(fixture.universe)
            .or_default()
            .push(fixture);
    }
    let mut errors = Vec::new();
    for mut sharing in per_universe.into_values() {
        sharing.sort_by_key(|fixture| fixture.span.first);
        let mut open: Option<&Fixture> = None;
        for second in sharing {
            if let Some(first) = open
                && second.span.first <= first.span.last
                && !clones(first, second)
            {
                errors.push(Error::Overlap {
                    first: first.entry.device.clone(),
                    first_span: first.span,
                    second: second.entry.device.clone(),
                    second_span: second.span,
                });
            }
            if open.is_none_or(|first| second.span.last > first.span.last) {
                open = Some(second);
            }
        }
    }
    errors
}

/// Whether the two fixtures answer to one channel table on one span.
///
/// The personality and the width decide the table, so one span under one
/// personality is one table on both. An operator patches a pair that way to
/// drive it from one set of channels.
fn clones(first: &Fixture, second: &Fixture) -> bool {
    first.span == second.span && first.entry.personality == second.entry.personality
}
