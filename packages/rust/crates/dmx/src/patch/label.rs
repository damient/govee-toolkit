//! How a message names one entry.

use std::fmt;

use govee_toolkit::DeviceId;

/// One entry, as a message names it. The operator reads the name the patch
/// gives, and the identity tells two entries of one name apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The identity the entry names.
    pub device: DeviceId,
    /// The name the entry gives the fixture, if any.
    pub name: Option<String>,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{name} ({})", self.device),
            None => write!(f, "{}", self.device),
        }
    }
}
