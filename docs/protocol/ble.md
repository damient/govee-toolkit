# Govee BLE protocol (`ble` mode)

`ble` is one of the three modes a user can enable per device — see
[`../modes.md`](../modes.md). It is opt-in and never enabled implicitly.

It works within Bluetooth range, and the device does not need to be on the
network. It requires a Bluetooth adapter on the host. Capability coverage
depends on the SKU family and is usually narrower than `lan`.

## Status

Everything below comes from physical units. The device file records the unit
it was taken on: the SKU, the firmware versions and every number measured. Two
families are covered so far, and they do not speak the same dialect. Where they
differ, this page gives both layouts and the device file says which one its unit
takes. A third family can differ again.

One exception: **the write direction of Wi-Fi provisioning (§4) has never been
sent to a device.** The layout comes from the other direction of the transfer
only. The golden vectors under `tests/fixtures/golden/ble/` reproduce it byte
for byte. That is a statement about this repository's encoder, not about what
the firmware accepts.

`packages/rust/src/ble/` carries the transport, behind the cargo feature of the
same name. The bytes come from `devices/*.yaml`. The code matches a reply
against the `reply:` layout that the device file declares for the command that
asked for it, so it never guesses the correlation.

Scenes (§6) are not implemented.

One data point from the LAN work: the raw LAN channel uses a
variable-length dialect prefixed `0xBB`, **distinct** from the 20-byte `0x33`
frames used here. Frames do not port between the two.

## 1. Link

### 1.1 GATT

| | |
| --- | --- |
| Service | `00010203-0405-0607-0809-0a0b0c0d1910` |
| Write characteristic | `00010203-0405-0607-0809-0a0b0c0d2b11`, write **without** response |
| Notify characteristic | `00010203-0405-0607-0809-0a0b0c0d2b10` |

One connection at a time. A connected device stops advertising, so a scan
returns nothing while the vendor's app holds the link. Check that first when
discovery finds no device that is plainly there.

### 1.2 Frame format

There is no MTU negotiation. Every frame is exactly 20 bytes:

```
| 0: proType | 1: commandType | 2..18: payload, zero-padded | 19: BCC |
```

The BCC is the XOR of bytes 0 to 18. `proType` says what kind of frame it is:

| `proType` | Frame |
| --------- | ----- |
| `0x33` | single write |
| `0xAA` | single read, answered on the notify characteristic |
| `0xA1` | multi-packet write, Wi-Fi provisioning |
| `0xA3` | multi-packet write, scenes |

A device file writes the layout as a `frame:` that ends in `<pad:20> <xor>`,
which is this shape.

### 1.3 Discovery

The advertised name is `GBK_<SKU>_<4 hex digits>`, and the SKU is the second
underscore-separated field. Older families advertise names prefixed `ihoment_`
or `Minger_` instead; neither was seen on the unit measured.

An advertisement carries the Bluetooth address, and the rest of this project
identifies a device by its Wi-Fi MAC. Nothing observed relates the two, so the
transport asks the caller to bind them.

### 1.4 What a write answers

The device answers a `0x33` write on the notify characteristic, under the two
bytes it was sent with:

```
33 <commandType> <status>
```

A status of `00` says the firmware accepted the frame. It does **not** say the
firmware applied it: a sub-mode the device does not implement is acknowledged
with `00` and then played by nothing. To know that a write took effect, watch
the device or read the state back with §3.

## 2. Writes — `proType` `0x33`

Every frame below is padded with zeros to byte 18, and byte 19 is the BCC.

### 2.1 Power

```
33 01 <0|1>
```

### 2.2 Brightness

```
33 04 <level>
```

One byte, and the scale is a property of the family. One family takes percent,
`1..100`. Another takes the whole byte, `0..255`. The device file carries the
range its unit takes, and nothing derives one scale from the other.

Where the field takes the whole byte, `0` is a level and not an off switch: the
device goes dark and still reports itself on, so only §2.1 turns it off.

The vendor app drives this field over a narrower band than the firmware accepts.
What the app offers is therefore not evidence of the range.

### 2.3 Color and white

The byte after the command type is the sub-mode, and it decides the layout of
what follows. Three layouts are known:

```
33 05 15 01 <R G B> <K_hi K_lo> <Rw Gw Bw> <mask>   zones, by mask
33 05 0d    <R G B> <K_hi K_lo> <Rw Gw Bw>          one colour for the device
33 05 02    <R G B> <flag>      <Rw Gw Bw>          older, a flag in place of
                                                    the kelvin field
```

A device answers a sub-mode it does not implement with a success code and then
plays nothing — see §1.4. Read §3 back to tell the two apart.

Color and white are mutually exclusive. A color temperature zeroes the leading
RGB triplet and carries the RGB **rendering** of that temperature in the second
triplet. The firmware does not compute that rendering. The host must send both:
the kelvin value alone leaves the strip dark. Kelvin range 2000..9000.

The layouts with no mask carry one colour for the whole device.

The mask is one bit per zone, least significant bit first, `ceil(count / 8)`
bytes wide. The field has room for 56 bits, and the firmware answers to fewer:
bits past the zone count are inert. That is a firmware limit, not a format
limit. How many zones a unit addresses is in its device file.

Two traps:

- a saturated mask looks exactly like an ignored mask, so "every zone changed"
  is not evidence that the mask was read;
- an out-of-range mask is indistinguishable from a no-op. The codec refuses a
  zone index the mask cannot carry rather than letting the firmware drop it in
  silence.

### 2.4 Brightness of masked zones

```
33 05 15 02 <level> <mask>
```

### 2.5 Per-zone brightness

```
33 05 15 03 <one level per zone>
```

All zones at once, one byte each, in zone order — as many bytes as the unit has
zones. There is no mask here: to leave a zone alone, the frame must repeat the
current level of that zone.

### 2.6 Zone interpolation

```
33 a3 <0|1>
```

Sets whether the firmware interpolates between zones. The interpolation wraps
from the last zone back to the first. `0` gives hard-edged zones.

It is the same user setting the LAN segment channel carries as the first byte of
its payload. The color frame above has no room left for it, so this mode gives
it a frame of its own. This is not a fade over time: two colors sent one after
the other cut to each other either way. The setting changes the boundary between
two zones painted differently.

A device with no zones accepts this frame and changes nothing observable, in
either position. Where a firmware fades from one colour to the next, this is not
the frame that turns the fade off, and no frame that does was found.

### 2.7 Scene sub-mode

```
33 05 04 <identifier>
```

Plays a scene the firmware carries. The identifiers are firmware data rather
than protocol, and the names the vendor app gives them do not match what a
device plays. Nothing here maps an identifier to an effect: a device file
records the ones somebody watched on its unit.

## 3. Reads — `proType` `0xAA`

Each read is answered on the notify characteristic, under the same two leading
bytes it was asked with.

| Frame | Answer |
| ----- | ------ |
| `aa 01` | `aa 01 <0\|1>` — power |
| `aa 04` | `aa 04 <1..100>` — brightness |
| `aa 0f` | segment count, one byte |
| `aa 40` | IC count, 16-bit big-endian; matches the LAN native resolution |
| `aa 05` | the live sub-mode, then its payload |
| `aa 14` | Wi-Fi MAC, 6 bytes |
| `aa 20` | hard version, ASCII |
| `aa 21` | soft version, ASCII |
| `aa 06` | soft version, ASCII, on a family that answers nothing at `aa 21` |
| `aa 07 03` | hard version, ASCII. The `03` is part of the request, and the answer repeats it |
| `aa ab` | dynamic API type, see §4 |
| `aa a5 <group>` | brightness and color for three zones. Groups are 1-based, five of them |

A read frame can carry a sub-type byte, as `aa 07 03` does. The same frame
without it gets no answer at all, which looks exactly like an unimplemented
read — see §7.

Three of these are traps:

- **What `aa 05` reports depends on the family.** On one it mirrors back codes
  the device never played, and the device file declares no command for it. On
  another it reports what is playing: a colour write moves it to the colour
  sub-mode carrying that colour, and a scene write moves it to the scene
  sub-mode carrying that identifier. The device file says which.
- **`aa a5` reports the stored color sub-mode**, not the live render. Nobody
  established the byte layout of its answer, so the device file sends the read
  and declares no `reply:` for it.
- **`aa 07 11` gets no reply at all**, on the family where `aa 20` carries the
  hard version. That is indistinguishable from an unimplemented feature — see
  §7. The sub-type is what the answer depends on: `aa 07 03` answers on the
  other family.

## 4. Wi-Fi provisioning — `proType` `0xA1`, `commandType` `0x11`

Plaintext. No encryption, no key exchange, no session token: anything within
Bluetooth range during provisioning sees the network password.

The payload is

```
[len_ssid][ssid utf8][len_pwd][pwd utf8][runMode][tz_h][iotVer][tz_min]
```

optionally followed by

```
[len_api_hi][len_api_lo][api url utf8]
```

An empty password is the single byte `0x00`. `runMode` and `iotVer` are `0` in
production. `tz_h` and `tz_min` are separate bytes, hours and minutes — not a
combined offset in minutes, and not a fraction. Nobody observed a negative
offset.

The API block is sent when `aa ab` (§3) answers with a type. Type 2 means
`https://device.govee.com`.

Cut the payload into 16-byte pieces. The data of each piece starts at byte 3:

```
A1 11 00 <nb_packets> 00 ...   start
A1 11 <i> <16 bytes>           i = 1..nb_packets
A1 11 FF ...                   end
```

Worked example — SSID `Test`, password `abc`, UTC+2, no API block:

```
payload  04 54657374 03 616263 00 02 00 00           (13 bytes)
start    a1110001000000000000000000000000000000b1
data 1   a1110104546573740361626300020000000000e2
end      a111ff000000000000000000000000000000004f
```

Status comes back on the notify characteristic as `A1 11 <status>`; `0` means
accepted.

**None of this has been sent to a device.** See Status above.

## 5. Throughput

A read round trip over this mode is slower than one over `lan`, and a device
accepts only so many writes a second. Past that, the failure is not a dropped
frame: a burst leaves the firmware unresponsive for seconds. The transport
therefore paces writes against a budget, and does not trust a caller to do it.

The budget, the sustained rate, the burst ceiling and the recovery time are
properties of a unit, a host adapter and a radio environment. They are not
properties of a SKU. They live in the `measurements.ble` block of the device
file, and nowhere else.

A repaint over §2.3 costs one write per distinct color, so a frame rate over
this mode falls with the number of colors in it.

## 6. Scenes — `proType` `0xA3`

Not implemented. The chunking differs from §4: 17-byte pieces, and the header
carries the first 14 payload bytes. The packet-count byte in that header does
not follow from what was observed, and a guess would invent a verification
nobody did.

## 7. Probing

A failed probe and an unimplemented feature look identical: the firmware answers
nothing either way. Assume a malformed request before concluding that a device
lacks a capability, and re-check after a firmware update — behavior changes
without notice.

## Captures

Real frames live under `tests/fixtures/ble-captures/<sku>/`. Nothing is
committed there yet. Redact a capture by hand before you commit it; the
checklist is in [`../../tests/fixtures/README.md`](../../tests/fixtures/README.md).
