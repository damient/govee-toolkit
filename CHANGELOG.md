# Changelog

This file records the device catalog in [`devices/`](devices/). Every package
reads that catalog. Each package records what its own code does in
its own file. The repository's
[releases page](https://github.com/damient/govee-toolkit/releases) shows what
each package published. A tag there carries the changelog section that it
shipped.

| Package | Changelog | Version |
| ------- | --------- | ------- |
| `govee-toolkit` (Rust) | [`packages/rust/CHANGELOG.md`](packages/rust/CHANGELOG.md) | 0.5.0 |
| `govee-toolkit-cli` (Rust) | [`packages/rust/crates/cli/CHANGELOG.md`](packages/rust/crates/cli/CHANGELOG.md) | unreleased |
| `govee-toolkit` (Python) | [`packages/python/CHANGELOG.md`](packages/python/CHANGELOG.md) | 0.0.0 |
| `govee-toolkit` (Node) | [`packages/node/CHANGELOG.md`](packages/node/CHANGELOG.md) | 0.0.0 |

## Catalog

The catalog has no version of its own. A package embeds the catalog at
build time and ships it, so a release pins the date below. `catalog.json` is
the generated artifact, and it carries the schema revision that it was built
at.

### 2026-09-11

#### Added

- `${<name>:rgb24}` in a `cloud` `payload:` — the one triple of an `rgb_list`
  argument, packed into 0xRRGGBB. A list of any other length is an error. It
  is what an entry marked `role: segment_color_masked` takes over that mode.
  `devices/schema.yaml` documents it beside `${r,g,b:rgb24}`.

#### Changed

- `H61A0` — `segment_color` under `cloud` claims `role: segment_color_masked`,
  so the SDK paints zones over that mode by role. It takes `colors`, an
  `rgb_list` of one color, and `zones`: the arguments the `ble` masked entry
  takes, so one command takes the same arguments over both modes. The
  conformance vectors in `tests/fixtures/golden/cloud/H61A0.json` carry the
  same bodies under the new argument names. A stream over this mode sends one
  request per distinct color, and the API takes one request every 6 seconds
  about one device. See [`docs/protocol/cloud.md`](docs/protocol/cloud.md) §5.

### 2026-09-10

#### Added

- Three command roles — `power`, `brightness` and `color` — and the argument
  roles `red`, `green` and `blue` beside them. A role names what a command
  does, so an SDK reaches it without a command name of its own. `H61A0`
  claims the three under `lan`, and `power` and `brightness` under `ble`;
  `H6114` claims the three under `ble`; the `cloud-openapi` family claims them
  under `cloud`. `devices/schema.yaml` documents each one and the arguments it
  must mark.
- A `zones` argument in a `cloud` `payload:` — the zones a segment capability
  writes, carried as the array this API takes in place of a mask.
  `devices/schema.yaml` documents it beside `${r,g,b:rgb24}`.
- `H61A0` declares `segment_color` and `segment_brightness` under `cloud` —
  the documented `devices.capabilities.segment_color_setting` capability, at
  its `segmentedColorRgb` and `segmentedBrightness` instances. Each takes an
  array of zones and paints the zones it names, leaving the rest alone. The
  zone range is 0 to 14, where the `lan` raw channel reaches 42 individually
  addressable ICs. Neither entry claims a painting role, so no stream opens
  over this mode: the API takes one request every few seconds. See
  [`docs/protocol/cloud.md`](docs/protocol/cloud.md) §5.
- The conformance vectors for both, in
  `tests/fixtures/golden/cloud/H61A0.json`. The two single-zone vectors went
  to a live account and the rope applied them, which each `source` says.
- `H61A0` declares `music` under `cloud` — the documented
  `devices.capabilities.music_setting` capability, at its `musicMode`
  instance. It takes an `effect` over 1 to 11, the enum the account list
  declares, and a `sensitivity` over 0 to 100. The device listens on its own
  microphone, and this mode carries no audio and no stream. The effect
  identifiers are this API's own: they are not the sub-mode codes the `ble`
  music frame takes. See [`docs/protocol/cloud.md`](docs/protocol/cloud.md)
  §6.
- The conformance vectors for `music`, in
  `tests/fixtures/golden/cloud/H61A0.json`. All three went to a live account
  and the rope played them, which each `source` says.
- `H61A0` records `measurements.zone_0_end: controller` — zone 0 is the block
  at the controller and the power cable, and zone 14 the block at the free
  end. Established over `cloud` on that unit, and not over `lan` or `ble`.
- `${name:mask32}` in a `frame:` layout — four bytes of zone bits, least
  significant bit first. The masked zone frames of the `ble` dialect carry a
  field that wide, so a unit with more than 16 zones declares its mask at full
  width. `devices/schema.yaml` documents it beside `${name:mask16}`. See
  [`docs/protocol/ble.md`](docs/protocol/ble.md) §2.4.

#### Changed

- `H61A0` — `modes.cloud` reaches `segments`, `segment_brightness` and
  `music`, each verified against a live account on the unit at firmware
  2.06.02, so that mode is `full`. Per-segment brightness over `cloud` is a
  capability `lan` does not reach: that channel carries color only. The music
  capability also declares an `autoColor` flag and an `rgb` color: the unit
  accepted both and played the same colors, so the entry sends neither.
- `H61A0` — `modes.lan` lists `music` unreachable for the `transport` reason.
  That mode carries no music command: the device renders no music over `lan`,
  and a host that streams computed colors over the segment channel is not this
  capability. See [`docs/protocol/lan.md`](docs/protocol/lan.md) §2.4.

### 2026-09-09

#### Added

- `measurements.ble.write_drain_ms` in `devices/schema.yaml` — how long the
  `ble` transport holds a link open after the last write, in milliseconds. That
  wire takes no acknowledgement, so a link dropped sooner loses the frame and
  reports nothing. The schema says how to measure it.
- `H61A0` records `measurements.ble.write_drain_ms: 50`. On that unit a link
  dropped 10 ms after a write loses the frame, and 25 ms carries it.
- `devices/families/cloud-openapi.yaml` — the five entries the documented
  HTTPS API carries for a light: `power`, `brightness`, `color`, `colortemp` and
  `status`. The capability names are the API's own. `H61A0` includes the
  fragment, and all five entries are verified on that unit.
- The `cloud:` command table in `devices/schema.yaml`. An entry names one
  `capability:` and the `payload:` that is its value, or a `role: status` with
  the `reads:` that say which capability answers into which argument. This wire
  carries no frame and no `cmd:`.
- `${r,g,b:rgb24}` in a `payload:` template — the three channel arguments
  packed into one integer, which is how the cloud API carries a color.
- `tests/fixtures/golden/cloud/H61A0.json` — the conformance vectors for that
  table, worked out from the documented API rather than from a capture, which
  each vector's `source` says.

#### Changed

- `H61A0` — `modes.cloud` claims `power`, `brightness`, `color` and
  `colortemp`, and every one is verified against a live account on the unit at
  firmware 2.06.02. A color of 255, 40, 0 reads back as 16721920. Color and
  white temperature exclude each other, as they do over `lan` and `ble`.
  `modes.ble` becomes `full`, since it now reaches every capability the file
  declares.
- A device stores its color setting through a power off, and `power` with `on`
  = 1 restores it. `power` on is not a full state write. See
  [`docs/protocol/state.md`](docs/protocol/state.md).

#### Removed

- **Breaking:** the `scenes` capability, and every command that wrote one. The
  project does not carry scenes in any mode. `H6114` loses its `scene` entry
  and its conformance vector, `H61A0` loses the capability, and
  `docs/protocol/ble.md` loses the sub-mode it documented. A file that still
  declares `scenes:` gets a column of its own and no SDK support.
- **Breaking:** `capture:`, the per-command key that pointed at a capture file.
  The conformance vector's `source` carries the provenance instead, under
  `tests/fixtures/golden/<mode>/<SKU>.json`: it says whether the bytes come from
  a capture or were worked out from the documented layout. Captures keep their
  place under `tests/fixtures/lan-captures/` and `tests/fixtures/ble-captures/`,
  and the redaction checklist in `tests/fixtures/README.md` covers them. The key
  goes from `devices/schema.yaml`, `devices/H6114.yaml`, `devices/H61A0.yaml`
  and `devices/families/ble-wifi-provision.yaml`.

### 2026-09-07

#### Added

- `devices/families/` and the `include:` key. A dialect several SKUs speak to
  the byte lives in one fragment, and a device file names it instead of
  repeating it. The loader merges the fragment's commands in, so validation,
  the encoder and `catalog.json` all see one flat device. A fragment carries
  layout only. Two clashes are errors rather than overrides: an `include:`
  naming no fragment, and a command a file and its fragment both declare.
- `devices/families/ble-wifi-provision.yaml`, the four entries
  `provision_wifi()` runs: `read_dynamic_api` at `0xAA` `0xAB`,
  `wifi_link_start` at `0x33` `0x17`, and the two `0xA1` `0x11` transfers.
  `devices/H61A0.yaml` includes it and declares none of them itself.
- `wifi_link_start` wakes the Wi-Fi module before provisioning and releases it
  after. The module answers no provisioning transfer until it arrives: on the
  H61A0 a transfer sent on its own left the unit off the network, and the same
  transfer 3 s later put it on. The unit then answered over `lan`.
- Four command roles and nine argument roles in
  [`devices/schema.yaml`](devices/schema.yaml): `wifi_link`, `wifi_api_type`,
  `wifi_provision` and `wifi_provision_with_api`, with the arguments an SDK
  fills for each. A device file tags its entries and the SDK runs them in
  order, so provisioning carries no command name in code.
- `music_effect` on `devices/H61A0.yaml`, the chunked music channel at `0xA3`
  `0x41`. A transfer carries a color list and the parameters of one effect, and
  a `33 05 13` frame after it plays what was transferred. This is the channel
  that reaches the effects the single `music` frame acknowledges and renders
  nothing for: effect 50 renders here. Six effect codes were driven on the unit
  and all six rendered, each visibly different from the others. **None of the
  six was matched to the label the vendor app gives its code** — the file, its
  `verified:` block and
  [`docs/protocol/ble.md`](docs/protocol/ble.md) 6 all state this. The
  per-effect parameter tail is a `bytes` argument the codec does not interpret,
  which the notes call out as the second place in that file where the no-clamp
  guarantee does not hold.
- [`docs/protocol/ble.md`](docs/protocol/ble.md) 6 describes the `0xA3` cutting
  rather than leaving it open. The header carries payload bytes, the closing
  frame carries the last piece, and the count byte in the header counts every
  frame of the transfer. Scenes stay unimplemented: they ride the same
  `proType` under another command type, and no scene transfer has been sent to
  a device.
- `devices/H61A0.yaml` declares `ble`, verified on the same unit. The commands
  are power, brightness, color, color temperature, brightness by zone mask,
  per-zone brightness and zone interpolation. The file also declares seven
  `0xAA` read commands and a `role: status` entry. That entry reads power and
  brightness in two more exchanges. Each command is `documented: false`, and
  each one points to the section of
  [`docs/protocol/ble.md`](docs/protocol/ble.md) that describes it.
- `modes.ble` moves from `unknown` to `partial`. Segments are reachable over
  `ble`, but they are narrower than over `lan`: fifteen zones by mask, against
  the unit's 42 individually addressable LEDs. Scenes are `unimplemented`.
- Wi-Fi provisioning, as two chunked entries: one with the trailing API block
  and one without it. The layout has no optional field. The entry carrying the
  block put a unit on a network. The entry without it was not sent: the unit
  asks for an endpoint.
- `measurements.ble` — the read round trip, the sustained write rate, the burst
  that makes the firmware unresponsive, the time that the firmware stays
  unresponsive, and the fifteen addressable zones. All of these numbers come
  from the same 3 m unit as the `lan` numbers. `frame_rate` carries no `ble`
  rows. A division of the write budget by one write per color is arithmetic,
  not a stutter test, and the file states this where the rows would go.
- Conformance vectors in
  [`tests/fixtures/golden/ble/H61A0.json`](tests/fixtures/golden/ble/H61A0.json)
  for every `ble` command. The vectors include the refusals that prove that an
  out-of-range value is an error and not a clamp. The refused values are a zone
  past the width of the mask, a brightness of 0, and a color temperature under
  the device's range.
- `devices/schema.yaml` documents the `ble` command shape as the codec reads
  it. The shape uses the same layout language as `lan`, with no `cmd:` and no
  `payload:`. It adds the new frame tokens, the `string`, `zones` and `bytes`
  argument types, `reply:` with `frames:`, `body:` with `chunk:`, and the
  `segment_color_masked` role. The schema revision stays 1, which has a cost
  worth stating: a build older than this one reads a `chunk:` command without
  seeing the chunking, and encodes it to nothing rather than refusing the file.
  Nothing can be done about builds already published; use a catalog no older
  than the SDK reading it.

- `devices/H61A0.yaml` declares the music sub-mode over `ble`, and `music`
  joins the capabilities the file lists. The sub-mode is `19` and the device
  listens on its own microphone. The frame carries `effect`, `sensitivity`,
  `soft`, a colour mode and an RGB triplet. `effect` runs `0..7`: each of the
  eight renders one effect, several of them are visibly different from each
  other, and `8..15`, `50` and `255` render nothing at all. Nothing maps an
  identifier to an effect. `soft` is `0` for a rendering that is sharp on the
  beat and `1` for one that runs in fades. The colour mode is `0` to leave the
  colours to the firmware and `1` to impose the triplet, which plays the rope
  in that colour and no other. `sensitivity` runs `0..99`, the range the vendor
  app drives; the unit reacted at `0`, `99` and `255` alike, and what the value
  changes was not told apart there. Music is `unprobed` over `lan` and over
  `cloud`.
- `devices/H61A0.yaml` declares `read_mode`, the `aa 05` read. It answers the
  live sub-mode and its eight payload bytes: `21` after a colour frame and `19`
  after a music one, each with the fields of that frame. It reports what was
  written, which is not what the firmware renders: it echoes an identifier that
  the device plays with nothing, so a read is not evidence on its own.
- `devices/H6114.yaml` declares the RGB Car LED Strip Lights, a 12 V interior
  car light with four 22 cm strips and 48 LEDs. The four strips carry one
  colour: the hardware is RGB, so the file declares power, brightness, colour,
  scenes and music, and no zone capability. `lan` and `cloud` are `none`,
  because Govee gives the model one radio, Bluetooth.
- `modes.ble` is `full`, verified on the unit against soft version 2.03.11
  and hard version 2.01.01. The commands are power, brightness, colour, the
  scene sub-mode and the music sub-mode, plus four reads: power, brightness,
  the live sub-mode and the two versions. Each command is `documented: false`
  and points to the section of [`docs/protocol/ble.md`](docs/protocol/ble.md)
  that describes it.
- The colour sub-mode of this family is `13`, and its payload carries a 16-bit
  kelvin field between the two RGB triplets. Sub-mode `2`, which an older
  dialect uses, is acknowledged with a success code and then ignored.
- The music sub-mode of this family is `19`. The device listens on its own
  microphone. The frame carries `effect`, `sensitivity`, `soft`, a colour mode
  and an RGB triplet. `effect` selects the rendering: `0` and `1` each render a
  music effect, `2` renders a fixed white, and `3..7` render nothing and leave
  the strips dark. Nothing told `0` apart from `1` on the unit, so neither
  identifier maps to an effect. `soft` changes nothing observable there.
  `sensitivity` runs `1..100`. The colour mode is `0` to leave the colours to
  the firmware, which then ignores the triplet and keeps the last one it was
  given, and `1` to impose the triplet. Sub-modes `14`, `17`, `3` and `22`,
  which other dialects use, are acknowledged with a success code and then
  ignored.
- Brightness takes the whole byte, `0..255`, and not the percent the other
  family takes. `0` renders nothing and does **not** turn the device off: it
  reports itself on with a brightness of 0.
- The file declares no `colortemp`. The colour frame carries a kelvin field and
  the firmware renders no temperature: a frame naming a temperature and no RGB
  leaves the strips dark, and one naming a temperature and a green triplet
  lights them green.

#### Changed

- `read_dynamic_api` reads a second byte: the hidden-network flag beside the
  endpoint type. The H61A0 answers type 2 and flag 0. A device answering 1
  takes a trailing flag byte that no entry encodes, and
  [`docs/protocol/ble.md`](docs/protocol/ble.md) 4 says so.
- `modes.ble` on `devices/H61A0.yaml` records Wi-Fi provisioning as working.
- `devices/H6114.yaml`: the byte between the sensitivity and the colour mode of
  the music frame is the `soft` flag of
  [`docs/protocol/ble.md`](docs/protocol/ble.md) 2.8, and the file declares it
  as an argument over `0..1`. The unit stores it and reads it back at `aa 05`,
  and the two values render the same thing there. The vendor app drives this
  device at sub-mode `3`, which carries neither a `soft` byte nor an `effect`
  byte, so neither field has a value the app writes to compare against.
- `devices/H61A0.yaml`: the `ble` entry `fade` becomes `gradient`, and it
  declares the new `role: segment_gradient`. `33 a3` is not a fade over time.
  It is the same zone interpolation that the `lan` segment channel carries as
  the first byte of its payload. The color frame has no space for that
  interpolation. The unit shows this behavior: two zones with different colors
  meet on a hard edge at `0` and blend at `1`. Two colors that go to the device
  one after the other cut to each other in both cases.
- The two firmware version reads declare `${version:ascii:17}` instead of
  `${version:ascii}`. A read to the end of the frame includes the trailing
  checksum. That checksum is not padding and it is not printable. Both reads
  thus failed on every attempt. `aa 20` answers `1.02.00` and `aa 21` answers
  `2.06.02` on the verified unit. The file already stated these two answers.
- `measurements.ble.write_budget_hz` is the second entry of a `measurements:`
  block that an SDK reads, beside `frame_rate`. It is the sustained rate the
  `ble` transport paces its writes to, and it must be at or under the
  `sustained_writes_hz` measured on the same unit. `cargo test` refuses a file
  that breaks either rule. `burst_frames_before_stall` stays free-form: it
  records the count that stalled a unit, and nothing derives a budget from it.
  [`devices/schema.yaml`](devices/schema.yaml) says so where the block is
  described.

#### Note

Every `ble` command in the two files has been sent to a unit, and its effect or
its answer was observed.

On the H6114 the firmware fades from one colour to the next and fades in at
power on. `33 A3`, which sets zone interpolation on a device that has zones, is
accepted there and changes nothing observable. Scene identifiers were not
enumerated. DIY and Wi-Fi provisioning were not exercised. The firmware accepts
a value past every music range above and reads it back, so a read that echoes a
value is not evidence that the device plays it. Between two identifiers the
device must be turned off: one that the firmware renders nothing for otherwise
reads as the rendering it kept.

No BLE capture is committed. A redaction is a step of its own. Every `capture:`
in the `ble` tables is thus empty, with a TODO beside it.

### 2026-09-05

#### Added

- `devices/schema.yaml`, schema revision 1 — the device-file format. The top of
  that file documents the two executable mini-languages: `payload:` for a JSON
  command, and `frame:` for a raw variable-length frame with its XOR checksum.
- `devices/H61A0.yaml` — the first device file, verified on hardware at
  firmware 2.06.02. It carries the documented LAN commands, four undocumented
  commands marked `documented: false`, and the measurements taken on a 3 m
  unit. Those measurements are 42 addressable LEDs, 10 zones in the app, the
  idle latency and the sustainable frame rate per zone count.
  `candidate_aliases` lists the other lengths of the product. Those lengths are
  a different unit and they are not interchangeable.
- The file declares the capabilities per mode: what the hardware can do, which
  part of it a mode reaches, and a reason for the rest. The default reason is
  `unprobed`. The reasons are:
  - `transport` — somebody established that the transport does not carry the
    capability.
  - `unimplemented` — the file declares no command for the capability yet.
  - `unprobed` — nobody checked.
- `role:` on a command and on an argument. A file thus names what the SDK
  issues on its own initiative and what the SDK fills in. The segment channel
  drives a file whose arguments are called `armed`, `blend` and `pixels` as
  easily as it drives the reference file.
- Conformance vectors under [`tests/fixtures/golden/`](tests/fixtures/golden/)
  for every command in the catalog. The `source` of each vector states
  whether the bytes come from a capture or were worked out from the documented
  layout. `cargo test` fails on a command that has no vector.
- `cargo run -p xtask -- compat` generates the two tables in
  [`docs/compatibility.md`](docs/compatibility.md) from the catalog. CI fails
  on drift.
