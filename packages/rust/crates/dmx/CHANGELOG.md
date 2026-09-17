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
- `profile::Personality` — `full`, `segment` and `pixel`. Channel 1 is the
  dimmer and channel 2 the mode channel on all three, so a cue carries between
  models of different widths.
- `full` takes 6 channels on every device. Where `lan` reaches no white
  temperature, channel 6 holds its place and drives nothing, and the table
  names it `unreached`.
- A device whose every zone is one addressable LED serves `pixel` alone:
  `segment` would lay out the same table.
- `profile::Scale` — a slot scaled into what a device parameter takes, plus
  the step count the pair resolves to. The dimmer and the white channel carry
  no value at slot 0: the dimmer powers the device off there, and the white
  channel sends no command.
- A color component scales over the pair its `lan` command declares, from
  slot 0. A component at 0 is a color the device shows, so the channel has no
  off.
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
- `input::artnet::Poll` — the `ArtPoll` parser. A poll takes 14 bytes, under
  the 18 an `ArtDmx` header takes, so the parser reads the opcode before it
  asks for a header.
- `input::artnet::replies` — one `ArtPollReply` for each group of 4
  port-addresses the patch holds. The node answers every poll, so a desk lists
  it with no manual entry of its address.
- `node::Node::named` — the name a desk lists the node under, which
  `govee-dmx run` takes from the patch.
- `input::socket::Listener::local_ip_towards` — the address of the interface
  that reaches one peer. A node bound to `0.0.0.0` answers a poll with it,
  because `0.0.0.0` is an address nothing can send to.
- `node.signal_loss_secs` — how long a fixture waits for a frame before
  `on_signal_loss` decides what it shows. The default is 4 seconds.
- `apply::Look::quiet` — what a fixture shows once the sender has gone quiet:
  `hold` keeps the last look, `black` takes every color to 0 and keeps the
  device on, and `off` powers it down.
- A fixture whose device stopped answering is retried on a backoff, which
  doubles from 250 ms to 8 seconds. A write that lands clears it, and every
  other fixture keeps running.
- `report::personalities` — the channel tables as the JSON array alone.
  `dist/catalog.json` carries it beside each device, so the catalog, the site
  and the node read one table.

### Changed

- `apply::Applier::start` and `node::Node::live` take an `apply::Timing`
  rather than the refresh interval alone.
