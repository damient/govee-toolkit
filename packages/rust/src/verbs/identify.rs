//! Light one device, so a person sees which identity drives which fixture.
//!
//! The pass powers the device on and paints one color. A caller that walks a
//! rig powers every device off first, and then lights one device at a time:
//! the dark room is what makes the answer readable. [`IDENTIFY_WAIT`] is the
//! interval those callers wait between two steps.
//!
//! The look the device held is lost. Nothing reads it back first — a caller
//! that must restore it reads the status itself before the pass.

use std::time::Duration;

use super::RoleEntry;
use crate::codec::{ArgBound, ArgRole, Role};
use crate::device::DeviceHandle;
use crate::error::Result;

/// The color a pass paints where the caller names none.
pub const IDENTIFY_COLOR: [u8; 3] = [0, 255, 0];

/// How long a walk waits between two steps, so a person reads the room
/// between two fixtures.
pub const IDENTIFY_WAIT: Duration = Duration::from_secs(1);

/// How long a walk holds the last device lit before every device goes off.
pub const IDENTIFY_HOLD: Duration = Duration::from_secs(5);

/// What one identify pass shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identify {
    /// The color the device paints.
    pub color: [u8; 3],
    /// Whether the pass sets the brightness to the top of the range the
    /// device file declares. A fixture the operator left dim shows the color
    /// weakly otherwise.
    pub full_brightness: bool,
}

impl Default for Identify {
    fn default() -> Self {
        Self {
            color: IDENTIFY_COLOR,
            full_brightness: true,
        }
    }
}

impl DeviceHandle<'_> {
    /// Power the device on and paint one color.
    ///
    /// The pass sends the `power` role, the `brightness` role and the `color`
    /// role, over the mode a send goes over now. It waits
    /// [`DeviceHandle::command_gap`] between two of them, which one device
    /// needs to apply all three.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`] and [`DeviceHandle::color`]. A device
    /// file that marks no `role: brightness` costs nothing: the pass leaves
    /// the brightness alone.
    pub async fn identify(&self, options: &Identify) -> Result<()> {
        let gap = self.command_gap()?;
        self.power(true).await?;
        tokio::time::sleep(gap).await;
        if options.full_brightness
            && let Some(level) = self.brightest()
        {
            self.brightness(level).await?;
            tokio::time::sleep(gap).await;
        }
        self.color(options.color).await?;
        Ok(())
    }

    /// The top of the range the `brightness` argument declares, and `None`
    /// where the device file states no such command, argument or range.
    fn brightest(&self) -> Option<i64> {
        let entry = self.role_entry(Role::Brightness).ok()?;
        let name = entry.arg(ArgRole::Brightness).ok()?;
        range(&entry, name).map(|[_, max]| max)
    }
}

/// The pair `name` declares on the entry, and `None` where it declares
/// another bound or none.
fn range(entry: &RoleEntry<'_>, name: &str) -> Option<[i64; 2]> {
    match entry.bound(name)? {
        ArgBound::Range(pair) => Some(pair),
        ArgBound::Zones(_) | ArgBound::MaxLen(_) | ArgBound::None => None,
    }
}
