# Changelog

Changes to `govee-toolkit-dmx`, the crate that builds the `govee-dmx` binary
from `packages/rust/crates/dmx`. It versions apart from `govee-toolkit` and
releases under `dmx-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md). The design
is [`../../../../docs/dmx.md`](../../../../docs/dmx.md).

Nothing is published yet: the manifest carries `publish = false`.

### Added

- The crate, the `govee-dmx` binary and the `artnet` and `sacn` cargo
  features. The binary answers `--version` and `--help`, and drives no device
  yet.
- `profile` — the channel table, derived from the device file and from nothing
  else. It answers the personalities a device serves over `lan`, and the
  channels of each: the offset, what the channel drives, and the pair a scaled
  channel writes into. A table wider than one universe is an error, never a
  truncation.
- `profile::Scale` — a slot scaled into what a device parameter takes, plus
  the step count the pair resolves to. Slot 0 carries no value: the dimmer
  powers the device off there, and the white channel sends no command.
- `govee-dmx profile <SKU>` — the channel table of one device, with every
  channel, its offset, and the step count of each scaled channel.
- `profile --personality <name>` prints one table, and `--json` prints the
  record a machine reads. A personality the device serves through nothing
  exits 5, and a name no personality carries exits 2.
- `report` — one channel table as the text an operator reads, and as the JSON
  `--json` prints.
- `patch` — the patch file: the node settings, and one entry per fixture in
  either spelling of the Art-Net address. An unknown key is refused.
- `Patch::resolve` — the patch joined to the devices the bridge found, as a
  `Rig`. It reports every fault at once, so a desk is corrected in one pass.
- `patch::Error` — an overlap, an address outside a universe, a fixture past
  the end of one, a device patched twice, and a personality served by nothing.
- `input::UniverseFrame` — one universe of channel values, from whichever
  protocol carried it. `slot()` reads a channel at the address a desk shows.
- `input::artnet` — the `ArtDmx` parser, behind the `artnet` feature. It drops
  an odd length, a length over 512 and a protocol version under 14.
- `input::artnet::Sequence` — drops a packet older than the last one accepted,
  in a window of 256. A sender that writes 0 disables the check.
- `input::artnet::Gate` — one `Sequence` per sender and port-address, so the
  packets of one sender never drop the packets of another.
- `input::socket::Listener` — the UDP socket the node receives on. It carries
  `SO_REUSEADDR`, because port 6454 is fixed by the protocol and a second
  Art-Net application on the host must start too.
- `apply::Look` — what one frame asks of one fixture: the power, the
  brightness, the color, the white temperature and the zones, in the units the
  device file declares.
- `apply::Applier` — the send path, one task per fixture. It writes only what
  changed, sends the current values again after the refresh interval, and
  counts the looks a later one replaced. A device that fails stops no other
  one.
- `node::Node` — the run loop: receive, drop what the protocol refuses,
  resolve against the patch, and hand each fixture its look. It prints nothing
  and reports through `node::Observer`.
- `govee-dmx run <patch>` — receive Art-Net on port 6454 and drive the patched
  devices. `--config` names the configuration file, and `--json` prints the
  records a machine reads.
- `govee-dmx run --dry-run` — print every packet and what each fixture reads
  out of it, and write to no device. It debugs a patch before any device is at
  risk.
- A device the patch names that enables no `lan` mode fails at the start and is
  named. The bridge drives a device over `lan` alone and substitutes no other
  mode.
