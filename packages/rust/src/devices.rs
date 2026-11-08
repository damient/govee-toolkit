//! Several devices driven as one. Each member answers its own [`Outcome`] over
//! its own modes, and a member that fails stops no other one.

use std::future::Future;

use futures_util::future::join_all;

use crate::codec::Mode;
use crate::device::DeviceHandle;
use crate::error::Result;
use crate::event::{Device, Served};
use crate::transport::DeviceId;
use crate::verbs::{Music, Paint};

/// What one member answered.
#[derive(Debug)]
pub struct Outcome<T = Served> {
    /// The member.
    pub id: DeviceId,
    /// What the call on that member returned.
    pub result: Result<T>,
}

/// The devices that [`Govee::devices`](crate::Govee::devices) selects. Each
/// call here runs on every member at once, and answers one [`Outcome`] per
/// member, in member order.
#[derive(Debug, Clone)]
pub struct Devices<'a> {
    members: Vec<DeviceHandle<'a>>,
}

impl<'a> Devices<'a> {
    pub(crate) fn new(members: Vec<DeviceHandle<'a>>) -> Self {
        Self { members }
    }

    /// The members, in the order that each [`Outcome`] list follows.
    #[must_use]
    pub fn members(&self) -> &[DeviceHandle<'a>] {
        &self.members
    }

    /// The members, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, DeviceHandle<'a>> {
        self.members.iter()
    }

    /// The number of members.
    #[must_use]
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the filter selected no device.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// What the SDK holds for each member: the SKU, the name, the groups, the
    /// modes and the health. Reads no hardware.
    ///
    /// A member that no transport knows and that the configuration pins no
    /// SKU for is not in the list. [`Devices::members`] holds every member.
    #[must_use]
    pub fn list(&self) -> Vec<Device> {
        self.members
            .iter()
            .filter_map(|handle| {
                let sku = handle.govee.sku(handle.id()).ok()?;
                Some(handle.govee.describe(handle.id(), &sku))
            })
            .collect()
    }

    /// Run `verb` on every member at once.
    pub async fn each<T, F, Fut>(&self, verb: F) -> Vec<Outcome<T>>
    where
        F: Fn(DeviceHandle<'a>) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let calls = self.members.iter().map(|handle| {
            let call = verb(handle.clone());
            async move {
                Outcome {
                    id: handle.id().clone(),
                    result: call.await,
                }
            }
        });
        join_all(calls).await
    }

    /// [`DeviceHandle::ensure_known`] on every member at once, so absent
    /// members cost one scan window between them.
    pub async fn ensure_known(&self) -> Vec<Outcome<Mode>> {
        self.each(|handle| async move { handle.ensure_known().await })
            .await
    }

    /// [`DeviceHandle::power`] on every member.
    pub async fn power(&self, on: bool) -> Vec<Outcome> {
        self.each(|handle| async move { handle.power(on).await })
            .await
    }

    /// [`DeviceHandle::brightness`] on every member, against its own range.
    pub async fn brightness(&self, level: i64) -> Vec<Outcome> {
        self.each(|handle| async move { handle.brightness(level).await })
            .await
    }

    /// [`DeviceHandle::color`] on every member.
    pub async fn color(&self, rgb: [u8; 3]) -> Vec<Outcome> {
        self.each(|handle| async move { handle.color(rgb).await })
            .await
    }

    /// [`DeviceHandle::color_temp`] on every member.
    pub async fn color_temp(&self, kelvin: i64) -> Vec<Outcome> {
        self.each(|handle| async move { handle.color_temp(kelvin).await })
            .await
    }

    /// [`DeviceHandle::segment`] on every member, against its own zones.
    pub async fn segment(&self, paint: &Paint<'_>) -> Vec<Outcome> {
        self.each(|handle| async move { handle.segment(paint).await })
            .await
    }

    /// [`DeviceHandle::gradient`] on every member.
    pub async fn gradient(&self, on: bool) -> Vec<Outcome> {
        self.each(|handle| async move { handle.gradient(on).await })
            .await
    }

    /// [`DeviceHandle::music`] on every member.
    pub async fn music(&self, music: &Music) -> Vec<Outcome> {
        self.each(|handle| async move { handle.music(music).await })
            .await
    }
}

/// Handles from [`Govee::device`](crate::Govee::device) make a set with no
/// resolution and no scan, each member with its own pin.
impl<'a> FromIterator<DeviceHandle<'a>> for Devices<'a> {
    fn from_iter<I: IntoIterator<Item = DeviceHandle<'a>>>(handles: I) -> Self {
        Self::new(handles.into_iter().collect())
    }
}

impl<'s, 'a> IntoIterator for &'s Devices<'a> {
    type IntoIter = std::slice::Iter<'s, DeviceHandle<'a>>;
    type Item = &'s DeviceHandle<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.iter()
    }
}

impl<'a> IntoIterator for Devices<'a> {
    type IntoIter = std::vec::IntoIter<DeviceHandle<'a>>;
    type Item = DeviceHandle<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.into_iter()
    }
}
