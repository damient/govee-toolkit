//! What a `cloud` command declares, and what a transport needs to send it.
//!
//! This wire carries no frame and no envelope of its own: the API names a
//! capability, and the value the device file builds goes in beside that name.
//! Nothing here knows a URL — the route is a property of the transport, the
//! same way a port is on `lan`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The capability a command writes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Capability {
    /// The capability name, as the API spells it. `type` in the device file
    /// and in the request.
    #[serde(rename = "type")]
    pub kind: String,
    /// Which instance of it the command writes.
    pub instance: String,
}

/// Which argument one capability answers into, on a status read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Read {
    /// The capability instance the answer carries.
    pub instance: String,
    /// The declared argument that holds it. Its `role:` says which field of
    /// the reported status it is, so no instance name reaches the transport.
    pub arg: String,
}

/// What a transport needs beyond the body, for one encoded cloud command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Which argument each capability instance answers into. Empty on a
    /// command that only writes.
    pub reads: BTreeMap<String, String>,
}
