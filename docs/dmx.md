# DMX

An Art-Net node that drives devices over `lan`. This page is the design
reference for `packages/rust/crates/dmx`. Nothing here ships yet — see
[`dmx-plan.md`](dmx-plan.md) for the order of the work, and
[`roadmap.md`](roadmap.md) for where it sits among the milestones.

## What it is

A lighting desk, a media server or a show controller sends DMX over the
network. The bridge receives it, maps each slot to a device parameter, and
writes the result to the device over the LAN segment channel.

The bridge is an **input**, not a mode. It does not add a fourth transport, it
does not add a match arm, and it reaches a device the same way any other caller
does. A device that the bridge drives must have `lan` enabled. Where the device
has no `lan` mode, the bridge fails at start and names the device.

The bridge pins `lan` on every write. A device can enable more than one mode,
and the bridge uses none of the others: where the device stops answering over
`lan`, the bridge reports the device unreachable. It never substitutes another
mode — see [`modes.md`](modes.md).

## Package

| | |
| --- | --- |
| Crate | `govee-toolkit-dmx`, at `packages/rust/crates/dmx` |
| Binary | `govee-dmx` |
| Release tag | `dmx-vX.Y.Z`, versioned apart — see [`versioning.md`](versioning.md) |
| Features | `artnet` (default), `sacn` (planned) |

Art-Net is one way in. sACN is the second one people ask for, and it carries the
same channel model. The package name says what the package does, and a cargo
feature says which protocol carries it.

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

## The input layer

Every protocol produces one type:

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

### Art-Net

The node listens on UDP port 6454 and accepts a broadcast frame and a unicast
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
replies go to the desk that polled rather than to the broadcast address: a poll
names its sender, and one rig then reaches no other application on the
network.

**Sequence.** The ArtDmx sequence field detects a packet that UDP delivered out
of order. A value of 0 disables it. Where it is non-zero, the bridge drops a
packet that is older than the last one it accepted, in a window of 256.

**Merge.** Art-Net asks for an HTP merge of at most 2 sources on one universe.
The first version takes the last source instead, and logs a warning that names
both addresses. HTP comes when somebody has the case.

**Refusals.** The bridge drops a packet with an odd length, a length above 512
or a protocol version below 14. It never truncates a packet to make it fit.

### sACN

Planned. E1.31 carries a 16-bit universe, a per-source sequence and a priority
byte. The `UniverseFrame` above already holds all three.

## The channel table

The table is **derived**. No SKU name and no per-model table exists in the
code. Three things in `devices/*.yaml` decide it:

- `capabilities:` — which parameters the hardware has, and their bounds;
- `modes.lan.capabilities` and `modes.lan.unreachable` — which of them `lan`
  reaches;
- the `role:` of each `lan` command — which of them the SDK can send.

A device with no `segments` capability has no zone personality. A device whose
`lan` mode does not reach `colortemp` keeps the white channel and drives
nothing from it.

### Personalities

| Name | Channels | Condition |
| --- | --- | --- |
| `full` | 6 | `brightness` and `color` |
| `segment` | 2 + 3 × `segments.count` | and `segments` |
| `pixel` | 2 + 3 × `segments.native_pixels` | and a measured `native_pixels` |

Channel 1 and channel 2 are the same on every personality:

| Offset | Slot |
| --- | --- |
| 1 | Dimmer |
| 2 | Mode |

A desk therefore reads one fixture the same way whatever personality it is
patched on, and a cue file carries between two models of different widths. The
mode channel goes second rather than last, which is where a fixture with a
fixed table puts it: a table derived from a measured zone count changes width
between models and between two lengths of one model, and a trailing channel
would move with it.

`full` adds:

| Offset | Slot |
| --- | --- |
| 3 | Red |
| 4 | Green |
| 5 | Blue |
| 6 | White temperature |

A device `lan` reaches no white temperature on keeps channel 6 and drives
nothing from it. `full` is 6 channels wide on every device, and
`govee-dmx profile` names the channel `unreached`.

`segment` holds one RGB triple for each zone, in zone order, from offset 3.
`pixel` holds one triple for each addressable LED. A device whose every zone is
one addressable LED serves `pixel` alone: the two would lay out one table, and
one table carries one name.

The mode channel carries 0 to 9 for no action and 250 to 255 to force a full
resend. Every other value is reserved.

### Slot values

A DMX slot holds 0 to 255. A device parameter holds what the device file says.
The bridge scales between the two, and the operator sees a full 0 to 255 travel
on every device.

**Dimmer.** Slot 0 turns the device off. This is what a lighting desk operator
expects, and it makes a blackout work with no patch of its own. Slot 1 to 255
turns the device on and sets the brightness over the capability range:

```
value = min + round((slot - 1) × (max - min) / 254)
```

**Red, green and blue.** Each component scales over the pair the `lan` color
command declares for it, with slot 0 at the bottom of the pair. A component at
0 is a color the device shows, so the channel carries every slot and has no
off. Every device file declares the whole byte today, so each slot goes out as
it is.

**White temperature.** Slot 0 sends no white command, so the color stays. Slot 1
to 255 scales over `colortemp.range_kelvin` with the formula above. Without
this, a channel at zero would turn a rig white at every blackout.

**Quantization.** A brightness range of `[1, 100]` maps 255 slots onto 100
steps, so about 2.5 slots share one step. A slow fade on the desk looks
stepped on the device. This is the hardware resolution, and the bridge reports
the step count in `govee-dmx profile`.

A scale is not a clamp: every slot has a value inside the range. The rule that
an out-of-range value is an error still holds everywhere else.

### Width

A personality wider than 512 channels is an error when the patch loads. See
**Open questions** for a device that needs more.

## The send policy

The desk sends up to 44 frames per second. A device accepts far less, and the
rate falls as the frame grows. Three rules protect the fast path:

1. **Send nothing where nothing changed.** The bridge compares the new channel
   values against the last ones it sent. Equal values produce no write. A desk
   that holds a static look therefore puts no traffic on the device.
2. **Refresh every 10 seconds.** A device that received no write for 10 seconds
   receives the current values once. Nothing acknowledges a LAN frame, so a lost
   frame would otherwise leave a zone at a stale color until the next change.
   The interval is configurable. The refresh sends no power command to a fixture
   whose segment channel is armed: a power command ends that channel on some
   devices — see [`protocol/lan.md`](protocol/lan.md) 2.3. The armed channel is
   proof the device is on.
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
disarms it when the dimmer returns to 0. The channel holds the colors only
while it is armed, and arming a dark strip paints nothing, so the order is
fixed.

### Signal loss

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

### A device that fails

A show does not stop because one fixture drops. Where a device becomes
unreachable, the bridge logs the failure, keeps every other device running, and
retries with a backoff. It reports the failure rather than hiding it.

The wait doubles from 250 ms to 8 seconds, and a write that lands clears it. A
desk sends up to 44 frames per second, and one retry per frame would put 44
failures per second on the link and 44 lines in front of the operator. The
fixture keeps taking looks while it waits, so the attempt that follows carries
the current one and not a stale one. A stream whose device stopped answering is
dropped rather than disarmed: a disarming frame has nothing to reach.

## The patch

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
the fixture answers to, the same number the operator sets on a real fixture.
The personality decides how many channels follow it.

`enabled: false` takes one fixture out of the rig and moves no address. The
bridge sends that entry nothing, and it gives those channels to no other
fixture: the addresses the desk carries stay where they are while one device is
off. An entry that carries `sku:` holds its channels even where the device does
not answer, because the SKU alone states the width. Delete the entry to hand
its channels back.

The scan writes `enabled:`, in both directions. A device that did not answer
takes `false`, and it takes `true` again at the scan that reaches it. The file
therefore states which fixtures the node reached, and `scanned:` states when.
`hold: true` on an entry is what stops that: the scan leaves the `enabled` of
that entry as the operator wrote it, whatever it finds, and reports the state it
read. That is how a fixture stays out of a rig while its device is on the
network.

A patch entry for a 10-zone device with `personality: segment` and
`address: 1` therefore takes channels 1 to 32 of universe 0: channel 1 is the
dimmer, channel 2 is the mode channel, channels 3 to 5 are zone 0, channels 6
to 8 are zone 1, and so on. A second device on the same universe starts at
address 33.

The patch loader refuses an overlap, an address past 512 and a personality the
device cannot serve.

## Output

`govee-dmx patch [FILE]` scans the LAN and writes the rig. It patches the
devices that answered the scan, and not the devices the cache holds: a fixture
in the file is one the node reached. It adds one entry for each device that has
none, on the lowest free channels of the lowest universe from `--universe`, and
it moves no entry the file already carries: an address is patched on the desk
too. A device the file already names is left
alone, enabled or not, so a fixture an operator disabled does not come back at
the next scan.

- `--personality` takes `full`, `segment`, `pixel`, or `widest` for the widest
  table the device serves. The default is `full`, which is 6 channels per
  fixture.
- `--dry-run` prints what the scan would do, and writes nothing.
- `--reset` writes the file from the scan alone. Every address in it is set
  again, and the file it replaces is kept as `.yaml.bak`.

The writer appends. It keeps the comments and the key order of a file an
operator edited by hand.

`govee-dmx run [FILE] --scan` does the same scan first, and then starts the
node from the file it wrote. One command therefore takes a rig from nothing to
a node a desk can patch.

`govee-dmx profile <SKU>` prints the channel table, so the operator can patch
the desk. It also prints the step count for each scaled channel.

`cargo run -p xtask -- dmx` writes `docs/dmx-profiles.md` from the device
files. CI fails on drift, the same way it does for
[`compatibility.md`](compatibility.md).

The table also reaches `dist/catalog.json` and the devices page of the site.

`govee-dmx run <patch>` receives Art-Net on port 6454 and drives the patched
devices. `--dry-run` prints every packet it receives and what each fixture
reads out of it, and writes to no device: that is what debugs a patch before
any device is at risk.

A live run prints a line for each answered `ArtPoll`, with the desk that polled
and the count of replies. An operator who cannot find the node in a desk's node
list needs that line to see whether the node answered.

The node prints nothing itself. It reports each packet, each resolved look and
each failed write to the binary, which decides what a person reads.

## Tests

A conformance vector under `tests/fixtures/golden/` covers one mode, and DMX is
not a mode. Art-Net captures go to `tests/fixtures/artnet/` instead, and the
tests read a packet and check the channel table and the arguments it produces.

The profile tests run over the whole catalog: every device whose `lan` mode
carries `segments` must produce a valid zone personality.

`crates/sim` carries an end-to-end test with no hardware.

## Open questions

- **A device wider than 512 channels.** A long strip at native resolution needs
  more than one universe. The candidate answer is a spill into the next
  universe, in order. Nothing decides it yet.
- **Gradient.** A `segment_color` command can carry a `role: gradient`
  argument. The mode channel can hold it, or the patch can.
- **HTP merge.** See **Merge** above.
