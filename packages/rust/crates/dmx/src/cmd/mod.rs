//! The subcommands, and what they report.
//!
//! [`govee_toolkit::exit`] carries the exit codes, the error record and the
//! two output forms, so `govee` and `govee-dmx` report a fault the same way.

pub(crate) mod identify;
pub(crate) mod observe;
pub(crate) mod patch;
pub(crate) mod profile;
pub(crate) mod rig;
pub(crate) mod run;

use govee_toolkit::exit::Failure;
use govee_toolkit_dmx::patch::{MAX_PORT_ADDRESS, PortAddress};
use govee_toolkit_dmx::profile::Personality;
use tracing_subscriber::EnvFilter;

/// Send the library traces to stderr, so a stream that stops says so.
///
/// Nothing acknowledges a LAN frame: a segment frame the transport refused and
/// a stream that ended are reported through `tracing` and nowhere else.
pub(crate) fn trace(debug: bool) {
    let default = if debug {
        "warn,govee_toolkit=debug,govee_toolkit_dmx=debug"
    } else {
        "warn"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    // A second install is the caller's error and not the operator's, and a run
    // without traces is better than a run that stops here.
    drop(
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .try_init(),
    );
}

pub(crate) fn spellings() -> String {
    Personality::ALL
        .iter()
        .map(|personality| format!("`{personality}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn port_address(universe: u16) -> Result<PortAddress, Failure> {
    PortAddress::new(universe).ok_or_else(|| {
        Failure::usage(format!(
            "universe {universe} is over the {MAX_PORT_ADDRESS} Art-Net holds"
        ))
    })
}
