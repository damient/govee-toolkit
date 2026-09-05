# Changelog

The device catalogue in [`devices/`](devices/), which all three packages read.
What a package's own code does is in its file, and what each has published is
on the repository's
[releases page](https://github.com/damient/govee-toolkit/releases) — a tag
there carries the changelog section it shipped.

| Package | Changelog | Version |
| ------- | --------- | ------- |
| `govee-toolkit` (Rust) | [`packages/rust/CHANGELOG.md`](packages/rust/CHANGELOG.md) | 0.2.0 |
| `govee-toolkit` (Python) | [`packages/python/CHANGELOG.md`](packages/python/CHANGELOG.md) | 0.0.0 |
| `govee-toolkit` (Node) | [`packages/node/CHANGELOG.md`](packages/node/CHANGELOG.md) | 0.0.0 |

## Catalogue

The catalogue has no version of its own: a package embeds it at build time and
ships it, so the date below is what a release pins. `catalog.json`, the
generated artefact, carries the schema revision it was built at.

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
