# Changelog

Changes to `govee-toolkit`, the crate published to crates.io from
`packages/rust`. `govee-toolkit-sim` and `xtask` are workspace crates with
`publish = false`, and the same entries cover them. `govee-toolkit-cli`
releases apart and keeps
[its own changelog](crates/cli/CHANGELOG.md). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

## [Unreleased]

### Added

- `DeviceHandle::power`, `DeviceHandle::brightness` and `DeviceHandle::color` —
  the commands a person names, reached through the new `power`, `brightness`
  and `color` roles. Each reads the entry the device file marks with that role
  and fills the arguments it marks with an argument role, so no command name
  and no argument name lives in this crate. A mode whose file claims no such
  entry fails with `Error::NoRoleCommand`; nothing is approximated through
  another command.
- `Role::Power`, `Role::Brightness` and `Role::Color`, and the argument roles
  `ArgRole::Red`, `ArgRole::Green` and `ArgRole::Blue`. `cargo test` refuses a
  file where two entries of one mode claim the same role, or where a claiming
  entry marks no argument for it.
- `DeviceHandle::segment` — paint zones one color. A zone list paints those
  zones and leaves the rest alone, which needs `role: segment_color_masked`:
  a `role: segment_color` frame states the color of every zone, and this crate
  does not hold what the other zones show, so it refuses the subset rather
  than repaint them. No list paints every zone, over whichever painting role
  the file marks, from `capabilities.segments.count`. The channel is armed
  where the file marks `role: segment_enable`, and nothing disarms it. A
  gradient the file can carry nowhere is refused rather than dropped.
- `DeviceHandle::serving_mode` — the mode a command sent now would go over,
  from the same recorded state a send reads. What a caller needs to read the
  device file for the right mode before it builds arguments.
- `DeviceHandle::spec` — what `devices/<SKU>.yaml` declares for this device.
  Reads no hardware.
- `Support` implements `Display`, so a mode's support level prints as the word
  the device file carries.
- A `cloud` `payload:` resolves `${<name>:rgb24}`: the one triple of an
  `rgb_list` argument, packed into 0xRRGGBB. A list of any other length fails
  with `Error::OutOfRange`.

## [0.5.0] — 2026-09-10

`cloud` is a mode with a transport behind it. It is opt-in and off by default,
and it changes neither `lan` nor `ble`. The codec grew the fields that mode
needs, so this release is the breaking bump that pre-1.0 reserves the minor
for.

### Added

- `govee_toolkit::cloud`, behind the non-default `cloud` cargo feature — the
  documented HTTPS API as a `Transport`. It lists the account's devices, writes
  one capability per command, reads a state back, and keeps one request per
  device per `min_interval`. A command that must wait longer than `max_wait`
  fails with `Error::RateLimited` rather than queue out of sight. The API says
  whether it accepted a command, so this mode sends no verification request
  after a write. Its key comes from `GOVEE_API_KEY` or from the file
  `cloud.key_file` names, never from `config.yaml`. `examples/cloud_tour.rs`
  runs the whole table against a real account. See
  [`../../docs/protocol/cloud.md`](../../docs/protocol/cloud.md).
- `CloudConfig`, the `cloud:` section of the configuration — where the API
  lives, the request timeout, the throttle and the breaker thresholds. A build
  that finds no key starts without the mode, and a device that enables it
  fails with `Error::MissingCredential`. `config::KEY_ENV` names the
  environment variable.
- `Error::MissingCredential`, with the code `missing_credential` — a mode the
  configuration enables whose credential the configuration does not carry. It
  names what to set. `Config::missing_credential` answers the same question
  for one mode, and a mode that needs no credential answers `None`.
- `codec::cloud` — `Capability`, `Read` and `Request`: what a cloud entry
  declares beyond its value. `Encoded::request` carries it to the transport.
- `ArgRole::Color` and `ArgRole::ColorTemp`, so a status answer reaches
  `DeviceStatus::color` and `DeviceStatus::color_temp_kelvin` without a
  capability name in code.
- `transport::Error::Api` and `transport::Error::RateLimited`, with the codes
  `api` and `rate_limited`.
- `${r,g,b:rgb24}` in a `payload:` template — three channel arguments packed
  into `0xRRGGBB`, which is how the cloud API carries a color. The three keep
  the names the frame layouts give them, so `color` takes the same arguments
  over every mode.
- A `zones` argument in a `payload:` template becomes an array of zone
  indices, ascending and each index once, which is how the cloud API names the
  zones a capability writes. A frame layout carries the same argument as a
  mask, so one call names the same zones over every mode.
- `${name:mask32}` in a `frame:` layout — a zone mask four bytes wide, least
  significant bit first, reaching zone 31. It agrees with `${name:mask16}` on
  every zone below 16, so widening the field leaves the bytes a device file
  already sent unchanged. A zone past the width of its mask is still
  `codec::Error::OutOfRange`: the firmware drops such a bit in silence, and a
  saturated mask looks exactly like an ignored one.
- `Govee::shutdown()` — release what every transport holds, before the program
  ends. Over `ble` it holds each open link open long enough for the last frame
  to leave, since that wire acknowledges nothing. Every other mode does
  nothing here. A transport serves commands again afterwards, at the cost of a
  new connection.
- `Transport::close()`, with a default that does nothing — the same seam for
  an implementation of the trait. `ble::Transport::close()` overrides it.
- `ble::Options::write_drain` — how long `close()` holds a link open for a unit
  whose device file records no `measurements.ble.write_drain_ms`. 50 ms by
  default. A device file that records the field wins over it.
- `codec::measurements::Ble::write_drain_ms`, read off each device file.

### Changed

- **Breaking:** a mode that this build carries but that has no credential
  fails with `Error::MissingCredential` rather than `Error::ModeNotImplemented`.
  A caller that matches `ModeNotImplemented` to detect a missing API key
  matches the new variant instead. `ModeNotImplemented` keeps its meaning: the
  build carries no transport for the mode.
- **Breaking:** `codec::Encoded` gains the `request` field. `codec::encode`
  fills it in; a struct literal that builds one by hand needs the new field.
- **Breaking:** `codec::ArgRole` gains two variants. A match over it needs an
  arm for each, or a wildcard.
- `Encoded::cmd` carries the capability instance over `cloud`, and still the
  `msg.cmd` over `lan`. It stays empty where the wire carries no name.
- `DeviceStatus::from_captured` fills `color` and `color_temp_kelvin` from a
  captured argument that carries `ArgRole::Color` or `ArgRole::ColorTemp`. It
  answered `None` for both. Every mode assembles a status from the same
  roles, so `cloud` and a frame wire report a color the same way. No device
  file declares those roles on a `reply:` today.

### Fixed

- A command written over `ble` reaches the device when the program ends right
  after it. The write characteristic refuses a write with a response, so a
  frame carries no acknowledgement, and a link dropped too early loses it in
  silence. Call `Govee::shutdown()` before the program ends. See
  [`../../docs/protocol/ble.md`](../../docs/protocol/ble.md) §1.4.
- `Transport::last_status` reports a status that arrived while nothing was
  watching the device. The value is replaced rather than sent, so a mode that
  publishes a status outside a `status()` call keeps it.

## [0.4.0] — 2026-09-09

The device schema drops a key and the crate drops the public field that carried
it, so this is the breaking bump that pre-1.0 reserves the minor for. The
published documentation covers every feature.

### Changed

- docs.rs builds the crate with `--all-features`, so `ble` and the crate-root
  link to it are on the published pages, and every item carries a label naming
  the feature that gates it. `[package.metadata.docs.rs]` sets it, and CI builds
  the docs the same way so the published build is checked before a release
  rather than after one.

### Removed

- **Breaking:** `codec::Command::capture`. Read the provenance of a command's
  bytes from its conformance vector's `source`, under `tests/fixtures/golden/`.
  A device file declares no capture path.

## [0.3.0] — 2026-09-08

`ble` is a mode with a transport behind it. It is opt-in and off by default, and
it does not change `lan`. The trait that the two modes now share moved several
public types. This release is therefore the breaking bump that pre-1.0 reserves
the minor for.

### Added

- `govee_toolkit::transport` — what every mode has in common: the `Transport`
  trait, the device identity, the circuit breaker, the reported status and the
  errors. None of it was ever specific to UDP; only its place in the module tree
  was. `docs/architecture.md` names this trait as the prerequisite for `ble`
  rather than something to add early, and `ble` decided its shape. `lan`
  re-exports all of it and keeps its own richer inherent surface.
- `govee_toolkit::ble`, behind the non-default `ble` cargo feature — the GATT
  surface, a scan that reads the SKU out of the advertised name, one held
  connection per device, writes paced against a budget, and the same per-device
  circuit breaker. It sends the frames that the codec built, and it reads a
  reply through the layout that the device file declares. The firmware does not
  drop a frame that it cannot keep up with: it gives no answer for seconds. The
  transport therefore does the pacing, so a caller that bypasses the segment
  stream still cannot cause that silence. The protocol is verified on the H61A0
  and the H6114, and on no other family.
- `govee_toolkit::ble::wire` — the seam between the protocol and a Bluetooth
  stack, as an `Adapter` and a `Peripheral` trait. `ble::Radio` implements them
  over the platform's radio, and `ble::Transport::with_adapter` takes another
  one. Nothing above the seam names `btleplug`.
- `ble::Transport::bind` — this project keys a device by its Wi-Fi MAC
  everywhere, and an advertisement carries a Bluetooth address. No observation
  relates the two. The application therefore declares which is which, and this
  crate does not guess it. The handle a scan reports is `Advertised::endpoint`,
  the handle that the platform addresses the peripheral by: macOS reports every
  Bluetooth address as `00:00:00:00:00:00` and never exposes the real one. The
  transport refuses a handle that more than one peripheral carries, rather than
  resolving it to one of them.
- `DeviceHandle::provision_wifi` puts a device on a Wi-Fi network over `ble`,
  which is how a device out of the box becomes reachable over `lan`. It takes
  `WifiCredentials`: the network, the password and the device's UTC offset as
  hours and minutes. The SDK does not read the host clock, and what a negative
  offset looks like on the wire was never observed. The password travels in
  plaintext, so anything in Bluetooth range during provisioning reads it. `ble`
  must be enabled for the device, as for every other call. The call reports what
  the writes did: the device answers a status byte, the codec reads no reply on
  a chunked command, and whether the device joined is what says it worked.
- Four command roles — `wifi_link`, `wifi_api_type`, `wifi_provision` and
  `wifi_provision_with_api` — and the argument roles that fill them. The bytes
  stay in the device file and the order lives in the crate: wake the module,
  wait, transfer, release. A device file tags its entries, so a family whose
  frames differ changes its file and no code.
- A device file can pull in a shared command table with `include:`, naming a
  fragment under `devices/families/`. `Catalog::from_sources_with` takes the
  device files and the fragments; `Catalog::embedded` and an overlay resolve
  against what the build shipped. The merge happens on load, so `Device` and
  everything reading it see one flat table. `Error::UnknownFamily` and
  `Error::DuplicateCommand` are the two ways it refuses.
- The `frame:` language gained `<pad:N>`, `${name:str8}`, `${name:str16}`,
  `${name:mask8}`, `${name:mask16}` and `${name:bytes}`, and arguments gained the
  `string`, `zones` and `bytes` types. A zone that the mask cannot carry is an
  error: the firmware drops those bits in silence, and the crate must never
  report that as success. Padding that is not at the end of a frame is refused.
- `body:` with `chunk:` — a command whose payload is split across several
  frames, described in the device file rather than in code. `${count}`,
  `${index}` and `${chunk}` are reserved and supplied by the codec. A second way
  of cutting a body rides the same block: `head_size:` puts the first slice in
  the header, a footer that reads `${chunk:bytes}` carries the last one, `then:`
  sends one frame after the transfer, and `${total}` is the frame count of the
  whole transfer beside the `${count}` of data frames. This is what the `0xA3`
  channel of `docs/protocol/ble.md` 6 needs; a body cut the way `0xA1` cuts it
  is unchanged.
- `reply:` and `frames:` — a command can declare what an answer looks like, and
  can issue several exchanges in order. A `reply:` layout matches the bytes of
  one reply and captures fields out of them, in the same grammar as `frame:` and
  capture-only. That is how one entry marked `role: status` reads power and
  brightness over `ble` without either name reaching this crate. A captured
  field carries an argument `role:`; the roles `on` and `brightness` join the
  roles that a transport assembles `DeviceStatus` from, and everything else
  stays in `DeviceStatus.raw`.
- `Transport::read` and `DeviceHandle::read` return what a command's `reply:`
  layouts captured, as a map keyed by the names that the device file gave them.
  That is how a segment count, a MAC or a firmware version reaches a caller with
  no field name in this crate. `lan` refuses it, because its replies are JSON.
- `${name:ascii:N}` in a `reply:` layout — a text field of a given length. The
  unbounded `${name:ascii}` reads to the end, and so it also takes whatever
  follows the text. Every frame on this wire ends on a checksum, and that byte
  is neither padding nor printable.
- `role: segment_color_masked`, with argument roles `colors` and `zones` — a
  segment channel that paints one color per write over the zones a mask names,
  for a mode with no per-pixel channel. A repaint over such a frame costs one
  write per distinct color rather than one write per frame.
  `measurements.frame_rate` may be keyed by mode, with the rows it already used,
  so the crate paces each mode from what was measured on it. A bare list stays
  the `lan` table, and `Measurements::clean_hz` takes the mode: a rate measured
  over one channel is never carried to another. The stream refuses three
  conditions when it opens:
  - `Zones::Native` on such a mode, because a mask names zones and reaches no
    pixel behind them;
  - a zone count larger than what the mask reaches;
  - a file that bounds its mask by nothing, through neither a `count:` nor the
    width of the mask field.
- A command `role: segment_gradient`, with an argument marked `role: gradient`,
  for a mode that carries zone interpolation in a frame of its own rather than
  in the painting frame. A stream sends that command when it opens. Without it,
  `StreamOptions::gradient` encoded into nothing over such a mode, and the crate
  told a caller about a setting that the device never got. `ble` on the H61A0 is
  such a mode. The arming is the device file's too: a mode whose zones are
  always addressable declares no `role: segment_enable`, and a stream over it
  opens with nothing to arm.
- `govee_toolkit_sim::ble` — a fake peripheral on GATT and the radio that finds
  it. It takes one connection, refuses a frame whose length or BCC is wrong,
  answers a write and answers a read the test registered. Beside the faults the
  `lan` device carries, it refuses a connection and it stalls under a burst. The
  `ble` transport is thus tested end to end on a machine with no Bluetooth: see
  `tests/ble_sim.rs`.
- `examples/lan_tour.rs` and `examples/ble_tour.rs` — one walkthrough per mode.
  Each example sends every command of the H61A0's table to a real device in
  order, and reads back everything that its file declares an answer for. The
  same `--all-targets` lint that CI runs compiles them, so a signature that
  changes breaks them the same day rather than leaving a stale snippet in a
  document.
- `Measurements::ble` types the `measurements.ble` block, exported as
  `codec::BleMeasurements`. The fields the block carries are
  `read_round_trip_ms`, `sustained_writes_hz`, `write_budget_hz`,
  `burst_frames_before_stall`, `burst_recovery_s` and `addressable_zones`, and
  anything else a file records stays in `BleMeasurements::extra`.
  `codec::validate` refuses a `write_budget_hz` that is not finite and above
  zero, or that is above the `sustained_writes_hz` beside it.
- Error codes `native_zones_unreachable`, `zone_count_unsupported` and
  `zone_mask_unbounded` for those three refusals.
- Error codes `field_too_long`, `frame_overflow`, `chunk_syntax`, `serialize`,
  `no_envelope`, `reply_syntax`, `reply_mismatch` and, on a transport,
  `out_of_range` for an option outside the range the mode can honor — an
  out-of-range write budget is refused, never moved to the nearest value it
  could serve — and `no_reply_layout` where there is nothing to read.

### Changed

- **Breaking.** `Govee::attach` takes an iterator of `Arc<dyn Transport>` rather
  than one `lan::Transport`, and refuses two that claim the same mode, because
  one of them would never be reached. The facade holds one transport per mode
  and looks the mode up, rather than matching on it in every method.
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
- **Breaking.** `Transport` gained `scan_window`, which answers how long a scan
  on that mode must listen. `Govee::scan` asks each transport for its own window
  instead of carrying `lan.scan_window_ms` to every mode: `lan` waits for
  replies to a request it sent, `ble` waits for advertisements that arrive on
  each device's own interval, and one mode's window reported a device that was
  there as absent on another. `lan::Transport::scan_window` answers
  `lan::Options::scan_window`, and the new `ble::Options::scan_window` is the
  `ble` one. `Transport::scan` still takes a window, for a caller that wants a
  shorter or a longer one deliberately. `Govee::scan` spends the windows at the
  same time, so a scan over several modes costs the longest window and not the
  sum of them.
- **Breaking.** `ble::Transport::start` and `ble::Transport::with_adapter` take
  the `&Catalog` the caller builds the facade with. The transport reads the
  write budgets from it, so a caller that binds its own adapter paces each
  device at the same rate the facade does.
- **Breaking.** `Verify::With` carries an `Arc<Encoded>` and `Verify` has no
  lifetime parameter. A verified send gives the request to the task that runs
  the verification. A shared request costs one pointer copy, where a borrowed
  one made the task copy the frames of every command.
- **Breaking, and a config file to edit.** The fallback frame rate moved out of
  `lan`: the configuration key is `stream.fallback_hz` and the field is
  `Config::stream.fallback_hz` on the new `StreamConfig`, replacing
  `LanConfig::stream_fallback_hz`. A stream picks whichever mode the device has
  enabled, so the rate is not `lan`'s to hold. The `lan` section refuses keys it
  does not know, so a file still carrying `lan.stream_fallback_hz` fails to
  load: move the key rather than deleting it, or the fallback returns to 10 Hz.
- The `ble` transport paces each device at the budget its own device file
  records. `ble::Budgets::from_catalog` reads
  `measurements.ble.write_budget_hz` for every SKU a catalog carries, verified
  aliases included. A device whose file records no budget is written at
  `ble::Options::writes_per_second`, the one rate anybody measured. The burst
  stays `ble::Options::burst` for every device:
  `measurements.ble.burst_frames_before_stall` records the count that stalled a
  unit, which is not a count that is safe. A device file recording a rate no
  write could go out under fails `ble::Transport::start` with `out_of_range`.
- The `ble` send path locks per device. A command waits for its own device's
  connection and for nothing else, where one lock over the whole table made a
  command to a reachable device wait out another device's scan and connection.
  Nothing probes the link before a write either: a connection the device dropped
  is found by the write that fails on it, so the first command after a device
  goes away returns `io` and the next one opens a connection. The transport
  scans again when the platform no longer holds the peripheral, and only then;
  a connection that never answers fails with `unreachable` once
  `ble::Options::connect_timeout` is up.
- The send path repeats less work. `ble::Radio` resolves the write and the
  notify characteristic once, when discovery runs, rather than on every frame.
  `Catalog::device` and the `ble` budget lookups take an uppercase SKU without
  allocating one. A chunked command clones the caller's arguments once, not once
  per slice.
- `codec::chunk::Chunk::then` is an `Option<String>`, and
  `codec::chunk::Layout::frames` returns an iterator. A `chunk:` block that
  declares no `then:` is `None` rather than an empty string.
- A command that declares a `frame:` does not have to name it with `${frame}` in
  a `payload:`. Only a mode that wraps the frame in an envelope must name it. A
  wire that carries the frame on its own has no payload to name it in.
- `lan` re-exports the moved types, so `crate::lan::DeviceId` and its neighbours
  still resolve, and keeps its own richer surface: a `lan` caller still gets the
  address and the four firmware strings a scan reply carries.
- The published crate description says "over the LAN or Bluetooth".

### Not in this release

- Scenes. The channel is a second chunked dialect. Its header count byte does
  not follow from what was observed, and a guess would be invented verification.
  `docs/protocol/ble.md` records what is known.
- No `ble` capture is committed yet, so every `capture:` in the H61A0's `ble`
  table is empty. That is a statement about the evidence this repository holds,
  not about whether the commands work.
- Wi-Fi provisioning put a unit on a network with the entry that carries the API
  block. The entry without that block was never accepted by a device, and no
  provisioning vector's bytes come from a capture: they pin the encoder, and
  each one's `source` says which is which.

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
