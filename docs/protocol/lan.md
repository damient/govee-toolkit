# Govee LAN protocol (`lan` mode)

`lan` is the default mode — see [`../modes.md`](../modes.md). Lowest latency,
full capability coverage, and it never leaves the local network.

Two parts: the LAN protocol documented by Govee, and the undocumented commands
found through reverse engineering.

> **No authentication, no encryption.** Control frames carry no key and no
> signature, payloads are plaintext, and a discovery reply can be sent by any
> host on the segment claiming any identity. Anyone on the same layer-2 network
> can control the devices. The trust boundary is the network, and the answer is
> to segment it — [`../security.md`](../security.md).

The official part below follows Govee's WLAN guide
(<https://app-h5.govee.com/user-manual/wlan-guide>, retrieved 2026-09-04). The
models that expose the LAN switch are listed in
[`../lan-supported-devices.md`](../lan-supported-devices.md).

---

## 1. Official part

### Prerequisites

"LAN Control" must be enabled for the device in the Govee Home app: add the
device, make sure its Wi-Fi is connected, then turn on the LAN switch in the
device settings. Not every SKU exposes the switch — see
[`../lan-supported-devices.md`](../lan-supported-devices.md).

### Ports and addresses

| Role | Address | Port | Direction |
| ---- | ------- | ---- | --------- |
| Discovery (request) | `239.255.255.250` (multicast) | `4001` | client → device |
| Discovery (response) | unicast back to the client | `4002` | device → client |
| Control / status | device IP | `4003` | client → device |

Transport: UDP, UTF-8 JSON payloads.

### Message envelope

```json
{ "msg": { "cmd": "<command>", "data": { } } }
```

### Discovery

Request sent by multicast to `239.255.255.250:4001`:

```json
{ "msg": { "cmd": "scan", "data": { "account_topic": "reserve" } } }
```

Response received on port `4002`:

```json
{
  "msg": {
    "cmd": "scan",
    "data": {
      "ip": "192.0.2.10",
      "device": "<MAC>",
      "sku": "Hxxxx",
      "bleVersionHard": "",
      "bleVersionSoft": "",
      "wifiVersionHard": "",
      "wifiVersionSoft": ""
    }
  }
}
```

### Documented commands (port 4003)

| Command | `cmd` | `data` |
| ------- | ----- | ------ |
| Power | `turn` | `{ "value": 0 \| 1 }` |
| Brightness | `brightness` | `{ "value": 1-100 }` (percent, integer) |
| Color / color temperature | `colorwc` | `{ "color": { "r": 0-255, "g": 0-255, "b": 0-255 }, "colorTemInKelvin": 0 or 2000-9000 }` |
| Status request | `devStatus` | `{}` |

For `colorwc`, a non-zero `colorTemInKelvin` wins: the device converts the
temperature to RGB itself and ignores `color`. Set it to `0` to apply the `r` /
`g` / `b` values.

The `devStatus` reply arrives on port `4002`:

```json
{
  "msg": {
    "cmd": "devStatus",
    "data": {
      "onOff": 1,
      "brightness": 100,
      "color": { "r": 255, "g": 0, "b": 0 },
      "colorTemInKelvin": 7200
    }
  }
}
```

### Latency notes

- Reuse one UDP socket per device (or a shared one) — never recreate it per
  send.
- Do not re-run a multicast scan before each command: scan at startup plus a
  periodic background refresh, with a persistent on-disk cache.
- Fire-and-verify: send without waiting for an ACK, verify state asynchronously.

Measuring a device's headroom: see [2.7](#27-throughput).

---

## 2. Undocumented commands

None of the following appears in Govee's guide. It is protocol, not device data:
what a given model actually accepts, how many zones it exposes and how fast it
can be driven belong in `devices/<SKU>.yaml`, never here.

Support varies by model. Treat every section below as "the protocol works this
way where it is implemented", and record per SKU whether it is.

### 2.1 Behaviors that apply to every command

**Out-of-range values are clamped in silence.** The firmware never rejects and
never answers with an error:

| Sent | Applied |
| ---- | ------- |
| `brightness` below the minimum | the minimum — `0` does **not** turn a device off |
| `brightness` above 100 | 100 |
| a negative `brightness` | ignored, previous value kept |
| `colorTemInKelvin` below range | the low bound |
| `colorTemInKelvin` above range | the high bound |

**Color and white are mutually exclusive.** Setting `colorTemInKelvin > 0`
resets `color` to `{0,0,0}` in the status; setting an RGB color resets
`colorTemInKelvin` to `0`. `devStatus` therefore also reports which of the two
modes a device is in.

**An unknown command is ignored in silence** — no error, no ACK. So is a
malformed frame on the raw channel of 2.3. A failed probe is indistinguishable
from an unimplemented feature: assume a malformed request before concluding the
device does not support something.

### 2.2 `status` — an undocumented fifth command

Distinct from `devStatus`, and the only extra command name known to answer:

```json
{"msg":{"cmd":"status","data":{"onOff":1,"brightness":75,"pt":"<base64>"}}}
```

The `pt` field decodes to a frame of the raw dialect described in 2.3, checksum
included. Its value is static — unchanged by power, color, temperature or
brightness — so it reads as a capability descriptor rather than a data channel,
and writing it back has no observable effect.

It is still useful while probing writes: a machine-readable observation
channel, instead of watching the device.

### 2.3 Per-segment color — the `razer` raw channel

The command is **`razer`**, carrying a base64 frame in `data.pt`. Not `ptReal`,
`pt` or `ptIotOp` — those belong to the cloud channel (2.5).

The dialect is **variable-length and prefixed `0xBB`**, distinct from the
20-byte `0x33` frames used over BLE. Frames do not port between the two.

```
BB <len_hi> <len_lo> <opcode> <payload…> <XOR of every preceding byte>
```

`len` is the length of the **payload alone**, 16-bit, header and checksum
excluded. An inconsistent length gets the frame dropped silently.

| opcode | Role | Payload |
| ------ | ---- | ------- |
| `0xB1` | Enable the segment channel | `[0\|1]` |
| `0xB0` | Per-segment colors | `[gradient, nbSeg, R,G,B × nbSeg]` |
| `0xB4` | Zoned variant, used by some models | `[gradient, nbSeg, (R,G,B,zone) × nbSeg]` |
| `0xB2` | Appears in the `status` reply, role unknown | `[0]` |

Sequence: `turn(1)` → `0xB1` with `1` to arm → stream of `0xB0`.

```
arm      : bb 00 01 b1 01 0a
n zones  : bb <len16> b0 <gradient> <nbSeg> <3 × nbSeg RGB bytes> <xor>
                └── len = 2 + 3 × nbSeg
```

**The `gradient` byte.** With `1` the firmware interpolates between zones and
wraps from the last back to the first, so a single lit zone at one end also
glows at the other. With `0` the zones are hard-edged. Which default a model
uses is a per-SKU fact — in the vendor's own desktop app it is a user setting
overridden for a list of models.

**Zone count is not fixed at 10.** `nbSeg` is a single byte and the length
field is 16-bit, so the protocol allows up to 255 zones. The Govee app exposes
10; firmwares accept more. The real ceiling is the number of individually
addressable LEDs, which the protocol never reports.

**Measuring the native resolution.** Asking for `n` zones makes the firmware
group LEDs into blocks of `ceil(N/n)`, so the rendering only changes when that
block size changes. Sweep `n` upwards, note every value where the pattern
refines, and solve for the `N` whose changepoints match — the sequence is
usually unique enough to identify `N` exactly.

This resolution depends on the **physical length** of the unit, not only on the
model: two devices sharing a SKU in different lengths do not share a value.
Record measured numbers in `devices/<SKU>.yaml`; never extrapolate one from
another.

### 2.4 Music mode — host-side, nothing to discover on the device

The vendor's desktop app captures audio, computes a spectrum, groups the LEDs
and streams the result over the `razer` channel of 2.3. The device only obeys.

Music mode is therefore reproducible locally over the channel documented
above; the only missing piece is audio capture, which is a host concern. On
macOS, capturing *system* audio needs a loopback device; a microphone is
directly accessible.

### 2.5 Internal scenes, DIY and per-segment brightness — not on this transport

The manufacturer's scene library does not travel over UDP. It is published over
MQTT to AWS IoT, with an account topic and a transaction id, so it needs a Govee
account and an internet connection. The `pt`, `ptReal`, `ptIotOp` and `bulb`
commands belong to that cloud channel — probing them over LAN stays silent
because the command exists, but not on this transport.

The same channel carries **per-segment brightness**, which the segment channel
of 2.3 does not offer: over LAN, brightness is global and color is per-segment.

This is a real boundary of `lan` mode, not a gap waiting to be filled. See
[`cloud.md`](cloud.md).

### 2.6 Sensors / telemetry

_TODO — nothing probed yet on a SKU that reports sensors._

### 2.7 Throughput

Write commands never answer, so a device's headroom cannot be measured directly.
Measure it indirectly: time `devStatus` round-trips at rest, then again **during**
a stream of segment frames. A rising RTT or dropped replies mark saturation.

The ceiling drops as frames get larger, since payload size grows with the zone
count. Two consequences for an implementation:

- derive the frame rate from the zone count rather than hardcoding one;
- back off on the measured ceiling of the device at hand, not on a constant.

Measured values belong in `devices/<SKU>.yaml`, alongside the length the
measurement was taken on.

### 2.8 Re-verify after firmware updates

Firmware changes can open or close behavior without notice, and a device that
answered a probe last month may not today. Ship the probes with the SDKs rather
than trusting a table: which command names answer, the real clamping bounds read
back through `devStatus`, and a visual test for the raw channel, which never
replies.

## 3. Discovery method

1. Try a candidate payload through the playground's raw payload field.
2. Capture the frame (`tcpdump`, Wireshark) → `tests/fixtures/lan-captures/`.
3. Document the protocol above; record what the SKU supports in its device file.
4. Formalize it in `devices/<sku>.yaml`.

Two things that paid off and are worth repeating:

- **Decompile the vendor's desktop app.** It implements the segment channel
  over LAN, so it pins down the frame format and the per-SKU tables behind it.
- **Capture that app's traffic.** `udp.port == 4003` in Wireshark while it
  drives a device yields real frames instead of guessed ones.

A failed probe on this protocol is indistinguishable from an ignored one — no
error is ever returned. Assume a malformed frame before assuming an unsupported
feature.
