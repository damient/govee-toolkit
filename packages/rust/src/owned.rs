use crate::codec::Mode;
use crate::device::DeviceHandle;
use crate::devices::Devices;
use crate::error::Result;
use crate::govee::Govee;
use crate::select::{Filter, Target};
use crate::transport::DeviceId;

/// One identity on an SDK, with its pin, for a caller that cannot hold a
/// borrow.
#[derive(Debug, Clone)]
pub struct OwnedDevice {
    govee: Govee,
    id: DeviceId,
    pinned: Option<Mode>,
}

impl OwnedDevice {
    /// The device's identity.
    #[must_use]
    pub fn id(&self) -> &DeviceId {
        &self.id
    }

    /// The one mode every call goes over, or `None`.
    #[must_use]
    pub fn pinned(&self) -> Option<Mode> {
        self.pinned
    }

    /// The handle for one call.
    #[must_use]
    pub fn handle(&self) -> DeviceHandle<'_> {
        DeviceHandle::new(&self.govee, self.id.clone(), self.pinned)
    }
}

/// The identities a filter selected on an SDK, with one pin for every member.
#[derive(Debug, Clone)]
pub struct OwnedDevices {
    govee: Govee,
    ids: Vec<DeviceId>,
    pinned: Option<Mode>,
}

impl OwnedDevices {
    /// The members, in the order that each outcome list follows.
    #[must_use]
    pub fn ids(&self) -> &[DeviceId] {
        &self.ids
    }

    /// The one mode every call goes over, or `None`.
    #[must_use]
    pub fn pinned(&self) -> Option<Mode> {
        self.pinned
    }

    /// One owned handle per member, in member order.
    #[must_use]
    pub fn members(&self) -> Vec<OwnedDevice> {
        self.ids
            .iter()
            .map(|id| OwnedDevice {
                govee: self.govee.clone(),
                id: id.clone(),
                pinned: self.pinned,
            })
            .collect()
    }

    /// The set for one call. It scans nothing.
    #[must_use]
    pub fn devices(&self) -> Devices<'_> {
        self.ids
            .iter()
            .map(|id| DeviceHandle::new(&self.govee, id.clone(), self.pinned))
            .collect()
    }
}

impl Govee {
    /// [`Govee::device`], as a handle that owns a clone of the SDK.
    ///
    /// # Errors
    ///
    /// As for [`Govee::device`].
    pub fn device_owned<'t>(
        &self,
        target: impl Into<Target<'t>>,
        mode: Option<Mode>,
    ) -> Result<OwnedDevice> {
        let id = self.device(target, mode)?.id().clone();
        Ok(OwnedDevice {
            govee: self.clone(),
            id,
            pinned: mode,
        })
    }

    /// [`Govee::devices`], as a set that owns a clone of the SDK.
    ///
    /// # Errors
    ///
    /// As for [`Govee::devices`].
    pub async fn devices_owned(&self, filter: Filter, mode: Option<Mode>) -> Result<OwnedDevices> {
        let ids = self
            .devices(filter, mode)
            .await?
            .iter()
            .map(|handle| handle.id().clone())
            .collect();
        Ok(OwnedDevices {
            govee: self.clone(),
            ids,
            pinned: mode,
        })
    }
}
