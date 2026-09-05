# Changelog

All notable changes to the Rust packages are documented in this file.

The format is based on
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

`govee-toolkit` is the one crate published to crates.io. `govee-toolkit-sim`
and `xtask` are workspace crates with `publish = false`, and are covered by the
same entries.

## [0.2.0] — not yet released

The version the manifests carry. Nothing is published to crates.io and the
`rust-v0.2.0` tag has not been pushed; everything below is on `main` only.

### Added

- `govee_toolkit::codec` — the protocol codec. Reads `devices/*.yaml`, validates
  arguments and builds the exact bytes for a command, including the raw
  variable-length frame with its XOR checksum. No I/O, and no SKU name, command
  name or argument name in the code: a `role:` on a command, and one on an
  argument, are how the SDK reaches what it issues on its own initiative.
- The device catalogue is embedded at build time by `build.rs`, so an SDK is one
  artifact with no data directory to install. `GOVEE_DEVICES_DIR` overrides
  where the files are read from at build time.
- `Catalog::overlay()` — an opt-in, always-reported replacement of catalogue
  entries with locally supplied device files.
- `govee_toolkit::lan` — the `lan` transport, behind the default `lan` feature:
  multicast discovery, an on-disk device cache so a command never waits for a
  scan, one reused UDP socket, fire-and-verify, and a per-device circuit breaker
  with `OK` / `DEGRADED` / `DOWN` states.
- `govee_toolkit::stream` — the raw segment channel, armed once and fed frames
  on a clock paced from the rate measured on the unit. A write never blocks: a
  source faster than the device replaces the frame that had not gone out yet
  rather than being throttled.
- The facade at the crate root: configuration from
  `~/.config/govee-toolkit/config.yaml`, the enabled modes per device, the mode
  that served each command, and mode-transition events.
- `cargo run -p xtask` generates `dist/catalog.json`, one file holding every
  device, built by CI and attached to a release. An unknown `schema_version` is
  a typed error rather than a best-effort parse. Loading an external catalogue
  at runtime is not part of this.
- A codec-only build: with default features off there is no socket and no async
  runtime. `tools/check-no-io.sh` and a CI job keep it that way.
- `cargo test` fails when a command in the catalogue has no conformance vector.
- Property tests over the network-facing parsers: an arbitrary datagram is read
  or dropped, never a panic.
- `govee-toolkit-sim` — a fake device on UDP with fault injection (silence, late
  replies, dropped replies), so the transport and the breaker are tested in CI
  without hardware.
- Conformance vectors under `tests/fixtures/golden/`, replayed by `cargo test`,
  which also validates every `devices/*.yaml`.
- Workspace lints: `unsafe_code` forbidden, `unwrap` / `expect` / `panic` /
  `indexing_slicing` warned in library code, clippy `pedantic` and `cargo`
  groups on.
- CI (`ci.yml`) covering format on nightly, clippy, tests, docs, the MSRV,
  `cargo deny`, spelling, capture redaction, the codec-only build, the codec
  layering check and the 400-line-per-file limit, on Linux, macOS and Windows;
  `tools/qa.sh` runs the same list locally.

### Notes

- Mode dispatch is a `match` in the facade. A `Transport` trait is the
  prerequisite for the BLE pull request, not part of this one — see
  [`../../docs/architecture.md`](../../docs/architecture.md).
- `ble` and `cloud` are declared as modes but have no transport. Enabling one is
  reported as unavailable, never silently skipped.
- One SKU is verified end to end — see
  [`../../docs/compatibility.md`](../../docs/compatibility.md).
