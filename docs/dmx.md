# DMX bridge (`govee-dmx`)

A lighting desk, a media server or a show controller sends DMX over the
network. The bridge receives it, maps each slot to a device parameter, and
writes the result over `lan`. The devices become fixtures of the show.

This page is the design reference for `packages/rust/crates/dmx`. The channel
table of each device is in [`dmx-profiles.md`](dmx-profiles.md).

> **The bridge is an input, not a mode.** It adds no fourth transport and no
> match arm, and it reaches a device the way any other caller does. Every write
> goes over `lan` alone: where the device stops answering, the bridge reports
> it unreachable, and it never substitutes another mode — see
> [`modes.md`](modes.md).

## What you need

- A device that `lan` reaches. Where a patched device has no `lan` mode, the
  bridge fails at start and names the device.
- A desk that sends Art-Net on the network.
- A patch file that says which device answers which channels.

```sh
govee-dmx patch      # scan the LAN and write patch.yaml
govee-dmx identify   # light each fixture, in patch order
govee-dmx run        # receive DMX and drive the rig
```

| | |
| --- | --- |
| Crate | `govee-toolkit-dmx`, at `packages/rust/crates/dmx` |
| Binary | `govee-dmx` |
| Release tag | `dmx-vX.Y.Z`, versioned apart — see [`versioning.md`](versioning.md) |
| Input protocols | `artnet` (the default feature), `sacn` (planned) |

---

## 1. The channel table

### 1.1 Where it comes from

The table is **derived**. No SKU name and no per-model table exists in the
code. Three things in `devices/*.yaml` decide it:

- `capabilities:` — which parameters the hardware has, and their bounds;
- `modes.lan.capabilities` and `modes.lan.unreachable` — which of them `lan`
  reaches;
- the `role:` of each `lan` command — which of them the SDK can send.

A device with no `segments` capability has no zone personality. A device whose
`lan` mode does not reach `colortemp` keeps the white channel and drives
nothing from it.

### 1.2 Personalities

A personality is one channel layout, the way it is on any other fixture.

| Name | Channels | Condition |
| --- | --- | --- |
| `full` | 6 | `brightness` and `color` |
| `segment` | 2 + 3 × `segments.count` | and `segments` |
| `pixel` | 2 + 3 × `segments.native_pixels` | and a measured `native_pixels` |

`full` is 6 channels wide on every device:

| Offset | Slot |
| --- | --- |
| 1 | Dimmer |
| 2 | Mode |
| 3 | Red |
| 4 | Green |
| 5 | Blue |
| 6 | White temperature |

`segment` holds one RGB triple for each zone, in zone order, from offset 3.
`pixel` holds one triple for each addressable LED. A device whose every zone is
one addressable LED serves `pixel` alone: the two would lay out one table, and
one table carries one name. A device `lan` reaches no white temperature on
keeps channel 6 of `full` and drives nothing from it, and `govee-dmx profile`
names that channel `unreached`.

Channel 1 and channel 2 are the same on all three personalities, so a desk
reads one fixture the same way whatever it is patched on, and a cue file
carries between two models of different widths. The mode channel goes second
rather than last, which is where a fixture with a fixed table puts it: a table
derived from a measured zone count changes width between models and between two
lengths of one model, and a trailing channel would move with it.

The mode channel carries 0 to 9 for no action, and 250 to 255 to force a full
resend. Every other value is reserved.

A personality wider than 512 channels is an error when the patch loads. See
[8. Open questions](#8-open-questions) for a device that needs more.

### 1.3 Slot values

A DMX slot holds 0 to 255. A device parameter holds what the device file says.
The bridge scales between the two, so the operator sees a full 0 to 255 travel
on every device:

```
value = min + round((slot - 1) × (max - min) / 254)
```

**Dimmer.** Slot 0 turns the device off, which is what an operator expects and
what makes a blackout work with no patch of its own. Slot 1 to 255 turns the
device on and sets the brightness over the capability range.

**Red, green and blue.** Each component scales over the pair the `lan` color
command declares for it, with slot 0 at the bottom of the pair. A component at
0 is a color the device shows, so the channel carries every slot and has no
off. Every device file declares the whole byte today, so each slot goes out as
it is.

**White temperature.** Slot 0 sends no white command. Without this, a channel
at zero would turn a rig white at every blackout. Slot 1 to 255 scales over
`colortemp.range_kelvin`.

A white command replaces the color on the device. Two rules follow from that:

- A pass that carries a white temperature writes no color. The color would show
  for the few milliseconds before the white command, which reads as a blink on
  the way back up from a dimmer at 0.
- The node sends the color again where the white channel comes back to 0, even
  where the color channels did not move. That is what takes the device out of
  white, and the color channels at 0 take it dark.

**Quantization.** A brightness range of `[1, 100]` maps 255 slots onto 100
steps, so about 2.5 slots share one step. A slow fade on the desk looks stepped
on the device. This is the hardware resolution, and `govee-dmx profile` reports
the step count.

A scale is not a clamp: every slot has a value inside the range. The rule that
an out-of-range value is an error still holds everywhere else.

---

## 2. The patch

The patch is a separate file. The command line names it, and the default is
`patch.yaml` beside `config.yaml`. It does not live in `config.yaml`: that file
says which modes are enabled for a device and refuses an unknown key, and a rig
of 40 fixtures does not belong in it.

```yaml
scanned: 2026-09-19T20:04:11Z  # when `govee-dmx patch` last scanned
node:
  bind: 0.0.0.0
  name: govee-toolkit       # what the desk shows in its node list
  refresh_secs: 10
  signal_loss_secs: 4       # how long a fixture waits for a frame
  off_delay_secs: 5         # how long the dimmer stays at 0 before the
                            # fixture powers off
patch:
  - device: "AA:BB:CC:DD:EE:FF"
    sku: H61A0              # optional. What sizes the entry while the device
                            # is off the network
    enabled: true           # the scan writes it. `false` keeps the channels
                            # and drives nothing
    hold: true              # optional. The scan leaves `enabled` alone
    universe: 0             # or: net: 0, subnet: 0, universe: 0
    address: 1              # the DMX start address, 1 to 512
    personality: segment
    max_hz: 20              # optional. The rate the fixture takes writes at:
                            # the device file measurement for a zone
                            # personality, and 30 for a command
    on_signal_loss: hold
```

`device` is the identity `govee scan` reports. `address` is the first channel
the fixture answers to, the same number an operator sets on a real fixture, and
the personality decides how many channels follow it.

A 10-zone device on `personality: segment` and `address: 1` therefore takes
channels 1 to 32 of universe 0: channel 1 is the dimmer, channel 2 is the mode
channel, channels 3 to 5 are zone 0, channels 6 to 8 are zone 1, and so on. A
second device on the same universe starts at address 33.

**Which fixtures the scan writes.** The scan writes `enabled:`, in both
directions: a device that did not answer takes `false`, and it takes `true`
again at the scan that reaches it. The file therefore states which fixtures the
node reached, and `scanned:` states when. `hold: true` on an entry stops that:
the scan leaves the `enabled` of that entry as the operator wrote it, whatever
it finds, and reports the state it read. That is how a fixture stays out of a
rig while its device is on the network.

**A disabled entry keeps its channels.** `enabled: false` takes one fixture out
of the rig and moves no address. The bridge sends that entry nothing, and
`govee-dmx patch` gives those channels to no new entry, so the addresses the
desk carries stay where they are while one device is off. An entry that carries
`sku:` holds its channels even where the device does not answer, because the
SKU alone states the width. Delete the entry to hand its channels back.

A disabled entry is out of the overlap check. It takes no frame, so a driven
fixture can cover the channels it holds, and the operator does not have to
readdress the rig to widen one fixture over a device that is off.

**What the loader refuses.** An overlap, an address past 512, and a personality
the device cannot serve. Two entries on one port-address, one start address and
one personality are a clone: both answer to one channel table, so the desk
drives both from one set of values, and the loader allows it. Two fixtures that
share a part of a span answer to one channel under two tables, and the loader
refuses that.

---

## 3. What the node sends

The desk sends up to 44 frames per second. A device accepts far less, and the
rate falls as the frame grows. Three rules protect the fast path:

1. **Send nothing where nothing changed.** The bridge compares the new channel
   values against the last ones it sent. Equal values produce no write. A desk
   that holds a static look therefore puts no traffic on the device.
2. **Refresh every 10 seconds.** A device that received no write for 10 seconds
   receives the current values once. Nothing acknowledges a LAN frame, so a
   lost frame would otherwise leave a zone at a stale color until the next
   change. The interval is configurable. The refresh sends no power command to
   a fixture whose segment channel is armed: a power command ends that channel
   on some devices — see [`protocol/lan.md`](protocol/lan.md) 2.3. The armed
   channel is proof the device is on.
3. **The device sets the rate.** A zone personality streams, and `src/stream/`
   reads the rate from the device file and from the zone count. A command
   carries no such measurement, so the bridge writes commands at 30 frames per
   second, and `max_hz` in the patch raises that. Both paths hold the latest
   look: a later look replaces an unwritten earlier one, and
   `frames_superseded` counts what the desk sent above what the device takes.
   Report that counter: it is what tells the operator the desk sends too fast.

A fourth rule protects the device rather than the fast path: **two commands to
one device never go out back to back.** A device drops a command that arrives
directly behind two others, and nothing says so — see
[`protocol/lan.md`](protocol/lan.md) 1, "Consecutive commands". One look can
carry a power, a brightness and a color, so the bridge waits a few milliseconds
between them. A look that changes one slot writes once and waits not at all, so
a color chase pays nothing.

One task drives one fixture, so a slow device holds up no other one and no
write holds up the socket. A look the task did not take before the next one
arrived is counted with the frames the stream superseded.

A zone personality arms the segment channel once the device powers on, and
disarms it when the fixture powers off. The channel holds the colors only while
it is armed, and arming a dark strip paints nothing, so the order is fixed.

### 3.1 The dimmer at 0

A dimmer that reaches 0 takes every color to 0 and **leaves the device on**.
The power off follows `node.off_delay_secs` later, and only while the dimmer
stays at 0. The default is 5 seconds. `0` powers the device off on the pass
that reads the 0.

The wait is what makes a dip through 0 work. A power off and a power on that
arrive close together are applied in the wrong order by the firmware, and the
fixture then stays dark with no error — see
[`protocol/lan.md`](protocol/lan.md) 1. A zone personality pays more: the power
off disarms the segment channel, and the channel needs time to arm again. A
dimmer that comes back up before the delay is over costs one repaint and no
power command.

A device that is off draws no power and shows nothing. A device that is on and
black still glows on some units, which is why the power off happens at all.

`on_signal_loss: off` waits no off delay: a sender that went away is not a
dimmer that dipped, so the rig goes off at `node.signal_loss_secs`.

### 3.2 Signal loss

The bridge holds the last look. A fixture that receives no frame for
`node.signal_loss_secs` applies what `on_signal_loss` asks for instead. The
default is 4 seconds, which is what Art-Net calls a sender lost:

| Value | Result |
| --- | --- |
| `hold` | The device keeps the last values. The default. |
| `black` | Every color goes to 0. The device stays on. |
| `off` | The device powers off. |

The answer is applied once, and the next frame arms the wait again. A fixture
that has taken no frame yet holds: a rig waits dark for the desk rather than
powering off at start.

`black` sends no white command. The color it sends is what the device shows,
and a white command would light the rig at a blackout.

### 3.3 A device that fails

A show does not stop because one fixture drops. Where a device becomes
unreachable, the bridge logs the failure, keeps every other device running, and
retries with a backoff. It reports the failure rather than hiding it.

The wait doubles from 250 ms to 8 seconds, and a write that lands clears it. A
desk sends up to 44 frames per second, and one retry per frame would put 44
failures per second on the link and 44 lines in front of the operator. The
fixture keeps taking looks while it waits, so the attempt that follows carries
the current one and not a stale one. A stream whose device stopped answering is
dropped rather than disarmed: a disarming frame has nothing to reach.

---

## 4. The commands

Every command reads `patch.yaml` beside `config.yaml`, and `--patch FILE`
names another file.

### 4.1 `govee-dmx patch`

The command scans the LAN and writes the rig. It patches the devices that
answered the scan, and not the devices the cache holds: a fixture in the file
is one the node reached. It adds one entry for each device that has none, on
the lowest free channels of the lowest universe from `--universe`, and it moves
no entry the file already carries, because an address is patched on the desk
too. A device the file already names is left alone, enabled or not, so a
fixture an operator disabled does not come back at the next scan.

- `--personality` takes `full`, `segment`, `pixel`, or `widest` for the widest
  table the device serves. The default is `full`.
- `--dry-run` prints what the scan would do, and writes nothing.
- `--reset` writes the file from the scan alone. Every address in it is set
  again, and the file it replaces is kept as `.yaml.bak`.

The writer appends. It keeps the comments and the key order of a file an
operator edited by hand.

### 4.2 `govee-dmx run`

The command receives DMX on the Art-Net port and drives the patched devices.
`--scan` does the scan of `patch` first, and then starts the node from the file
it wrote, so one command takes a rig from nothing to a node a desk can patch.

`--dry-run` prints every packet it receives and what each fixture reads out of
it, and writes to no device: that is what debugs a patch before any device is
at risk.

The run stores black on every driven fixture at start, and leaves the fixture
off. A device shows the color that it held when it next comes on: the first
frame that raises the dimmer powers the device on before the color of the frame
reaches it, so a color from an earlier run flashes there. The pass takes 3
commands per fixture, it runs once, and `--no-reset` leaves it out. A dry run
writes to no device, so it sends the pass on no device either. A fixture that
refuses the pass stops no other one, and the node reports it again on the first
frame.

`--debug` prints a line for each answered `ArtPoll`, with the desk that polled
and the count of replies. A desk polls every few seconds, so a live run leaves
the lines out: an operator who cannot find the node in a desk's node list turns
the flag on to see whether the node answered.

`--debug` also raises the library traces to `debug`. A live run carries `warn`,
which is what reports a segment frame the transport refused and a stream that
stopped: nothing acknowledges a LAN frame, so those two report through the
traces and nowhere else. `RUST_LOG` wins over the flag, and every trace goes to
stderr, so `--json` keeps stdout for its records.

The node prints nothing itself. It reports each packet, each resolved look and
each failed write to the binary, which decides what a person reads.

### 4.3 `govee-dmx identify [TARGET...]`

The command names each enabled entry and lights the fixture it drives, so an
operator maps a line of the patch to a fixture in the room. Every fixture goes
off at once first, and the walk then lights one fixture at a time, in the order
the patch lists them. The terminal prints the entry before its fixture lights.

A command line that names nothing walks the whole rig. A `TARGET` names the
device of a fixture, in the grammar `govee identify` reads: an identity, a
SKU, or `name:<name>`. `--universe` and `--address` name fixtures
by the channels they answer to instead:

```sh
govee-dmx identify --address 33        # the fixture on channel 33 of universe 0
govee-dmx identify --universe 1        # every fixture of universe 1
govee-dmx identify H6008 name:kitchen  # by model, and by the name in config.yaml
```

- `--universe` is the port-address to light. With `--address`, the universe
  that channel sits on. The default is 0.
- `--address` is a DMX channel, 1 to 512. It lights the fixture that answers to
  that channel, and not only the one that starts there.
- `--color` is the color each fixture shows. The default is `#00ff00`.
- `--wait-ms` is the interval between two steps: after the rig goes off, and
  after each fixture lights. The default is 1000.
- `--hold-ms` is how long the last fixture holds the color before every fixture
  goes off. The default is 5000.
- `--keep` leaves every fixture lit, and sends no power command at the end.

The blackout covers the whole rig, whatever the walk lights afterwards: one
lit fixture in a dark room is what the operator reads. A target and an address
name fixtures separately, and the walk lights every fixture either one names,
in patch order. A target that the patch drives no fixture for, and an address
that no driven fixture answers to, fail the command: an operator who types one
means to see it light.

The walk drives the devices over `lan`, the way a run does, and it drives a
device that `config.yaml` enables `lan` for. A fixture that refuses the first
blackout leaves the walk there. The walk does not light it, and does not ask
it to go off at the end. A fixture that fails later stops no other one: the
walk prints the failure, carries on, and exits non-zero. The look each
fixture held is lost — the walk reads no state back first.

### 4.4 `govee-dmx profile <SKU>`

The command prints the channel table, so the operator can patch the desk. It
also prints the step count for each scaled channel.

`cargo run -p xtask -- dmx` writes [`dmx-profiles.md`](dmx-profiles.md) from
the device files. CI fails on drift, the same way it does for
[`compatibility.md`](compatibility.md). The table also reaches
`dist/catalog.json` and the devices page of the site. Each channel there
carries the bands of its slots: the pair of slots, and what the device does
over it. The page and the node read one description.

---

## 5. The input protocols

### 5.1 Art-Net

The node listens on UDP port 6454, and accepts a broadcast frame and a unicast
frame.

**Addressing.** An Art-Net address is a 15-bit port-address: a 7-bit Net, a
4-bit Sub-Net and a 4-bit Universe. The bridge holds one `u16` and accepts both
spellings in the patch, because a desk shows one or the other.

**ArtPoll and ArtPollReply.** The node answers every poll. A desk that receives
no reply does not list the node, and the operator cannot patch it. One reply
carries 4 output ports, and the 4 share a Net and a Sub-Net, so the node sends
one reply for each group of 4 universes in the patch. A rig on no universe
still answers once.

The reply carries the node name from the patch, and the address a desk must
send `ArtDmx` to. A node bound to `0.0.0.0` answers with the interface that
reaches the desk, because `0.0.0.0` is an address nothing can send to. The
replies are broadcast, which Art-Net asks for. A unicast reply to port 6454
reaches one socket alone, so a desk that shares the host with the node lists
nothing: the node receives its own reply instead.

**Sequence.** The `ArtDmx` sequence field detects a packet that UDP delivered
out of order. A value of 0 disables it. Where it is non-zero, the bridge drops
a packet that is older than the last one it accepted, in a window of 256. After
4 refusals in a row the bridge takes the packet and counts from it. A sender
that starts again, or a burst the socket lost, moves the count half the range
forward, and every packet that follows reads as older: without the resync the
bridge refuses that sender for good and the rig goes dark.

**Merge.** Art-Net asks for an HTP merge of at most 2 sources on one universe.
The first version takes the last source instead, and logs a warning that names
both addresses. HTP comes when somebody has the case.

**Refusals.** The bridge drops a packet with an odd length, a length above 512
or a protocol version below 14. It never truncates a packet to make it fit.

### 5.2 sACN

Planned. E1.31 carries a 16-bit universe, a per-source sequence and a priority
byte, and it takes the same channel model. One input protocol is one cargo
feature.

---

## 6. Inside the crate

```
crates/dmx/
  src/input/     the protocols: one socket, one parser, one universe frame
  src/profile/   capabilities -> channel table. No I/O.
  src/patch/     the patch file
  src/apply/     channel values -> device commands
  src/node/      the run loop that joins the four
  src/main.rs
```

`src/profile/` does no I/O on purpose. The site and the catalog need the same
table, so that module can move into `packages/rust` unchanged.

Every input protocol produces one type:

```rust
struct UniverseFrame {
    universe: u16,          // Art-Net port-address, or sACN universe
    slots: [u8; 512],
    len: u16,
    source: SocketAddr,
    priority: Option<u8>,   // sACN carries one; Art-Net does not
}
```

The `u16` universe holds an Art-Net port-address and an sACN universe number,
and `priority` holds what sACN adds. A second protocol therefore adds a module
and no new type.

---

## 7. Tests

A conformance vector under `tests/fixtures/golden/` covers one mode, and DMX is
not a mode. Art-Net captures go to `tests/fixtures/artnet/` instead, and the
tests read a packet and check the channel table and the arguments it produces.

The profile tests run over the whole catalog: every device whose `lan` mode
carries `segments` must produce a valid zone personality.

`crates/sim` carries an end-to-end test with no hardware.

## 8. Open questions

- **A device wider than 512 channels.** A long strip at native resolution needs
  more than one universe. The candidate answer is a spill into the next
  universe, in order. Nothing decides it yet.
- **Gradient.** A `segment_color` command can carry a `role: gradient`
  argument. The mode channel can hold it, or the patch can.
- **HTP merge.** See **Merge** in 5.1.
