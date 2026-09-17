//! The patch, joined to what each device is.
//!
//! The file names a device by identity, and the identity alone says nothing
//! about the channels the device answers to. The caller hands the devices it
//! found, and this is where the channel table, the width and the overlaps
//! come out.

use std::collections::{BTreeMap, BTreeSet};

use govee_toolkit::DeviceId;
use govee_toolkit::codec::Device;

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

/// Every fixture of a patch, checked.
///
/// No two fixtures share a channel, every fixture fits its universe, and
/// every device serves the personality the patch asks of it.
#[derive(Debug, Clone)]
pub struct Rig {
    fixtures: Vec<Fixture>,
}

impl Rig {
    /// Every fixture, in the order the patch lists them.
    #[must_use]
    pub fn fixtures(&self) -> &[Fixture] {
        &self.fixtures
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
    /// The patch, joined to the devices `device_of` answers with.
    ///
    /// Every entry is checked, and every fault is reported: an operator at a
    /// desk corrects the whole patch once, not one line per run.
    ///
    /// # Errors
    ///
    /// Every [`Error`] the entries carry: an address outside a universe, a
    /// fixture past the end of one, two fixtures on one channel, one device
    /// patched twice, a device `device_of` does not answer, and a
    /// personality the device serves through nothing.
    pub fn resolve<'a>(
        &self,
        device_of: impl Fn(&DeviceId) -> Option<&'a Device>,
    ) -> Result<Rig, Vec<Error>> {
        let mut fixtures = Vec::with_capacity(self.patch.len());
        let mut errors = Vec::new();
        let mut seen: BTreeSet<&DeviceId> = BTreeSet::new();
        for entry in &self.patch {
            if !seen.insert(&entry.device) {
                errors.push(Error::Twice {
                    device: entry.device.clone(),
                });
                continue;
            }
            match fixture(entry, &device_of) {
                Ok(fixture) => fixtures.push(fixture),
                Err(error) => errors.push(error),
            }
        }
        errors.extend(overlaps(&fixtures));
        if errors.is_empty() {
            Ok(Rig { fixtures })
        } else {
            Err(errors)
        }
    }
}

/// One entry, joined to its device.
fn fixture<'a>(
    entry: &Entry,
    device_of: &impl Fn(&DeviceId) -> Option<&'a Device>,
) -> Result<Fixture, Error> {
    let universe = entry.port_address()?;
    let device = device_of(&entry.device).ok_or_else(|| Error::Unknown {
        device: entry.device.clone(),
    })?;
    let profile = Profile::of(device, entry.personality).map_err(|source| Error::Unserved {
        device: entry.device.clone(),
        source,
    })?;
    let span = span(entry, universe, profile.width())?;
    Ok(Fixture {
        entry: entry.clone(),
        universe,
        profile,
        span,
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

/// Every pair of fixtures that answers to one channel.
///
/// The fixtures are sorted per port-address, and the pass carries the fixture
/// that reaches furthest. A wide fixture therefore reports the narrow ones it
/// covers, and not the first alone.
fn overlaps(fixtures: &[Fixture]) -> Vec<Error> {
    let mut per_universe: BTreeMap<PortAddress, Vec<&Fixture>> = BTreeMap::new();
    for fixture in fixtures {
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
