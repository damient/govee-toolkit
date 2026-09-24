//! Several devices driven as one: a verb goes to every member at once.
//!
//! A member that fails stops no other one. Each member answers its own
//! [`Outcome`], over its own enabled modes: the group substitutes no mode, and
//! a member whose device file carries no entry for the verb fails alone.

use std::future::Future;

use futures_util::future::join_all;

use crate::codec::Mode;
use crate::device::DeviceHandle;
use crate::error::Result;
use crate::event::Served;
use crate::govee::Govee;
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

/// A borrow of the SDK and of the identities of the members, holding no
/// state of its own.
#[derive(Debug, Clone)]
pub struct GroupHandle<'a> {
    govee: &'a Govee,
    members: &'a [DeviceId],
    pinned: Option<Mode>,
}

impl Govee {
    /// A handle for several devices. [`Govee::targets`] turns a group name
    /// into its members.
    #[must_use]
    pub fn group<'a>(&'a self, members: &'a [DeviceId]) -> GroupHandle<'a> {
        self.group_maybe_on(members, None)
    }

    /// A handle for several devices, each one driven over `mode` alone. A
    /// member that does not enable `mode` fails alone.
    #[must_use]
    pub fn group_on<'a>(&'a self, members: &'a [DeviceId], mode: Mode) -> GroupHandle<'a> {
        self.group_maybe_on(members, Some(mode))
    }

    /// A handle for several devices, pinned to `mode` where the caller names
    /// one — see [`Govee::device_maybe_on`].
    #[must_use]
    pub fn group_maybe_on<'a>(
        &'a self,
        members: &'a [DeviceId],
        mode: Option<Mode>,
    ) -> GroupHandle<'a> {
        GroupHandle {
            govee: self,
            members,
            pinned: mode,
        }
    }
}

impl<'a> GroupHandle<'a> {
    /// The members, in the order each [`Outcome`] list follows.
    #[must_use]
    pub fn members(&self) -> &[DeviceId] {
        self.members
    }

    /// Run `verb` on every member at once, and answer one [`Outcome`] per
    /// member, in member order.
    pub async fn each<T, F, Fut>(&self, verb: F) -> Vec<Outcome<T>>
    where
        F: Fn(DeviceHandle<'a>) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let calls = self.members.iter().map(|id| {
            let call = verb(self.govee.device_maybe_on(id, self.pinned));
            async move {
                Outcome {
                    id: id.clone(),
                    result: call.await,
                }
            }
        });
        join_all(calls).await
    }

    /// Scan for every member that no mode knows yet, and answer the mode a
    /// command would go over — see [`Govee::ensure_known`]. A pinned group
    /// scans over its mode alone, and a member that does not enable it fails
    /// alone.
    ///
    /// Every member scans at once, so members that are absent cost one scan
    /// window and not one each. A `lan` scan reads every reply, whichever
    /// request it answers. `ble` runs one scan at a time on its adapter, and
    /// a member that an earlier scan heard does not scan again.
    pub async fn ensure_known(&self) -> Vec<Outcome<Mode>> {
        let calls = self.members.iter().map(|id| async move {
            let result = match self.pinned {
                Some(mode) => self.govee.ensure_known_on(id, mode).await,
                None => self.govee.ensure_known(id).await,
            };
            Outcome {
                id: id.clone(),
                result,
            }
        });
        join_all(calls).await
    }

    /// [`DeviceHandle::power`] on every member.
    pub async fn power(&self, on: bool) -> Vec<Outcome> {
        self.each(|handle| async move { handle.power(on).await })
            .await
    }

    /// [`DeviceHandle::brightness`] on every member. The range is each
    /// member's own, so one level can be in range for one member and out of
    /// range for another.
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

    /// [`DeviceHandle::segment`] on every member. A zone list reads against
    /// each member's own zones.
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
