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
has no `lan` mode, the bridge fails at start and names the device. It never
substitutes another mode — see [`modes.md`](modes.md).

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

A device with no `segments` capability has no pixel personality. A device whose
`lan` mode does not reach `colortemp` has no CCT channel.

### Personalities

| Name | Channels | Condition |
| --- | --- | --- |
| `basic` | 4 | `brightness` and `color` |
| `full` | 6 | and `colortemp` |
| `pixel` | 1 + 3 × `segments.count` | and `segments` |
| `pixel-native` | 1 + 3 × `segments.native_pixels` | and a measured `native_pixels` |

`basic`:

| Offset | Slot |
| --- | --- |
| 1 | Dimmer |
| 2 | Red |
| 3 | Green |
| 4 | Blue |

`full` adds:

| Offset | Slot |
| --- | --- |
| 5 | White temperature |
| 6 | Control |

`pixel` holds the dimmer at offset 1, then one RGB triple for each zone, in
zone order. `pixel-native` holds one triple for each addressable LED.

The control channel carries 0 to 9 for no action and 250 to 255 to force a full
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

**Red, green and blue.** Sent as they are. The wire takes 0 to 255.

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
   The interval is configurable.
3. **The device sets the rate.** `src/stream/` reads the rate from the device
   file and from the zone count, and holds the latest frame. A later frame
   replaces an unsent earlier one, and `frames_superseded` counts what the desk
   sent above what the device accepts. Report that counter: it is what tells the
   operator the desk sends too fast.

One task drives one fixture, so a slow device holds up no other one and no
write holds up the socket. A look the task did not take before the next one
arrived is counted with the frames the stream superseded.

A pixel personality arms the segment channel once the device powers on, and
disarms it when the dimmer returns to 0. The channel holds the colors only
while it is armed, and arming a dark strip paints nothing, so the order is
fixed.

### Signal loss

The bridge holds the last look. Where the patch sets `on_signal_loss`, it
applies that instead after the timeout:

| Value | Result |
| --- | --- |
| `hold` | The device keeps the last values. The default. |
| `black` | Every color goes to 0. The device stays on. |
| `off` | The device powers off. |

### A device that fails

A show does not stop because one fixture drops. Where a device becomes
unreachable, the bridge logs the failure, keeps every other device running, and
retries the stream with a backoff. It reports the failure rather than hiding
it.

## The patch

The patch is a separate file, named on the command line. It does not live in
`config.yaml`: that file says which modes are enabled for a device and refuses
an unknown key, and a rig of 40 fixtures does not belong in it.

```yaml
node:
  bind: 0.0.0.0
  name: govee-toolkit       # what the desk shows in its node list
  refresh_secs: 10
patch:
  - device: "AA:BB:CC:DD:EE:FF"
    universe: 0             # or: net: 0, subnet: 0, universe: 0
    address: 1              # the DMX start address, 1 to 512
    personality: pixel
    max_hz: 20              # optional. Default: the device file measurement
    on_signal_loss: hold
```

`device` is the identity `govee scan` reports. `address` is the first channel
the fixture answers to, the same number the operator sets on a real fixture.
The personality decides how many channels follow it.

A patch entry for a 10-zone device with `personality: pixel` and `address: 1`
therefore takes channels 1 to 31 of universe 0: channel 1 is the dimmer,
channels 2 to 4 are zone 0, channels 5 to 7 are zone 1, and so on. A second
device on the same universe starts at address 32.

The patch loader refuses an overlap, an address past 512 and a personality the
device cannot serve.

## Output

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

The node prints nothing itself. It reports each packet, each resolved look and
each failed write to the binary, which decides what a person reads.

## Tests

A conformance vector under `tests/fixtures/golden/` covers one mode, and DMX is
not a mode. Art-Net captures go to `tests/fixtures/artnet/` instead, and the
tests read a packet and check the channel table and the arguments it produces.

The profile tests run over the whole catalog: every device whose `lan` mode
carries `segments` must produce a valid pixel personality.

`crates/sim` carries an end-to-end test with no hardware.

## Open questions

- **A device wider than 512 channels.** A long strip at native resolution needs
  more than one universe. The candidate answer is a spill into the next
  universe, in order. Nothing decides it yet.
- **Gradient.** A `segment_color` command can carry a `role: gradient`
  argument. The control channel can hold it, or the patch can.
- **HTP merge.** See **Merge** above.
