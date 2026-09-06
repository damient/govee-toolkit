# Changelog

Changes to `govee-toolkit`, the crate published to crates.io from
`packages/rust`. `govee-toolkit-sim` and `xtask` are workspace crates with
`publish = false`, and the same entries cover them. The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

## [Unreleased]

### Added

- `govee_toolkit::transport` — what every mode shares, moved out of the facade:
  the `Transport` trait a mode implements, the transport-neutral error, the
  circuit breaker, `DeviceStatus` and the event types. `lan` re-exports all of
  it and keeps its own richer inherent surface.
- `govee_toolkit::ble` — the `ble` transport, behind the new non-default `ble`
  feature: a scan over advertised names, one held connection per device, a paced
  write path and the same per-device circuit breaker. It sends the frames that
  the codec built. It reads a reply through the layout that the device file
  declares for it. The protocol is verified on one device, the H61A0, and on no
  other family. The device file declares Wi-Fi provisioning, and nobody has ever
  sent it to a device.
- `ble::Transport::bind` relates a device's identity to the Bluetooth address
  that a scan found. Nothing infers one from the other.
- The device file can describe a fixed-size frame and a chunked write:
  `<pad:N>` zero-fills a layout, `${name:str8}` / `${name:str16}`,
  `${name:mask8}` / `${name:mask16}` and `${name:bytes}` are new field tokens,
  and `chunk:` splits one payload across several frames. `Encoded` carries
  `frames` beside an envelope that is now optional.
- The device file can describe what a device answers with. A `reply:` layout
  matches the bytes of one reply and captures fields out of them, in the same
  grammar as `frame:` and capture-only. `frames:` lists send and reply pairs
  that the transport issues in order, so one entry reads several values. A
  captured field carries an argument `role:`. The roles `on` and `brightness`
  join the roles that a transport assembles `DeviceStatus` from, and everything
  else stays in `DeviceStatus.raw`.
- `Transport::read` and `DeviceHandle::read` return what a command's `reply:`
  layouts captured, as a map keyed by the names that the device file gave them.
  That is how a segment count, a MAC or a firmware version reaches a caller with
  no field name in this crate. `lan` refuses it, because its replies are JSON.
- The segment stream runs over any mode whose device file declares a painting
  command. A new command `role: segment_color_masked`, with argument roles
  `colors` and `zones`, describes a frame that carries one color and the zones
  that wear it. A repaint over such a frame costs one write per distinct color
  rather than one write per frame. The stream refuses three conditions when it
  opens:
  - `Zones::Native` on such a mode, because a mask names zones and reaches no
    pixel behind them;
  - a zone count larger than what the mask reaches;
  - a file that bounds its mask by nothing, through neither a `count:` nor the
    width of the mask field.
- `examples/lan_tour.rs` and `examples/ble_tour.rs` — one walkthrough per mode.
  Each example sends every command of the H61A0's table to a real device in
  order, and reads back everything that its file declares an answer for. The
  same `--all-targets` lint that CI runs compiles them, so a signature that
  changes breaks them the same day rather than leaving a stale snippet in a
  document.
- A command `role: segment_gradient`, with an argument marked `role: gradient`,
  for a mode that carries zone interpolation in a frame of its own rather than
  in the painting frame. A stream sends that command when it opens. Without it,
  `StreamOptions::gradient` encoded into nothing over such a mode, and the crate
  told a caller about a setting that the device never got. `ble` on the H61A0 is
  such a mode.
- `${name:ascii:N}` in a `reply:` layout — a text field of a given length. The
  unbounded `${name:ascii}` reads to the end, and so it also takes whatever
  follows the text. Every frame on this wire ends on a checksum, and that byte
  is neither padding nor printable. The unbounded token broke two reads of a
  firmware version, and both failed with `reply_mismatch` on every attempt.
- Error codes `native_zones_unreachable`, `zone_count_unsupported` and
  `zone_mask_unbounded` for those three refusals.
- Error codes `field_too_long`, `frame_overflow`, `chunk_syntax`, `serialize`,
  `no_envelope`, `reply_syntax`, `reply_mismatch` and, on a transport,
  `out_of_range` for an option outside the range the mode can honor — an
  out-of-range write budget is refused, never moved to the nearest value it
  could serve — and `no_reply_layout` where there is nothing to read.

### Fixed

- `ble` addressed a peripheral by its Bluetooth address, which macOS does not
  expose. `CoreBluetooth` reports every peripheral as `00:00:00:00:00:00`, so a
  scan collapsed every device on the air into one entry, and a command connected
  to whichever unrelated peripheral the adapter listed first. The transport now
  tracks a device under the handle that the platform addresses it by. It refuses
  a handle that more than one peripheral carries, rather than resolving it to
  one of them. `Advertised::address` is `Advertised::endpoint`, and
  `Transport::bind` takes the same handle.
- `ble` declared `ble::Options::connect_timeout` and never applied it. A
  peripheral that never answered left the connection pending indefinitely, and
  held every other command behind it. A connection now fails with `unreachable`
  once the timeout is up.
- The transport could send a `ble` command only while the platform still held
  the peripheral from the last scan. macOS releases that peripheral the moment a
  link drops, so an idle connection left every later command in failure until
  someone restarted the process. The transport now scans again when the handle
  is gone, and only then.
- A segment stream over a mode whose device file declares no
  `role: segment_enable` no longer fails to open. The device file declares the
  arming. A mode whose zones are always addressable has nothing to arm, and that
  is the case over `ble`.

### Changed

- A command that declares a `frame:` does not have to name it with `${frame}` in
  a `payload:`. Only a mode that wraps the frame in an envelope must name it. A
  wire that carries the frame on its own has no payload to name it in.
- The facade holds one transport per mode and looks the mode up, rather than
  matching on it in every method. `Govee::attach` takes the transports and
  refuses two that claim the same mode. `Device.lan_health` became
  `Device.health`, one entry per mode.
- `measurements.frame_rate` may be keyed by mode, with the rows it already used.
  A bare list stays the `lan` table, and `Measurements::clean_hz` now takes the
  mode: a rate measured over one channel is never carried to another.
- **Breaking, and a config file to edit.** The fallback frame rate moved out of
  `lan`: the configuration key is `stream.fallback_hz` and the field is
  `Config::stream.fallback_hz` on the new `StreamConfig`, replacing
  `LanConfig::stream_fallback_hz`. A stream picks whichever mode the device has
  enabled, so the rate is not `lan`'s to hold. The `lan` section refuses keys it
  does not know, so a file still carrying `lan.stream_fallback_hz` fails to
  load: move the key rather than deleting it, or the fallback returns to 10 Hz.

## [0.3.0] — 2026-09-06

`ble` is a mode with a transport behind it. It is opt-in and off by default, and
it does not change `lan`. The trait that the two modes now share moved several
public types. This release is therefore the breaking bump that pre-1.0 reserves
the minor for.

### Added

- `govee_toolkit::transport` — what every mode has in common: the `Transport`
  trait, the device identity, the circuit breaker, the reported status and the
  errors. None of it was ever specific to UDP; only its place in the module tree
  was. `docs/architecture.md` names this trait as the prerequisite for `ble`
  rather than something to add early, and `ble` decided its shape.
- `govee_toolkit::ble`, behind the non-default `ble` cargo feature — the GATT
  surface, a scan that reads the SKU out of the advertised name, one connection
  per device, and writes paced against a budget. The firmware does not drop a
  frame that it cannot keep up with: it gives no answer for seconds. The
  transport therefore does the pacing, so a caller that bypasses the segment
  stream still cannot cause that silence.
- `ble::Transport::bind` — this project keys a device by its Wi-Fi MAC
  everywhere, and an advertisement carries a Bluetooth address. No observation
  relates the two. The application therefore declares which is which, and this
  crate does not guess it.
- The `frame:` language gained `<pad:N>`, `${name:str8}`, `${name:str16}`,
  `${name:mask8}`, `${name:mask16}` and `${name:bytes}`, and arguments gained the
  `string`, `zones` and `bytes` types. A zone that the mask cannot carry is an
  error: the firmware drops those bits in silence, and the crate must never
  report that as success.
- `body:` with `chunk:` — a command whose payload is split across several frames,
  described in the device file rather than in code. `${count}`, `${index}` and
  `${chunk}` are reserved and supplied by the codec.
- `reply:` and `frames:` — a command can declare what an answer looks like, and
  can issue several exchanges in order. That is how one entry marked
  `role: status` reads power and brightness over `ble` without either name
  reaching this crate. `DeviceHandle::read` returns what a layout captured.
- `role: segment_color_masked` — a segment channel that paints one color per
  write over the zones a mask names, for a mode with no per-pixel channel.
  `measurements.frame_rate` is keyed by mode, so the crate paces each mode from
  what was measured on it.

### Changed

- **Breaking.** `Govee::attach` takes an iterator of `Arc<dyn Transport>` rather
  than one `lan::Transport`, and refuses two that claim the same mode, because
  one of them would never be reached.
- **Breaking.** `Encoded` is `{ cmd, message: Option<Value>, frames: Vec<Vec<u8>> }`.
  A `ble` command carries no JSON envelope, and a chunked one carries several
  frames.
- **Breaking.** `Error::Transport` and `Event` carry `transport::` types instead
  of `lan::` ones. `Unreachable` carries an `endpoint: String` — a Bluetooth
  address is not a `SocketAddr` — and `Unavailable` names the mode it refuses.
  Every event carries its mode, so an application subscribes once, whatever the
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
- Scenes. The channel is a second chunked dialect. Its header count byte does
  not follow from what was observed, and a guess would be invented verification.
  `docs/protocol/ble.md` records what is known.
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

- `govee_toolkit::codec` — the protocol codec. It reads `devices/*.yaml`,
  validates arguments and builds the exact bytes for a command, including the
  raw variable-length frame with its XOR checksum. It does no I/O, and it holds
  no SKU name, command name or argument name in the code: a `role:` on a
  command, and one on an argument, are how the SDK reaches what it issues on its
  own initiative.
- `build.rs` embeds the device catalog at build time, so an SDK is one
  artifact with no data directory to install. `GOVEE_DEVICES_DIR` overrides
  where the files are read from at build time.
- `Catalog::overlay()` — an opt-in, always-reported replacement of catalog
  entries with locally supplied device files.
- `govee_toolkit::lan` — the `lan` transport, behind the default `lan` feature:
  multicast discovery, an on-disk device cache so a command never waits for a
  scan, one reused UDP socket, fire-and-verify, and a per-device circuit breaker
  with `OK` / `DEGRADED` / `DOWN` states.
- `govee_toolkit::stream` — the raw segment channel. The crate arms it once and
  feeds it frames on a clock, and the rate measured on the unit paces that
  clock. A write never blocks: a source faster than the device replaces the
  frame that has not gone out yet, and the crate throttles nothing. The emitting
  task sends exactly one disarming frame: `close` awaits it and reports what it
  did, a drop of the handle signals it, and a fatal encoding error disarms the
  channel as the task leaves.
- `govee_toolkit::codec::capabilities` — what the hardware can do, and per mode
  which of it is out of reach and why: `transport` when somebody established the
  transport does not carry it, `unimplemented` when the file declares no command
  for it yet, `unprobed` — the default — when nobody checked. Capability names
  are data; the codec reads one of them, `segments`, and treats the rest as
  opaque strings. A parameter that it does not know makes the file fail to load,
  and the codec never ignores it.
- The facade at the crate root: configuration from
  `~/.config/govee-toolkit/config.yaml`, the enabled modes per device, the mode
  that served each command, and mode-transition events.
- The facade reports `ble` or `cloud` as unavailable when the user enables one
  of them: a mode the build carries no transport for is reported, never silently
  skipped and never substituted with another mode.
- `DeviceHandle::status()` asks the device and waits, `last_status()` returns
  the last reply heard and `watch_status()` follows them as they arrive.
  `DeviceStatus` carries the parsed fields and the raw JSON beside them. A
  reply carries no request id, so each device owns one watch channel and
  concurrent callers share a single request. The two accessors return `None`
  unless `lan` is enabled for the device: the recorded status belongs to that
  transport, and a return of it under another mode would be a silent
  substitution.
- `cargo run -p xtask -- catalog` generates `dist/catalog.json`, one file
  holding every device, built by CI and attached to a release. An unknown
  `schema_version` is a typed error rather than a best-effort parse.
- `cargo run -p xtask -- compat` regenerates the two tables in
  `docs/compatibility.md`; `--check` fails CI when they drift from
  `devices/*.yaml`.
- A codec-only build: with default features off there is no socket and no async
  runtime. `tools/check-no-io.sh` and a CI job keep it that way.
- `cargo test` fails when a command in the catalog has no conformance vector.
- Property tests over the network-facing parsers: a parser reads or drops an
  arbitrary datagram, and never panics.
- `govee-toolkit-sim` — a fake device on UDP with fault injection (silence, late
  replies, dropped replies), so CI tests the transport and the breaker without
  hardware.
- Conformance vectors under `tests/fixtures/golden/`. `cargo test` replays them,
  and also validates every `devices/*.yaml`.
- Workspace lints: `unsafe_code` forbidden, `unwrap` / `expect` / `panic` /
  `indexing_slicing` warned in library code, clippy `pedantic` and `cargo`
  groups on.
