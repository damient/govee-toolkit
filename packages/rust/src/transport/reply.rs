//! What a read returned.

use crate::codec::Captured;
use crate::transport::DeviceId;

/// The fields one command's `reply:` layouts captured.
///
/// Nothing here interprets a field name: the device file named them, and a
/// caller that wants to know what a name means reads the same file. What the
/// SDK does model — whether the device is on, and how bright — it finds by
/// role, on [`DeviceStatus`](crate::transport::DeviceStatus).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Which device answered.
    pub id: DeviceId,
    /// Every field the exchanges captured, merged in the order they went out.
    pub fields: Captured,
}
