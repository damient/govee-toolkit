# Changelog

Changes to `govee-toolkit-dmx`, the crate that builds the `govee-dmx` binary
from `packages/rust/crates/dmx`. It versions apart from `govee-toolkit` and
releases under `dmx-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md). The design
is [`../../../../docs/dmx.md`](../../../../docs/dmx.md).

Nothing is published yet: the manifest carries `publish = false`.

### Added

- The Art-Net parser reads recorded packets under `tests/fixtures/artnet/`: an
  `ArtDmx` and an `ArtPoll` from QLC+, and an `ArtDmx` from TouchDesigner. A
  capture that lists no channel value fails the test, because it proves
  nothing.
- `identify` — names each enabled entry and lights the fixture it drives. Every
  fixture goes off at once first, and the walk then lights one at a time, in
  patch order. `--color`, `--wait-ms`, `--hold-ms` and `--keep` set what it
  shows and how long each step lasts.
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
- Two commands to one device never go out back to back. A device drops a
  command that arrives directly behind two others, and nothing reports it.
- The refresh sends no power command to a fixture whose segment channel is
  armed. A power command ends that channel on some devices.
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
- A command path writes at 30 frames per second, and `max_hz` in the patch
  raises it. The bridge holds the latest look and counts what it replaced in
  `frames_superseded`, the way the zone stream does.
- Every write pins `lan`. A device that enables a second mode keeps it for
  other callers, and a device that stops answering over `lan` is reported
  unreachable.
- A pass that carries a white temperature writes no color. A white command
  replaces the color on the device, so the color showed for the few
  milliseconds before it, which read as a blink on the way back up from a
  dimmer at 0.
- Two commands to one device keep a gap of 5 ms, across two passes as well as
  inside one. A device that reads a power off and a power on back to back can
  apply them in the other order.
- The node sends the color again where the white channel comes back to 0, even
  where the color channels did not move. A white command replaces the color on
  the device, so the fixture stayed white otherwise and the desk had no way to
  take it out. The color channels at 0 take the fixture dark.
- A run stores black on every driven fixture at start, and leaves the fixture
  off. A device shows the color that it held when it next comes on, so a color
  from an earlier run would flash on the first frame that raises the dimmer.
  `--no-reset` leaves the pass out, and a dry run sends it on no device.
- `run --debug` prints a line for each answered `ArtPoll`: the desk that
  polled, and the count of replies. A desk polls every few seconds, so a run
  leaves the lines out without the flag.
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

- `govee-dmx patch [FILE]` — scan the LAN and write the rig. It adds one entry
  per device that has none, and moves no entry the file carries.
- The scan patches the devices that answered it. The device cache holds a
  device that was on the network last week, and a patch names neither.
- A new entry takes the lowest free channels of the lowest universe from
  `--universe`. A second scan fills the gap a deleted entry left.
- `patch --personality` takes `full`, `segment`, `pixel`, or `widest` for the
  widest table the device serves. The default is `full`, which is 6 channels.
- `patch --dry-run` prints the entries and writes nothing. `--reset` writes the
  file from the scan alone, and keeps the one it replaces as `.yaml.bak`.
- `govee-dmx run --scan` scans and writes the patch before it starts the node,
  with the `--personality` and `--universe` of the `patch` command.
- The patch file is optional on both commands. The default is `patch.yaml`
  beside `config.yaml`.
- `enabled: false` on a patch entry keeps its channels and drives nothing. The
  addresses of every other fixture stay where the desk has them. The entry is
  out of the overlap check: it takes no frame, so a driven fixture can cover
  the channels it holds.
- The dimmer at 0 takes every color to 0 and leaves the device on. The power
  off follows `node.off_delay_secs` later, and only while the dimmer stays at
  0. The default is 5 seconds, and `0` powers the device off at once. A dip
  through 0 then costs one repaint: a power off and a power on that arrive
  close together are applied in the wrong order by the firmware, and a zone
  personality pays the arming delay on top. `on_signal_loss: off` waits no off
  delay.
- `node.off_delay_secs` — how long a fixture stays on and black before the
  dimmer at 0 powers it off.
- `govee-dmx run --debug` installs a `tracing` subscriber. A live run carries
  `warn`, which reports a segment frame the transport refused and a stream that
  stopped; `--debug` raises the library to `debug`. `RUST_LOG` wins over both,
  and the traces go to stderr.
- Two entries on one port-address, one start address and one personality are a
  clone. They answer to one channel table, the desk drives both from one set of
  values, and the loader allows it. A part of a span shared under two tables
  stays a fault.
- The scan writes `enabled:`, in both directions: `false` for a device that did
  not answer, and `true` again at the scan that reaches it.
- `hold: true` on a patch entry keeps `enabled:` as the operator wrote it. The
  scan reports the state it read and writes none, so a live fixture stays out.
- `scanned:` — the instant of the last scan, RFC 3339 in UTC, at the top of the
  patch. It states how old the rig below it is.
- `sku:` on a patch entry sizes it where the device does not answer, so a
  fixture that is off the network holds its channels.
- The patch writer appends. It keeps the comments and the key order of a file
  an operator edited by hand.

### Changed

- `apply::Applier::start` and `node::Node::live` take an `apply::Timing`
  rather than the refresh interval alone.
- `Patch::resolve` takes a second lookup, which answers a device file by SKU.
  It sizes a disabled entry whose device did not answer.
