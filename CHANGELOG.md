# Changelog

This file records the device catalog in [`devices/`](devices/). All three
packages read that catalog. Each package records what its own code does in
its own file. The repository's
[releases page](https://github.com/damient/govee-toolkit/releases) shows what
each package published. A tag there carries the changelog section that it
shipped.

| Package | Changelog | Version |
| ------- | --------- | ------- |
| `govee-toolkit` (Rust) | [`packages/rust/CHANGELOG.md`](packages/rust/CHANGELOG.md) | 0.3.0 |
| `govee-toolkit` (Python) | [`packages/python/CHANGELOG.md`](packages/python/CHANGELOG.md) | 0.0.0 |
| `govee-toolkit` (Node) | [`packages/node/CHANGELOG.md`](packages/node/CHANGELOG.md) | 0.0.0 |

## Catalog

The catalog has no version of its own. A package embeds the catalog at
build time and ships it, so a release pins the date below. `catalog.json` is
the generated artifact, and it carries the schema revision that it was built
at.

### 2026-09-06

#### Changed

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

#### Note

Every `ble` command in the file has been sent to the unit, and its effect or
its answer was observed. Wi-Fi provisioning is the exception. That one has
still never been sent to a device. The `capture:` fields stay empty. No BLE
capture has been redacted and committed yet.

### 2026-09-06

#### Added

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
  and one without it. The layout has no optional field. **Neither has ever been
  sent to a device.** The layout was read from the other direction only. Its
  notes, its `verified:` line and every one of its conformance vectors state
  this, and the vectors pin this repository's encoder rather than the firmware.
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
  `payload:`. It adds `reply:`, `frames:`, `body:` and `chunk:`. The schema
  revision stays 1.

#### Note

No BLE capture is committed. A redaction is a step of its own. Every
`capture:` in the `ble` table is thus empty, with a TODO beside it.

### 2026-09-06

#### Added

- `devices/H61A0.yaml` declares `ble` as `partial`. The file carries the
  command table, the numbers measured on the unit and a `verified:` block. That
  block states what was exercised and what was not. Segments are reachable over
  `ble`, but they are narrower than over `lan`: a fifteen-bit mask addresses
  the zones, against the forty-two individually addressable ICs that the `lan`
  raw channel reaches. The file states this limit and does not omit the
  capability.
- `tests/fixtures/golden/ble/H61A0.json` — conformance vectors for every `ble`
  command. The `source` of each vector separates bytes exercised on hardware
  from bytes worked out from the documented layout.
- `devices/schema.yaml` documents the constructs that the `ble` work needed:
  the new frame tokens, the `string`, `zones` and `bytes` argument types,
  `body:` with `chunk:`, `reply:` with `frames:`, and the
  `segment_color_masked` role. The schema revision stays 1, which has a cost
  worth stating: a build older than this one reads a `chunk:` command without
  seeing the chunking, and encodes it to nothing rather than refusing the file.
  Nothing can be done about builds already published; use a catalog no older
  than the SDK reading it.

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
