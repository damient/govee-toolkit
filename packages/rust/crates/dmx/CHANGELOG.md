# Changelog

Changes to `govee-toolkit-dmx`, the crate that builds the `govee-dmx` binary
from `packages/rust/crates/dmx`. It versions apart from `govee-toolkit` and
releases under `dmx-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md). The design
is [`../../../../docs/dmx.md`](../../../../docs/dmx.md).

## [0.3.0] — 2026-09-24

### Added

- A patch entry takes `name:`, the name of the fixture. Messages and the
  `--json` forms carry it. See [`docs/dmx.md`](../../../../docs/dmx.md) 2.
- `govee-dmx identify` reads the `name:` of the patch first, then
  `config.yaml`. A name that the two give to different devices is refused.
- `govee-dmx patch` writes the name that `config.yaml` gives a device into
  the new entry.
- A patch entry takes `groups:`. `govee-dmx identify` reads a group of the
  patch first, then `config.yaml`, and lights it in patch order.
- A group that the patch and `config.yaml` give to different devices is
  refused, and so is a bare target that reads as a name and a group.
- `govee-dmx patch` writes the groups that `config.yaml` gives a device into
  the new entry. The `--json` forms of the fixtures carry `groups`.
- The `added` line of `govee-dmx patch`, `--dry-run` included, shows the
  name and the groups of the new entry.

### Changed

- **Breaking:** `patch::Error` names an entry by `patch::Label`, the identity
  and the name, in place of a `DeviceId`.
- **Breaking:** `Placement::name` is `Placement::model`. `Placement::name` and
  `Candidate::name` carry the name of the fixture.
- **Breaking:** `Entry`, `Placement` and `Candidate` gain `groups`. Fill the
  new field where the code builds one.

## [0.2.0] — 2026-09-23

### Added

- The white channel drives `segment` and `pixel`: a white over the whole
  device. See [`docs/dmx.md`](../../../../docs/dmx.md) 1.3.
- A white on a zone personality goes out over the armed segment channel, which
  it ends. The white channel back at 0 arms the channel and paints every zone.
- A switch between white and zones pays `arm_settle_ms` before the first paint,
  and some units show dark for that time.
- A fixture on `segment`, on a device that declares
  `capabilities.segments.groups`, opens the stream at `Resolution::Groups`. See
  [`docs/dmx.md`](../../../../docs/dmx.md) 1.2.

### Changed

- **Breaking:** channels 1 to 3 are the dimmer, the mode and the white on every
  personality. Colors and zones start at 4: repatch and rewrite the cues.
- **Breaking:** `segment` and `pixel` take 3 + 3 × zones channels. A patch that
  packs two zone fixtures edge to edge overlaps: move the second one up.

## [0.1.0] — 2026-09-21

### Added

- The crate and the `govee-dmx` binary, with the `artnet` and `sacn` cargo
  features. `artnet` is the default.
- Every command discovers over `lan` alone, the mode the bridge drives. A
  target that names a SKU selects no handle of another mode.
- Every failure reports through `govee_toolkit::exit`, so one exit code means
  the same thing here and in `govee`. The error on stderr carries a `kind`.
- `govee-dmx profile <SKU>` — the channel table of one device, with every
  channel, its offset, and the step count of each scaled channel.
- `profile --personality <name>` prints one table, and `--json` the record a
  machine reads. A personality nothing serves exits 5, an unknown name exits 2.
- `profile` and `report` re-export `govee_toolkit::profile`, so the node, the
  catalog and the site read one channel table.
- `report::personalities` — the channel tables as the JSON array alone.
  `dist/catalog.json` carries it beside each device.
- `patch` — the patch file: the node settings, and one entry per fixture in
  either spelling of the Art-Net address. An unknown key is refused.
- `Patch::resolve` — the patch joined to the devices the bridge found, as a
  `Rig`. It reports every fault at once, so a desk is corrected in one pass.
- `patch::Error` — an overlap, an address outside a universe, a fixture past
  the end of one, a device patched twice, and a personality served by nothing.
- Two entries on one port-address, start address and personality are a clone,
  which the loader allows. A part of a span shared under two tables is a fault.
- `enabled: false` on an entry keeps its channels and drives nothing. It is out
  of the overlap check, so a driven fixture can cover the channels it holds.
- `sku:` on an entry sizes it where the device does not answer, so a fixture
  that is off the network holds its channels.
- `scanned:` — the instant of the last scan, RFC 3339 in UTC, at the top of
  the patch. It states how old the rig below it is.
- Every command takes the patch file as `--patch FILE`. The default is
  `patch.yaml` beside `config.yaml`.
- `govee-dmx patch` — scan the LAN and write the rig. It adds one entry per
  device that has none, and moves no entry the file carries.
- The scan patches the devices that answered it. The device cache holds a
  device that was on the network last week, and a patch names neither.
- The scan writes `enabled:` in both directions: `false` for a device that did
  not answer, and `true` again at the scan that reaches it.
- `hold: true` on an entry keeps `enabled:` as the operator wrote it. The scan
  reports the state it read and writes none, so a live fixture stays out.
- A new entry takes the lowest free channels of the lowest universe from
  `--universe`. A second scan fills the gap a deleted entry left.
- `patch --personality` takes `full`, `segment`, `pixel`, or `widest` for the
  widest table the device serves. The default is `full`, which is 6 channels.
- `patch --dry-run` prints the entries and writes nothing. `--reset` writes the
  file from the scan alone, and keeps the one it replaces as `.yaml.bak`.
- The patch writer appends. It keeps the comments and the key order of a file
  an operator edited by hand.
- `input::UniverseFrame` — one universe of channel values, from whichever
  protocol carried it. `slot()` reads a channel at the address a desk shows.
- `input::artnet` — the `ArtDmx` parser, behind the `artnet` feature. It drops
  an odd length, a length over 512 and a protocol version under 14.
- `input::artnet::Sequence` — drops a packet older than the last one accepted,
  in a window of 256. A sender that writes 0 disables the check.
- The gate takes a packet after 4 refusals in a row and counts from it, so a
  sender that starts again drives. A refused packet names its sequence.
- `input::artnet::Gate` — one `Sequence` per sender and port-address, so the
  packets of one sender never drop the packets of another.
- `input::artnet::Poll` — the `ArtPoll` parser. A poll takes 14 bytes, under
  the 18 an `ArtDmx` header takes, so the parser reads the opcode first.
- `input::artnet::replies` — one `ArtPollReply` for each group of 4
  port-addresses the patch holds. The node answers every poll, so a desk lists
  it.
- The node broadcasts its `ArtPollReply`. A unicast reply to port 6454 reaches
  one socket, so a desk that shares the host with the node lists nothing.
- `node::Node::replies_to` names the address an `ArtPollReply` goes to. The
  default is the broadcast address.
- `input::socket::Listener` — the UDP socket the node receives on. It carries
  `SO_REUSEADDR`: port 6454 is fixed, and a second Art-Net application on the
  host must start too.
- `input::socket::Listener::local_ip_towards` — the address of the interface
  that reaches one peer. A node bound to `0.0.0.0` answers a poll with it.
- `node::Node` — the run loop: receive, drop what the protocol refuses,
  resolve against the patch, and hand each fixture its look. It reports
  through `node::Observer`.
- `node::Node::named` — the name a desk lists the node under, which `govee-dmx
  run` takes from the patch.
- `govee-dmx run` — receive Art-Net on port 6454 and drive the patched
  devices. `--config` names the configuration file, and `--json` the records.
- A device the patch names that enables no `lan` mode fails at the start and is
  named. The bridge drives over `lan` alone and substitutes no other mode.
- `run --dry-run` — print every packet and what each fixture reads out of it,
  and write to no device. It debugs a patch before any device is at risk.
- `run --scan` scans and writes the patch before it starts the node, with the
  `--personality` and `--universe` of the `patch` command.
- A run stores black on every driven fixture at start and leaves it off, so no
  color from an earlier run flashes on the first frame that raises the dimmer.
- `run --no-reset` leaves that pass out, and a dry run sends it on no device.
- `run --debug` prints a line for each answered `ArtPoll`, and raises the
  library traces to `debug`. `RUST_LOG` wins over it, and both go to stderr.
- `apply::Look` — what one frame asks of one fixture: the power, the
  brightness, the color, the white temperature and the zones, in the units the
  device file declares.
- `apply::Applier` — the send path, one task per fixture. It writes only what
  changed and sends again after the refresh interval. A device that fails stops
  no other one.
- A command path writes at 30 frames per second, and `max_hz` in the patch
  raises it. It holds the latest look and counts what it replaced in
  `frames_superseded`.
- Every write pins `lan`. A device that enables a second mode keeps it for
  other callers, and a device that stops answering over `lan` is unreachable.
- Two commands to one device keep a gap of 5 ms, inside one pass and across
  two. A device that reads a power off and a power on back to back can apply
  them in the other order.
- The refresh sends no power command to a fixture whose segment channel is
  armed. A power command ends that channel on some devices.
- A pass that carries a white temperature writes no color. A white command
  replaces the color on the device, so the color read as a blink before it.
- The node sends the color again where the white channel comes back to 0. A
  white command replaced the color, so the fixture would hold white otherwise.
- The dimmer at 0 takes every color to 0 and leaves the device on.
  `node.off_delay_secs` — how long it stays on and black before the power off.
  The default is 5 seconds, and `0` powers the device off at once.
- A dip through 0 costs one repaint: a power off and a power on that arrive
  close together are applied in the wrong order by the firmware, and a zone
  personality pays the arming delay on top.
- `node.signal_loss_secs` — how long a fixture waits for a frame before
  `on_signal_loss` decides what it shows. The default is 4 seconds.
- `apply::Look::quiet` — what a fixture shows once the sender has gone quiet:
  `hold` keeps the last look, `black` takes every color to 0 and keeps the
  device on, and `off` powers it down. `on_signal_loss: off` waits no off
  delay.
- A fixture whose device stopped answering is retried on a backoff, which
  doubles from 250 ms to 8 seconds. A write that lands clears it, and every
  other fixture keeps running.
- `govee-dmx identify` — takes every fixture off at once, then lights them one
  at a time in patch order, so a person maps an entry to a fixture in the room.
- `identify [TARGET]...`, `--universe` and `--address` — walk a part of the
  rig: the fixtures of the devices a target names, of one universe, or the
  fixture that answers to one channel of it.
- `identify --color`, `--wait-ms`, `--hold-ms` and `--keep` set what each
  fixture shows, how long each step lasts, and whether the rig stays lit.
- `identify` exits non-zero and names every fixture that refused the pass, and
  every fixture that refused the blackout at the end.
- A fixture that refused the first blackout is neither lit nor asked to go off
  again.
