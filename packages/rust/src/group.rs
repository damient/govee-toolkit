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

/// A borrow of the SDK and the identities of the members, holding no state
/// of its own.
#[derive(Debug, Clone)]
pub struct GroupHandle<'a> {
    govee: &'a Govee,
    members: Vec<DeviceId>,
    pinned: Option<Mode>,
}

impl Govee {
    /// A handle for several devices. [`Govee::targets`] turns a group name
    /// into its members.
    #[must_use]
    pub fn group(&self, members: &[DeviceId]) -> GroupHandle<'_> {
        self.group_maybe_on(members, None)
    }

    /// A handle for several devices, each one driven over `mode` alone. A
    /// member that does not enable `mode` fails alone.
    #[must_use]
    pub fn group_on(&self, members: &[DeviceId], mode: Mode) -> GroupHandle<'_> {
        self.group_maybe_on(members, Some(mode))
    }

    /// A handle for several devices, pinned to `mode` where the caller names
    /// one — see [`Govee::device_maybe_on`].
    #[must_use]
    pub fn group_maybe_on(&self, members: &[DeviceId], mode: Option<Mode>) -> GroupHandle<'_> {
        GroupHandle {
            govee: self,
            members: members.to_vec(),
            pinned: mode,
        }
    }
}

impl<'a> GroupHandle<'a> {
    /// The members, in the order each [`Outcome`] list follows.
    #[must_use]
    pub fn members(&self) -> &[DeviceId] {
        &self.members
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
    /// command would go over — see [`Govee::ensure_known`].
    ///
    /// One member after the other: the first scan records every device that
    /// answered, so the next member is usually known already. Two scans at
    /// once would also compete for one Bluetooth adapter.
    pub async fn ensure_known(&self) -> Vec<Outcome<Mode>> {
        let mut outcomes = Vec::with_capacity(self.members.len());
        for id in &self.members {
            outcomes.push(Outcome {
                id: id.clone(),
                result: self.govee.ensure_known(id).await,
            });
        }
        outcomes
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
