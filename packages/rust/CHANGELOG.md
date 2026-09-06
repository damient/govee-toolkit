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
  paced write path and the same per-device circuit breaker. It sends the frames
  the codec built and reads a reply through the layout the device file declares
  for it. The protocol is verified on one device, the H61A0, and on no other
  family; Wi-Fi provisioning is declared in the device file but has never been
  sent to a device.
- `ble::Transport::bind` relates a device's identity to the Bluetooth address a
  scan heard. Nothing infers one from the other.
- The device file can describe a fixed-size frame and a chunked write:
  `<pad:N>` zero-fills a layout, `${name:str8}` / `${name:str16}`,
  `${name:mask8}` / `${name:mask16}` and `${name:bytes}` are new field tokens,
  and `chunk:` splits one payload across several frames. `Encoded` carries
  `frames` beside an envelope that is now optional.
- The device file can describe what a device answers with. A `reply:` layout
  matches the bytes of one reply and captures fields out of them, in the same
  grammar as `frame:` and capture-only; `frames:` lists send/reply pairs issued
  in order, so one entry reads several values. A captured field carries an
  argument `role:`, and `on` and `brightness` join the roles a transport
  assembles `DeviceStatus` from — everything else stays in `DeviceStatus.raw`.
- `Transport::read` and `DeviceHandle::read` return what a command's `reply:`
  layouts captured, as a map keyed by the names the device file gave them. That
  is how a segment count, a MAC or a firmware version reaches a caller with no
  field name in this crate. `lan` refuses it: its replies are JSON.
- The segment stream runs over any mode whose device file declares a painting
  command. A new command `role: segment_color_masked`, with argument roles
  `colors` and `zones`, describes a frame that carries one color and the zones
  wearing it: a repaint over it costs one write per distinct color rather than
  one per frame. Three refusals land when the stream opens: `Zones::Native` on
  such a mode, since a mask names zones and reaches no pixel behind them; a zone
  count past what the mask reaches; and a file that bounds its mask by nothing,
  through neither a `count:` nor the width of the mask field.
- Error codes `native_zones_unreachable`, `zone_count_unsupported` and
  `zone_mask_unbounded` for those three refusals.
- Error codes `field_too_long`, `frame_overflow`, `chunk_syntax`, `serialize`,
  `no_envelope`, `reply_syntax`, `reply_mismatch` and, on a transport,
  `out_of_range` for an option outside the range the mode can honour — an
  out-of-range write budget is refused, never moved to the nearest value it
  could serve — and `no_reply_layout` where there is nothing to read.

### Changed

- A command declaring a `frame:` no longer has to name it with `${frame}` in a
  `payload:`. That is required of a mode that wraps the frame in an envelope; a
  wire carrying the frame on its own has no payload to name it in.
- The facade holds one transport per mode and looks the mode up, rather than
  matching on it in every method. `Govee::attach` takes the transports and
  refuses two claiming the same mode. `Device.lan_health` became
  `Device.health`, one entry per mode.
- `measurements.frame_rate` may be keyed by mode, with the rows it already
  used. A bare list stays the `lan` table, and `Measurements::clean_hz` now
  takes the mode: a rate measured over one channel is never carried to another.
- **Breaking, and a config file to edit.** The fallback frame rate moved out of
  `lan`: the configuration key is `stream.fallback_hz` and the field is
  `Config::stream.fallback_hz` on the new `StreamConfig`, replacing
  `LanConfig::stream_fallback_hz`. A stream picks whichever mode the device has
  enabled, so the rate is not `lan`'s to hold. The `lan` section refuses keys it
  does not know, so a file still carrying `lan.stream_fallback_hz` fails to
  load: move the key rather than deleting it, or the fallback returns to 10 Hz.

## [0.3.0] — 2026-09-06

`ble` is a mode with a transport behind it. It is opt-in, off by default, and
`lan` is untouched by it — but the trait the two now share moved several public
types, so this is the breaking bump pre-1.0 reserves the minor for.

### Added

- `govee_toolkit::transport` — what every mode has in common: the `Transport`
  trait, the device identity, the circuit breaker, the reported status and the
  errors. None of it was ever specific to UDP; only its address in the tree was.
  `docs/architecture.md` named this trait as the prerequisite for `ble` rather
  than something to add early, and `ble` is what decided its shape.
- `govee_toolkit::ble`, behind the non-default `ble` cargo feature — the GATT
  surface, a scan that reads the SKU out of the advertised name, one connection
  per device, and writes paced against a budget. The firmware does not drop a
  frame it cannot keep up with: it stops answering for seconds. Pacing therefore
  lives in the transport, so a caller that bypasses the segment stream still
  cannot provoke that.
- `ble::Transport::bind` — a device is keyed by its Wi-Fi MAC everywhere in this
  project, and an advertisement carries a Bluetooth address. Nothing observed
  relates the two, so the application says which is which rather than this crate
  guessing.
- The `frame:` language gained `<pad:N>`, `${name:str8}`, `${name:str16}`,
  `${name:mask8}`, `${name:mask16}` and `${name:bytes}`, and arguments gained the
  `string`, `zones` and `bytes` types. A zone the mask cannot carry is an error:
  the firmware drops those bits in silence, which is exactly what must not be
  reported as success.
- `body:` with `chunk:` — a command whose payload is split across several frames,
  described in the device file rather than in code. `${count}`, `${index}` and
  `${chunk}` are reserved and supplied by the codec.
- `reply:` and `frames:` — a command may declare what an answer looks like, and
  may issue several exchanges in order. That is how one entry marked
  `role: status` reads power and brightness over `ble` without either name
  reaching this crate. `DeviceHandle::read` returns what a layout captured.
- `role: segment_color_masked` — a segment channel that paints one colour per
  write over the zones a mask names, for a mode with no per-pixel channel.
  `measurements.frame_rate` is keyed by mode so each one is paced from what was
  measured on it.

### Changed

- **Breaking.** `Govee::attach` takes an iterator of `Arc<dyn Transport>` rather
  than one `lan::Transport`, and refuses two claiming the same mode: one of them
  would never be reached.
- **Breaking.** `Encoded` is `{ cmd, message: Option<Value>, frames: Vec<Vec<u8>> }`.
  A `ble` command carries no JSON envelope, and a chunked one carries several
  frames.
- **Breaking.** `Error::Transport` and `Event` carry `transport::` types instead
  of `lan::` ones. `Unreachable` carries an `endpoint: String` — a Bluetooth
  address is not a `SocketAddr` — and `Unavailable` names the mode it refuses.
  Every event carries its mode, so an application subscribes once whatever the
  build carries.
- **Breaking.** `Device.lan_health` is `Device.health`, one entry per enabled
  mode a transport knows the device in.
- `lan` re-exports the moved types, so `crate::lan::DeviceId` and its neighbours
  still resolve, and keeps its own richer surface: a `lan` caller still gets the
  address and the four firmware strings a scan reply carries.
- The published crate description says "over the LAN or Bluetooth".

### Not in this release

- Wi-Fi provisioning over `ble` is encoded and pinned by conformance vectors,
  and **has never been sent to a device**. The vectors say so in their `source`;
  they pin the encoder, not the firmware.
- Scenes. The channel is a second chunked dialect whose header count byte does
  not follow from what was observed, and guessing it would be inventing
  verification. `docs/protocol/ble.md` records what is known.
- No `ble` capture is committed yet, so every `capture:` in the H61A0's `ble`
  table is empty.

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
