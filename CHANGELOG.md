# Changelog

The device catalogue in [`devices/`](devices/), which all three packages read.
What a package's own code does is in its file, and what each has published is
on the repository's
[releases page](https://github.com/damient/govee-toolkit/releases) — a tag
there carries the changelog section it shipped.

| Package | Changelog | Version |
| ------- | --------- | ------- |
| `govee-toolkit` (Rust) | [`packages/rust/CHANGELOG.md`](packages/rust/CHANGELOG.md) | 0.3.0 |
| `govee-toolkit` (Python) | [`packages/python/CHANGELOG.md`](packages/python/CHANGELOG.md) | 0.0.0 |
| `govee-toolkit` (Node) | [`packages/node/CHANGELOG.md`](packages/node/CHANGELOG.md) | 0.0.0 |

## Catalogue

The catalogue has no version of its own: a package embeds it at build time and
ships it, so the date below is what a release pins. `catalog.json`, the
generated artefact, carries the schema revision it was built at.

### 2026-09-06

#### Changed

- `devices/H61A0.yaml`: the `ble` entry `fade` is `gradient`, and it claims the
  new `role: segment_gradient`. `33 a3` is not a fade over time — it is the same
  zone interpolation the `lan` segment channel carries as the first byte of its
  payload, and the colour frame has no room for it. Exercised on the unit: two
  zones painted differently meet on a hard edge at `0` and blend at `1`, while
  two colours sent one after the other cut to each other either way.
- The two firmware version reads declare `${version:ascii:17}` rather than
  `${version:ascii}`. Reading to the end of the frame took in the trailing
  checksum, which is neither padding nor printable, so both reads failed on
  every attempt. `aa 20` answers `1.02.00` and `aa 21` answers `2.06.02` on the
  verified unit, as the file already said.

### 2026-09-06

#### Added

- `devices/H61A0.yaml` declares `ble`, verified on the same unit: power,
  brightness, colour, colour temperature, brightness by zone mask, per-zone
  brightness, zone interpolation, seven `0xAA` read commands and a
  `role: status` entry that reads power and brightness in two more exchanges.
  Every one is `documented: false` and points at the section of
  [`docs/protocol/ble.md`](docs/protocol/ble.md) that describes it.
- `modes.ble` moves from `unknown` to `partial`. Segments are reachable there
  but narrower than over `lan` — fifteen zones by mask, against the unit's 42
  individually addressable LEDs — and scenes are `unimplemented`.
- Wi-Fi provisioning, as two chunked entries: one with the trailing API block
  and one without, since the layout has no optional field. **Neither has ever
  been sent to a device.** The layout was read out of the other direction only;
  its notes, its `verified:` line and every one of its conformance vectors say
  so, and the vectors pin this repository's encoder rather than the firmware.
- `measurements.ble` — the read round trip, the sustained write rate, the burst
  that leaves the firmware unresponsive and how long it stays that way, and the
  fifteen addressable zones, all taken on the same 3 m unit as the `lan`
  numbers. `frame_rate` carries no `ble` rows: dividing the write budget by one
  write per colour is arithmetic, not a stutter test, and the file says so
  where the rows would go.
- Conformance vectors in
  [`tests/fixtures/golden/ble/H61A0.json`](tests/fixtures/golden/ble/H61A0.json)
  for every `ble` command, including the refusals that prove an out-of-range
  value is an error and not a clamp: a zone past the width of the mask, a
  brightness of 0, a colour temperature under the device's range.
- `devices/schema.yaml` documents the `ble` command shape as the codec reads
  it: the same layout language as `lan` with no `cmd:` and no `payload:`, plus
  `reply:`, `frames:`, `body:` and `chunk:`. The schema revision stays 1.

#### Note

No BLE capture is committed. Redacting one is a step of its own, so every
`capture:` in the `ble` table is empty with a TODO beside it.

### 2026-09-06

#### Added

- `devices/H61A0.yaml` declares `ble` as `partial`, with its command table, the
  numbers measured on the unit and a `verified:` block that says what was
  exercised and what was not. Segments are reachable there but narrower than
  over `lan` — zones are addressed through a fifteen-bit mask, against the
  forty-two individually addressable ICs the `lan` raw channel reaches — and the
  file says so rather than omitting the capability.
- `tests/fixtures/golden/ble/H61A0.json` — conformance vectors for every `ble`
  command. Each one's `source` distinguishes bytes exercised on hardware from
  bytes worked out from the documented layout.
- `devices/schema.yaml` documents the constructs the `ble` work needed: the new
  frame tokens, the `string`, `zones` and `bytes` argument types, `body:` with
  `chunk:`, `reply:` with `frames:`, and the `segment_color_masked` role. The
  schema revision stays 1, which has a cost worth stating: a build older than
  this one reads a `chunk:` command without seeing the chunking, and encodes it
  to nothing rather than refusing the file. Nothing can be done about builds
  already published; use a catalogue no older than the SDK reading it.

### 2026-09-05

#### Added

- `devices/schema.yaml`, schema revision 1 — the device-file format, with the
  two executable mini-languages documented at the top of it: `payload:` for a
  JSON command, `frame:` for a raw variable-length frame with its XOR checksum.
- `devices/H61A0.yaml` — the first device file, verified on hardware at
  firmware 2.06.02: the documented LAN commands, four undocumented ones marked
  `documented: false`, and the measurements taken on a 3 m unit — 42 addressable
  LEDs, 10 zones in the app, idle latency and the sustainable frame rate per
  zone count. `candidate_aliases` lists the other lengths of the product, which
  are a different unit and not interchangeable.
- Capabilities are declared per mode: what the hardware can do, which of it a
  mode reaches, and a reason for the rest — `transport` when somebody
  established the transport does not carry it, `unimplemented` when the file
  declares no command for it yet, `unprobed` when nobody checked. `unprobed` is
  the default.
- `role:` on a command and on an argument, so a file names what the SDK issues
  on its own initiative and what it fills in — the segment channel drives a
  file whose arguments are called `armed`, `blend` and `pixels` as readily as
  the reference one.
- Conformance vectors under [`tests/fixtures/golden/`](tests/fixtures/golden/)
  for every command in the catalogue, each saying in its `source` whether the
  bytes come from a capture or were worked out from the documented layout.
  `cargo test` fails on a command that has none.
- The two tables in [`docs/compatibility.md`](docs/compatibility.md) are
  generated from the catalogue by `cargo run -p xtask -- compat`, and CI fails
  on drift.
