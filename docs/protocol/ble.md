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

One exception: **a hidden network (§4) has never been provisioned.** The
trailing flag it takes comes from the phone controller, and no entry in a device file
encodes it.

`packages/rust/src/ble/` carries the transport, behind the cargo feature of the
same name. The bytes come from `devices/*.yaml`. The code matches a reply
against the `reply:` layout that the device file declares for the command that
asked for it, so it never guesses the correlation.

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
returns nothing while another controller holds the link. Check that first when
discovery finds no device that is plainly there.

### 1.2 Frame format

There is no MTU negotiation. A frame of every `proType` below but `0xA5` is
exactly 20 bytes:

```
| 0: proType | 1: commandType | 2..18: payload, zero-padded | 19: BCC |
```

The BCC is the XOR of bytes 0 to 18. `proType` says what kind of frame it is:

| `proType` | Frame |
| --------- | ----- |
| `0x33` | single write |
| `0xAA` | single read, answered on the notify characteristic |
| `0xA1` | multi-packet write, Wi-Fi provisioning |
| `0xA3` | multi-packet write, music effects |
| `0xA5` | colour from the host. Short frames, and a different checksum — see §8 |

A device file writes the layout as a `frame:` that ends in `<pad:20> <xor>`,
which is this shape. `0xA5` is the exception at both ends: its frames are
short, and their last byte is a sum. A device file writes that layout as a
`frame:` that ends in `<sum>` and carries no padding.

### 1.3 Discovery

A device advertises under one of three name families, and the family says how to
read the SKU out of the name:

| Name | SKU |
| ---- | --- |
| `GBK_<SKU>_<4 hex digits>` | the second underscore-separated field |
| `GV<SKU><4 hex digits>`, the SKU starting `H` or `R` | the five characters after `GV` |
| `ihoment_…`, `Govee_…`, `Minger_…` | older name prefixes |

The first two forms were both seen on units here; the third is reported rather
than seen. In the `GV` form the four hex digits carry the last two bytes of the
device's Wi-Fi MAC, with the final byte one higher. That held on the two units
measured, and nothing else confirms it.

An advertisement carries the Bluetooth address, and the rest of this project
identifies a device by its Wi-Fi MAC. Nothing observed relates the two, so the
transport asks the caller to bind them.

**A name family is not a dialect.** A device that advertises under one of these
names carries the GATT of 1.1; whether it answers the frames of 2 is a
per-device fact, and `devices/<SKU>.yaml` records it.

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

That answer is not a GATT acknowledgement, and no write on this wire gets one:
the write characteristic **refuses** a write with a response, with `Write Not
Permitted`. A frame therefore leaves in one direction only, and a link that
goes down too soon after a write loses it. Nothing reports the loss: the frame
is gone, and the device holds its previous state.

So a program that writes and then ends must hold the link open first. Measure
the wait on the unit: write a value, drop the link after `n` ms, then read the
value back. Record the result as `measurements.ble.write_drain_ms` in the
device file.

### 1.5 The advertisement data

The advertisement carries a six-byte field. It says whether the device encodes
its frames:

```
| 0: flags | 1..2: signature 88 EC | 3..4: pactType, big-endian | 5: pactCode |
```

Bit `0x40` of the flags byte says the device takes encoded frames only — see
§9. The low nibble of the flags byte is a version of the layout; nothing here
depends on it. `pactType` and `pactCode` name a firmware protocol generation.

A platform reads the first two bytes of the field as a 16-bit prefix,
little-endian, and hands the rest over. This layout does not start with such a
prefix, so a reader puts the two bytes back before it reads the layout. A
device file records nothing from this field. It is read live at each scan,
because a firmware update can raise the encoding bit on a device that shipped
without it.

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

The phone controller drives this field over a narrower band than the firmware
accepts. What it offers is therefore not evidence of the range.

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

Sets the zones the mask names to one level, and leaves the rest alone.

The mask field is 4 bytes, least significant byte first: zone 0 is bit 0 of the
first byte. The phone controller writes all four. A unit with 15 zones therefore
leaves the last two bytes zero, which is what the zero padding of §1.2 holds
anyway, so a 2-byte mask builds the same frame on such a unit. A longer unit of
the same product has more zones, and the field is what says a firmware can
address up to 32 of them. How many a given unit reads is a property of that
unit — read it with `aa 0f` (§3) and record it in its device file.

### 2.5 Per-zone brightness

```
33 05 15 03 <one level per zone>
```

All zones at once, one byte each, in zone order — as many bytes as the unit has
zones. There is no mask here: to leave a zone alone, the frame must repeat the
current level of that zone.

The firmware keeps these levels, and they are separate from the global
brightness of §2.2. It applies them in the static color modes of §2.3, and it
ignores them in a scene and in music mode, where it paints the zones itself.
A unit whose levels hold a ramp therefore fades from one end to the other on a
solid color, and looks correct on every animation.

The phone controller does not expose this setting, so the frame above is the only
way to change it. Read the levels back with `aa a5 <group>` (§3) — three zones
per group, and the only read that reports them.

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

### 2.7 Music sub-mode

```
33 05 <sub-mode> <effect> <sensitivity> <soft> <0|1> <R G B>
```

The device listens on its own microphone and renders what it hears. The
sub-mode byte is a property of the family: two families answer to 19, and 14,
17, 3 and 22, which other dialects carry, are acknowledged with a success code
and then played by nothing — see §1.4. The device file says which one its unit
takes.

The fields:

- `effect` selects the rendering. Which identifiers a firmware renders is a
  property of the family: one renders `0..7`, another renders a music effect at
  `0` and at `1`, a fixed white at `2`, and nothing above that. An identifier is
  not a name — a device file records the ones somebody watched on its unit, and
  this project maps none of them to an effect.
- `sensitivity` is how loud a sound must be to move the light. The phone
  controller drives it over a hundred values.
- `soft` chooses between two renderings of the same effect: `0` is sharp on the
  beat and `1` runs in fades. It was told apart on one family and changes
  nothing observable on another. The device file says which.
- The last flag chooses the colors. `0` lets the firmware choose them and
  ignores the triplet, which the device keeps stored. `1` plays the triplet.

Three traps:

- the firmware stores every byte of this frame and reports it back at §3, an
  effect it does not render included. A read that echoes a value is not
  evidence that the device plays it;
- the phone controller also offers its own microphone as the source. That is
  not a field of this frame. It is the channel of §8;
- an identifier the firmware renders nothing for leaves the light on what the
  frame before it rendered. Turn the device off between two identifiers, or a
  value that renders nothing reads as the rendering it kept.

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
| `aa ab` | dynamic API type, then the hidden-network flag. Two bytes. See §4 |
| `aa a5 <group>` | brightness and color for three zones. Groups are 1-based, five of them |

A read frame can carry a sub-type byte, as `aa 07 03` does. The same frame
without it gets no answer at all, which looks exactly like an unimplemented
read — see §7.

Three of these are traps:

- **What `aa 05` reports depends on the family.** On one it mirrors back codes
  the device never played, and the device file declares no command for it. On
  another it reports what is playing: a colour write moves it to the colour
  sub-mode carrying that colour. The device file says which.
- **`aa a5` reports the stored color sub-mode**, not the live render. Nobody
  established the byte layout of its answer, so the device file sends the read
  and declares no `reply:` for it.
- **`aa 07 11` gets no reply at all**, on the family where `aa 20` carries the
  hard version. That is indistinguishable from an unimplemented feature — see
  §7. The sub-type is what the answer depends on: `aa 07 03` answers on the
  other family.

## 4. Wi-Fi provisioning — `proType` `0xA1`, `commandType` `0x11`

Plaintext. No encoding, no seed exchange, no session token: anything within
Bluetooth range during provisioning sees the network password.

`provision_wifi()` runs the whole of this. Provisioning is four writes, in
this order:

1. `33 17 01` — wake the Wi-Fi module. See §2 for the `0x33` frame format.
2. Wait 3 s.
3. The `A1 11` transfer below.
4. `33 17 00` — release the module.

Step 1 is not optional. A transfer sent on its own left the unit off the
network. The same transfer, 3 s after `33 17 01`, put it on the network and
reached `lan` mode.

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
`https://device.govee.com`. Type 1 means `http://app.govee.com`.

`aa ab` answers a second byte after the type: the hidden-network flag. A device
that answers `1` takes one more byte at the end of the payload, `1` for a
hidden network and `0` for a visible one. With no API block that byte is the
third of `00 00 <flag>` instead. **Nobody provisioned a hidden network**, and no
device file encodes the flag: the unit here answers `0`.

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

Write 300 ms between two frames of the transfer. This is the pace the phone
controller keeps, for every device. Trap: a unit that takes the write budget of
§5 on a single frame drops the transfer at that rate. It then answers nothing,
and it stays off the network. The same transfer at 300 ms was accepted.

Status comes back on the notify characteristic as `A1 11 <status>`; `0` means
accepted. The codec reads no `reply:` on a chunked command, so the SDK does not
report this status. Whether the device joined the network is the only signal a
caller gets today.

## 5. Throughput

A read round trip over this mode is slower than one over `lan`, and a device
accepts only so many writes a second. Past that, the failure is not a dropped
frame: a burst leaves the firmware unresponsive for seconds. The transport
therefore paces writes against a budget, and does not trust a caller to do it.

The budget, the sustained rate, the burst ceiling and the recovery time are
properties of a unit, a host adapter and a radio environment. They are not
properties of a SKU. They live in the `measurements.ble` block of the device
file, and nowhere else.

The transport reads `write_budget_hz` for the device it writes to, so two
devices on one adapter are paced apart. A file that records none falls back to
the one budget anybody measured, which is a starting point and not a claim
about that device.

A repaint over §2.3 costs one write per distinct color, so a frame rate over
this mode falls with the number of colors in it.

## 6. Chunked writes — `proType` `0xA3`

A payload too long for one frame travels here. The cutting differs from §4 in
three ways, and each one is what makes a transfer built like §4 fail:

- The header carries payload bytes. It does not open the transfer and stop.
- The **closing frame carries the last piece.** Index `0xFF` is not an empty
  end marker.
- The count byte in the header is the number of frames in the transfer, the
  header and the closing frame included. It does not count data frames.

`commandType` `0x41` is a music effect, and §6.2 is that one. Four more command
types ride the same channel — `0x01`, `0x02`, `0x07` and `0x0A`. They are not
implemented: which cutter each one takes was not established, and no transfer
of that kind has been sent to a device.

### 6.1 The cutting

The first byte of a frame is the `proType`, the second is the index. The header
is index `0x00` and the closing frame is index `0xFF`:

```
A3 00 01 <frames> <commandType> <sub-command> <13 bytes>   header
A3 <i>   <17 bytes>                                        i = 1..
A3 FF    <last piece, up to 17 bytes>                      closing frame
```

Byte 2 of the header is `0x01` on every transfer seen here. Byte 3 counts the
frames: `2` where the body fits the header and the closing frame, and one more
per data frame after that. The header holds `15 - n` payload bytes, where `n`
is the number of command bytes after byte 3 — two of them for music, so 13.

Each piece is zero-padded out to its 17 bytes before the checksum, so a short
last piece looks like a full one with zeros after it.

A device file writes this as a `chunk:` block with a `head_size:`, a footer
that reads `${chunk:bytes}`, and `${total}` in the header. See
`devices/schema.yaml`.

### 6.2 Music effects — `commandType` `0x41`

The sub-command byte is the effect. The body is

```
[colour count][R G B]×count[per-effect parameters]
```

and the transfer stores it rather than plays it. What plays it is one frame of
§2.7's sub-mode, carrying the effect and the sensitivity and nothing else:

```
33 05 13 <effect> <sensitivity>
```

This is why an effect the single §2.7 frame acknowledges can render nothing:
that frame has no room for the parameters, and without a transfer before it the
firmware has none to render. The seven fields §2.7 lists belong to the effects
that need no transfer.

The colour count is one byte and the phone controller allows 1 to 8 entries. Which
parameters follow the colours is a property of the effect, not of the protocol:
a device file records the ones somebody drove on its unit, and the codec passes
them through uninterpreted.

Status comes back on the notify characteristic as `A3 <commandType>
<sub-command> <status>`; `0` means accepted. An accepted transfer is not a
rendered effect — §7 — and neither is the sub-mode read of §3 echoing it back.

Worked example — effect `0x32`, the seven-colour palette the phone controller sends
with no saved one, and a three-byte tail:

```
body     07 ff0000 ff7f00 ffff00 00ff00 0000ff 00ffff 8b00ff 03 00 63
header   a3000102413207ff0000ff7f00ffff0000ff0054
closing  a3ff0000ff00ffff8b00ff0300630000000000b7
plays it 3305133263000000000000000000000000000074
```

## 7. Probing

A failed probe and an unimplemented feature look identical: the firmware answers
nothing either way. Assume a malformed request before concluding that a device
lacks a capability, and re-check after a firmware update — behavior changes
without notice.

## 8. Colour from the host — `proType` `0xA5`

The host computes a colour and writes one frame per colour. The frames are
shorter than 20 bytes and the last byte is the **sum** of the bytes before it,
not the XOR of §1.2:

```
a5 02 90                request, does the device carry this channel
a5 02 83 <R G B>        one colour
```

Byte 1 is `0x02` on both frames. No device answered the request when that byte
held another value, so the byte is part of the address of the channel and not a
length.

The answer to the request differs by family. One device answered `a5 02 10 01`.
Another answered a five-byte constant that carries no `a5` and that two units
of the same model gave alike, so it is a property of the firmware and not of
the unit. Nobody established the layout of either answer. What the request
establishes is that the device carries the channel: a device that carries none
answers nothing.

`a5 02 83` renders the triplet it carries. Four properties were measured, and
each one is a trap for a caller that treats this as a colour command:

- **The render is temporary.** It survives the link going down. What ends it
  is the firmware asserting the state it stores, once a fixed hold runs out.
  So a device whose stored state is off ends no render, and one whose stored
  state is a colour comes back to that colour. Measure the hold against a
  stored state that is on, and record it as
  `measurements.ble.render_hold_ms` in the device file. Take the reading with
  no other traffic on any mode: a read is a candidate cause, and a write on
  another mode ends the render on its own.
- **It lights a device whose stored state is off.** The power of §2.1 does not
  gate it.
- **Nothing reports it.** The state reads of §3 keep reporting the stored
  state, and so does the LAN status channel. A read after a render reports what
  the device played before it.
- **The stored brightness scales it.** A triplet of `FF FF FF` is as bright as
  the brightness of §2.2 that the device holds, and `00 00 00` renders dark
  without turning the device off.

A device can carry this channel and answer nothing at all on §2 and §3. The
two are separate implementations in the firmware, so the silence of §7 on one
says nothing about the other. Probe both before you record what a device does.

## 9. The encoded link

A device that sets bit `0x40` of the flags byte (§1.5) takes encoded frames
only. A plaintext frame of §2 or §3 gets no answer at all: the silence is the
same as §7, so the flag is the only sign. The host colour channel of §8 stays
in the clear.

The encoding keeps the frame length, and the XOR checksum of §1.2 is computed
on the plaintext. A reply is decoded the same way.

A handshake runs once, as soon as the link and its notifications are up, and
before any command. It negotiates a session seed, under a base seed every
device of a generation takes:

| Step | Frame, plaintext | Answer, plaintext |
| ---- | ---------------- | ----------------- |
| 1 | `E7 01 <padding> <xor>` | `E7 01 <16-byte session seed> …` |
| 2 | `E7 02 <padding> <xor>` | the same frame, echoed back |

The padding carries nothing. The session seed covers every frame after step 2.

Where the base seed lives is the owner's call: an SDK that ships it lets anyone
who reads the code drive any device of this generation in range.
