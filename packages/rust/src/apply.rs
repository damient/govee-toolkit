//! Several role verbs on several devices, sent in one fixed order.

use crate::codec::Mode;
use crate::device::DeviceHandle;
use crate::devices::{Devices, Outcome};
use crate::error::Error;
use crate::stream::Resolution;
use crate::transport::DeviceId;
use crate::verbs::{Music, Paint};

/// One role verb, for [`Devices::apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    /// [`crate::DeviceHandle::power`].
    Power(bool),
    /// [`crate::DeviceHandle::brightness`].
    Brightness(i64),
    /// [`crate::DeviceHandle::color`].
    Color([u8; 3]),
    /// [`crate::DeviceHandle::color_temp`], in kelvin.
    ColorTemp(i64),
    /// [`crate::DeviceHandle::segment`], with the fields of a [`Paint`].
    Segment {
        /// `None` paints every zone.
        zones: Option<Vec<u16>>,
        /// One color for every zone, or one per zone in order.
        colors: Vec<[u8; 3]>,
        /// The zone count the colors address.
        resolution: Resolution,
        /// Whether the firmware interpolates between zones.
        gradient: bool,
    },
    /// [`crate::DeviceHandle::gradient`].
    Gradient(bool),
    /// [`crate::DeviceHandle::music`].
    Music(Music),
}

impl Verb {
    /// The name that a step reports, in snake case.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Power(_) => "power",
            Self::Brightness(_) => "brightness",
            Self::Color(_) => "color",
            Self::ColorTemp(_) => "color_temp",
            Self::Segment { .. } => "segment",
            Self::Gradient(_) => "gradient",
            Self::Music(_) => "music",
        }
    }

    /// Power on goes first, so that the later verbs reach a lit device.
    fn rank(&self) -> u8 {
        match self {
            Self::Power(true) => 0,
            Self::Gradient(_) => 1,
            Self::Brightness(_) => 2,
            Self::ColorTemp(_) => 3,
            Self::Color(_) => 4,
            Self::Segment { .. } => 5,
            Self::Music(_) => 6,
            Self::Power(false) => 7,
        }
    }
}

/// What one verb of [`Devices::apply`] answered.
#[derive(Debug)]
pub struct AppliedStep {
    /// [`Verb::name`].
    pub name: &'static str,
    /// One per member that the step reached, in member order.
    pub outcomes: Vec<Outcome>,
}

/// What [`Devices::apply`] answered.
#[derive(Debug)]
pub struct Applied {
    /// [`Devices::ensure_known`], run before the verbs.
    pub reached: Vec<Outcome<Mode>>,
    /// One per verb, in the order sent.
    pub steps: Vec<AppliedStep>,
}

impl Applied {
    /// Every failure: the scan first, then each step.
    pub fn failures(&self) -> impl Iterator<Item = (&DeviceId, &Error)> {
        let reached = self.reached.iter().filter_map(failed);
        let steps = self
            .steps
            .iter()
            .flat_map(|step| step.outcomes.iter().filter_map(failed));
        reached.chain(steps)
    }

    /// Whether every member took every step.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failures().next().is_none()
    }
}

fn failed<T>(outcome: &Outcome<T>) -> Option<(&DeviceId, &Error)> {
    outcome
        .result
        .as_ref()
        .err()
        .map(|error| (&outcome.id, error))
}

impl Devices<'_> {
    /// Send one verb to every member.
    pub async fn play(&self, verb: &Verb) -> Vec<Outcome> {
        match verb {
            Verb::Power(on) => self.power(*on).await,
            Verb::Brightness(level) => self.brightness(*level).await,
            Verb::Color(rgb) => self.color(*rgb).await,
            Verb::ColorTemp(kelvin) => self.color_temp(*kelvin).await,
            Verb::Segment {
                zones,
                colors,
                resolution,
                gradient,
            } => {
                self.segment(&Paint {
                    zones: zones.as_deref(),
                    colors,
                    resolution: *resolution,
                    gradient: *gradient,
                })
                .await
            }
            Verb::Gradient(on) => self.gradient(*on).await,
            Verb::Music(music) => self.music(music).await,
        }
    }

    /// Scan for the members, then send the verbs, power on first and power off
    /// last. A member that fails takes no later step, and stops no other
    /// member.
    pub async fn apply(&self, mut verbs: Vec<Verb>) -> Applied {
        verbs.sort_by_key(Verb::rank);
        let reached = self.ensure_known().await;
        let mut live = passed(self.members(), &reached);
        let mut steps = Vec::with_capacity(verbs.len());
        for verb in &verbs {
            if live.is_empty() {
                break;
            }
            let outcomes = Devices::new(live.clone()).play(verb).await;
            live = passed(&live, &outcomes);
            steps.push(AppliedStep {
                name: verb.name(),
                outcomes,
            });
        }
        Applied { reached, steps }
    }
}

/// Each outcome answers the member at its place.
fn passed<'a, T>(members: &[DeviceHandle<'a>], outcomes: &[Outcome<T>]) -> Vec<DeviceHandle<'a>> {
    members
        .iter()
        .zip(outcomes)
        .filter(|(_, outcome)| outcome.result.is_ok())
        .map(|(handle, _)| handle.clone())
        .collect()
}
