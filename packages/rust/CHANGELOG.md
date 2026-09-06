# Changelog

Changes to `govee-toolkit`, the crate published to crates.io from
`packages/rust`. `govee-toolkit-sim` and `xtask` are workspace crates with
`publish = false`, covered by the same entries. The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

## [Unreleased]

### Added

- `govee_toolkit::transport` — what every mode shares, lifted out of the facade:
  the `Transport` trait a mode implements, the transport-neutral error, the
  circuit breaker, `DeviceStatus` and the event types. `lan` re-exports all of
  it and keeps its richer inherent surface.
- `govee_toolkit::ble` — the `ble` transport, behind the new non-default `ble`
  feature: a scan over advertised names, one held connection per device, a
  paced write path and the same per-device circuit breaker. **The protocol it
  speaks is not verified against a device**: `docs/protocol/ble.md` records
  nothing probed, no device file declares a `ble` command, and every constant
  in the module says so. It sends the frames the codec built and reads nothing
  out of a reply.
- `ble::Transport::bind` relates a device's identity to the Bluetooth address a
  scan heard. Nothing infers one from the other.
- The device file can describe a fixed-size frame and a chunked write:
  `<pad:N>` zero-fills a layout, `${name:str8}` / `${name:str16}`,
  `${name:mask8}` / `${name:mask16}` and `${name:bytes}` are new field tokens,
  and `chunk:` splits one payload across several frames. `Encoded` carries
  `frames` beside an envelope that is now optional.
- Error codes `field_too_long`, `frame_overflow`, `chunk_syntax`, `serialize`,
  `no_envelope` and, on a transport, `out_of_range` for an option outside the
  range the mode can honour — an out-of-range write budget is refused, never
  moved to the nearest value it could serve.

### Changed

- The facade holds one transport per mode and looks the mode up, rather than
  matching on it in every method. `Govee::attach` takes the transports and
  refuses two claiming the same mode. `Device.lan_health` became
  `Device.health`, one entry per mode.

## [0.2.1] — 2026-09-05

The code is unchanged: this release fixes the crate metadata and the page
0.2.0 put on crates.io.

### Changed

- `authors` is `damient`.
- The crate README links to `docs/` by absolute URL: it is the description shown
  on crates.io, where a link out of the package directory is dead.

## [0.2.0] — 2026-09-05

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
  rather than being throttled. Exactly one disarming frame goes out, sent by the
  emitting task: `close` awaits it and reports what it did, dropping the handle
  signals it, and a fatal encoding error disarms as the task leaves.
- `govee_toolkit::codec::capabilities` — what the hardware can do, and per mode
  which of it is out of reach and why: `transport` when somebody established the
  transport does not carry it, `unimplemented` when the file declares no command
  for it yet, `unprobed` — the default — when nobody checked. Capability names
  are data; the codec reads one of them, `segments`, and treats the rest as
  opaque strings. A parameter it does not know fails the file to load rather
  than being ignored.
- The facade at the crate root: configuration from
  `~/.config/govee-toolkit/config.yaml`, the enabled modes per device, the mode
  that served each command, and mode-transition events.
- Enabling `ble` or `cloud` reports the mode as unavailable: a mode the build
  carries no transport for is reported, never silently skipped and never
  substituted with another mode.
- `DeviceHandle::status()` asks the device and waits, `last_status()` returns
  the last reply heard and `watch_status()` follows them as they arrive.
  `DeviceStatus` carries the parsed fields and the raw JSON beside them. A
  reply carries no request id, so each device owns one watch channel and
  concurrent callers share a single request. The two accessors return `None`
  unless `lan` is enabled for the device: the recorded status belongs to that
  transport, and handing it back under another mode would be a silent
  substitution.
- `cargo run -p xtask -- catalog` generates `dist/catalog.json`, one file
  holding every device, built by CI and attached to a release. An unknown
  `schema_version` is a typed error rather than a best-effort parse.
- `cargo run -p xtask -- compat` regenerates the two tables in
  `docs/compatibility.md`; `--check` fails CI when they drift from
  `devices/*.yaml`.
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
