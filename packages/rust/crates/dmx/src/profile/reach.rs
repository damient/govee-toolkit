//! What `lan` reaches on one device.
//!
//! Three statements in the device file decide a channel, and all three must
//! agree: the hardware declares the capability, `modes.lan` reaches it, and a
//! `lan` command claims the `role:` that drives it.

pub(super) use govee_toolkit::codec::capabilities::SEGMENTS;
use govee_toolkit::codec::{ArgBound, ArgRole, Device, Mode, Role};

/// The capability names the channel table reads.
///
/// They are the vocabulary of `devices/schema.yaml`, the way
/// [`SEGMENTS`] is. No SKU name and no command name reaches this crate.
pub(super) const POWER: &str = "power";
/// See [`POWER`].
pub(super) const BRIGHTNESS: &str = "brightness";
/// See [`POWER`].
pub(super) const COLOR: &str = "color";
/// See [`POWER`].
pub(super) const COLORTEMP: &str = "colortemp";

/// Whether `lan` reaches `capability` on this device and claims a command for
/// `role`.
pub(super) fn reaches(device: &Device, capability: &str, role: Role) -> bool {
    has(device, capability) && device.command_for(Mode::Lan, role).is_some()
}

/// Whether `lan` reaches `capability`, whatever commands the file declares.
pub(super) fn has(device: &Device, capability: &str) -> bool {
    let lan = device.modes.get(Mode::Lan);
    device.capabilities.has(capability)
        && !lan.unreachable.contains_key(capability)
        && lan
            .capabilities
            .resolve(&device.capabilities)
            .contains(&capability)
}

/// The pair the `lan` command claiming `role` declares for its `arg`
/// argument. `None` where the file declares no command, no such argument or
/// no bound on it.
pub(super) fn bounds(device: &Device, role: Role, arg: ArgRole) -> Option<[i64; 2]> {
    let (_, command) = device.entry_for(Mode::Lan, role)?;
    let name = command.arg_for(arg)?;
    match command.args.get(name)?.bound() {
        ArgBound::Range(pair) => Some(pair),
        ArgBound::Zones(_) | ArgBound::MaxLen(_) | ArgBound::None => None,
    }
}
