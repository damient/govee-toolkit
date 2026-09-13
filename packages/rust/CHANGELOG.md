# Changelog

Changes to `govee-toolkit`, the crate published to crates.io from
`packages/rust`. `govee-toolkit-sim` and `xtask` are workspace crates with
`publish = false`, and the same entries cover them. `govee-toolkit-cli`
releases apart and keeps
[its own changelog](crates/cli/CHANGELOG.md). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

### Added

- `ble::encode` and `ble::session` — the encoded `ble` link of
  [`docs/protocol/ble.md`](../../docs/protocol/ble.md) 9. A device that sets the
  advertisement's encoding flag runs a session-seed handshake, then the
  transport encodes every frame under it; a plaintext frame gets no answer.
- `ble::Beacon` reads the advertisement data: the encoding flag, the layout
  version, `pactType` and `pactCode`. See `docs/protocol/ble.md` 1.5.
- `scan::sku_of` reads a `GV<SKU><4 hex digits>` advertised name, so `ble`
  discovers a device of that family. `GVH` and `GVR` join `NAME_PREFIXES`.
- `Support::Capped` — a mode that reaches every capability the transport
  carries, where the transport carries less than the hardware has.
- `<sum>` in a `frame:` layout — the low byte of the sum of the bytes before
  it. A layout carries `<sum>` or `<xor>`, never both.
- `ble::HOST_COLOR_PROTYPE` — the `0xA5` frames of
  [`docs/protocol/ble.md`](../../docs/protocol/ble.md) 8. The `ble` wire
  carries them at their own length, shorter than `FRAME_LEN`.
- `measurements.ble.render_hold_ms` — how long a firmware holds a colour from
  the host before it returns to its stored state.
- `Provisioned` — what `provision_wifi()` reports: `Accepted` where the device
  acknowledged the transfer, `Sent` where its file declares no acknowledgement.
- `Error::ProvisionRefused` — a device answered a provisioning transfer with a
  status other than `0`. It holds the network it had.
- `ArgRole::AckStatus` — the status a chunked transfer is acknowledged with,
  captured by the `reply:` in its `chunk:` block.
- `codec::Encoded::exchanges()` — every frame, with the layout that reads its
  answer where the command declares one.
- `govee_toolkit_sim::ble::BleDevice::set_transfer_status()` — what the
  simulated device acknowledges a chunked transfer with.

### Changed

- **Breaking:** `provision_wifi()` returns `Provisioned` rather than `()`.
  `Provisioned::Accepted` says the device took the credentials; it says nothing
  about the network.
- **Breaking:** `ArgRole` carries a further variant. Match `ArgRole::AckStatus`
  wherever a match on it is exhaustive.
- A chunked command whose `chunk:` block declares a `reply:` is read back: the
  `ble` transport writes every frame, then waits for the one answer the device
  sends after the last.
- **Breaking:** `ble::Options` carries `chunk_gap`, the wait between two frames
  of a chunked command. `Options::default` sets it to 300 ms.
- The `ble` send path waits `chunk_gap` between the frames of a chunked
  command. A firmware drops a Wi-Fi transfer that arrives at the write budget
  and answers nothing. Single-frame commands keep their pace.
- **Breaking:** `ble::wire::Heard` carries `adverts`, and `scan::Advertised`
  carries `beacon`. Construct each with the new field; `Advertised::heard` reads
  both off one advertisement.
- **Breaking:** `ble::Options` carries `handshake_timeout`, how long each step
  of the handshake waits. `Options::default` sets it.
- **Breaking:** `NAME_PREFIXES` is `[&str; 6]`. Bind it as a slice,
  `&NAME_PREFIXES`, rather than as an array of a fixed length.
- **Breaking:** `Support` carries a fifth variant. Match `Support::Capped`
  wherever a match on it is exhaustive.
- Catalog validation reads the level off the reasons: a mode whose every
  unreachable capability is `transport` must be `capped`, and one that names an
  `unimplemented` or `unprobed` capability must be `partial`.

### Removed

- **Breaking:** `codec::Command` drops `documented`. Nothing read it but one
  validation rule, which is gone with it.

## [0.6.0] — 2026-09-12

A run reads its `GOVEE_*` variables from a `.env` file, and one painting
states a color per zone. Both change public signatures: `CloudConfig::key`
reads through an `Env`, and `stream::Zones` is `stream::Resolution`. This
release is therefore the breaking bump that pre-1.0 reserves the minor for.

### Added

- `Env` and `Config::env` — the `GOVEE_*` variables one run reads.
  `Config::load` reads the environment first, then the first `.env` it finds.
- The `.env` search starts in the working directory and goes up to `.git` or the
  home directory, and reads `~/.config/govee-toolkit/.env` last.
- `GOVEE_ENV_FILE` names one file and replaces the search. A file the search
  does not find is not an error: `lan` and `ble` need no credential.
- `Env::process`, `Env::from_file` and `Env::from_pairs` build one without the
  search, and `Config::load_from_with` takes it.
- `Config::with_env` replaces the variables of a configuration already loaded.
- `Error::Env`, code `env`, for a file that cannot be read or does not parse. A
  file `GOVEE_ENV_FILE` or `Env::from_file` names is an error when it is absent.
- `DeviceHandle::gradient` — set whether the firmware interpolates between
  zones, without painting, through the `segment_gradient` role.
- A mode that carries the gradient inside its painting frame fails with
  `Error::NoRoleCommand`. Pass `Paint::gradient` there, which sets both at once.
- `Measurements::arm_settle_ms` and `Measurements::arm_settle` — the delay the
  firmware needs after the arming frame of the segment channel, in milliseconds.
- `arm_settle` answers what the device file records, or a conservative default
  that is never zero: a paint sent too early is dropped in silence.
- `DeviceHandle::power`, `DeviceHandle::brightness` and `DeviceHandle::color` —
  the commands a person names, reached through the roles of the same names.
- Each role reads the entry the device file marks and fills the arguments it
  marks, so no command name and no argument name lives in this crate.
- A mode whose file claims no entry for a role fails with
  `Error::NoRoleCommand`. Nothing is approximated through another command.
- `Role::Power`, `Role::Brightness` and `Role::Color`, and the argument roles
  `ArgRole::Red`, `ArgRole::Green` and `ArgRole::Blue`.
- `cargo test` refuses a file where two entries of one mode claim the same role,
  or where a claiming entry marks no argument for it.
- `DeviceHandle::color_temp` — set the white temperature, in kelvin, through the
  `color_temp` role. It ends the color the device showed.
- Where the file marks the white components, `color_temp` renders the
  temperature and fills them, because that firmware renders nothing itself.
- Where the file marks a zone mask, `color_temp` fills every zone the mask can
  name. A mask bounded by nothing fails with `Error::ZoneMaskUnbounded`.
- `codec::white::rgb` — the RGB rendering of a temperature, sampled every 500 K
  off the Planckian locus. It is not the vendor's rendering.
- `Role::ColorTemp`, and the argument roles `ArgRole::WhiteRed`,
  `ArgRole::WhiteGreen` and `ArgRole::WhiteBlue`.
- `cargo test` refuses a `color_temp` entry that declares one or two of the
  three white components.
- `DeviceHandle::segment` — paint zones. No zone list paints every zone the
  frame reaches, over whichever painting role the file marks.
- A zone list paints those zones and leaves the rest alone, which needs
  `role: segment_color_masked`. This crate holds no color to repaint.
- `segment` arms the channel where the file marks `role: segment_enable`, and
  nothing disarms it. A gradient the file carries nowhere is refused.
- `DeviceHandle::music` and `Music` — play an effect the device renders from its
  own microphone, through the `music` role. The host sends nothing per beat.
- `Music` carries the effect, the sensitivity, the soft rendering and the color
  to impose; the SDK fills each where the entry declares the argument.
- An effect identifier a file accepts is not one the device renders. No entry
  stops an effect, so a caller ends it with `power`, `color` or `color_temp`.
- `Role::Music`, and the argument roles `ArgRole::Effect`,
  `ArgRole::Sensitivity`, `ArgRole::Soft` and `ArgRole::ColorMode`.
- `ArgRole::Red`, `ArgRole::Green` and `ArgRole::Blue` name the color a `music`
  entry imposes as well as the one a `color` entry sets.
- `DeviceHandle::serving_mode` — the mode a command sent now would go over, from
  the same recorded state a send reads.
- `DeviceHandle::spec` — what `devices/<SKU>.yaml` declares for this device.
  Reads no hardware.
- `Support` implements `Display`, so a mode's support level prints as the word
  the device file carries.
- `Govee::scan_on` — run a discovery scan on the modes named, and nothing on the
  others. A mode with no transport contributes nothing and is not an error.
- A `cloud` `payload:` resolves `${<name>:rgb24}`: one `rgb_list` triple, packed
  into 0xRRGGBB. Any other length fails with `Error::OutOfRange`.
- `Paint` — what one painting states: the zones, the colors, the zone count and
  the gradient. `DeviceHandle::segment` takes it.
- One color paints every zone, and a longer list states one zone each, which is
  how a mode with a per-LED channel reaches one LED.
- Only a `role: segment_color` entry states a color per zone; a masked entry
  groups the zones that share a color, one frame each.
- A color list that is neither one color nor one per zone fails with
  `Error::ColorCountMismatch`.
- A zone list with more than one color fails with `Error::ZoneListColorCount`.
- `measurements.resolution_changepoints` — every zone count at which one unit
  refines, read by `Measurements::renders_as`.
- A `Resolution::Exact` count the unit renders as a smaller one fails with
  `Error::ResolutionNotDistinct`, which names the counts the file records.
- `stream::reach` and `Reach` — how many zones one mode paints on one device,
  and whether that reaches every addressable LED. Reads no hardware.
- `Govee::ensure_known` — make one device reachable before the first command. It
  scans only where a scan is needed, over the modes that device enables.
- `ensure_known` looks over those modes at the same time and answers the first
  mode in that list. See [`../../docs/modes.md`](../../docs/modes.md).

### Changed

- `paths::config_file_from` reads `GOVEE_CONFIG` from an `Env`, so `.env` can
  name the configuration file. `paths::config_file` reads the environment alone.
- `CloudConfig::key` and `CloudConfig::transport_options` take the `Env` to read
  through. The environment wins over `.env`, and `.env` over `cloud.key_file`.
- Only `GOVEE_*` names are read, and a blank value counts as a placeholder.
- The `.env` values never reach the process environment, so nothing a process
  launches inherits the key and a test needs no process-wide variable.
- `stream::Zones` is `stream::Resolution`, and `StreamOptions::zones` is
  `StreamOptions::resolution`. `SegmentStream::zones` keeps its name.
- **Breaking:** `Transport` carries `scan_for(id, window)`, which answers as
  soon as the device is found. Implement it on any transport of your own.
- `lan` returns at the device's reply, `ble` reads what the adapter heard every
  200 ms, and `cloud` lists the account, which answers for every device at once.

### Fixed

- `DeviceHandle::open_stream` refuses `StreamOptions::gradient` over a mode
  whose file carries the setting nowhere, with `Error::NoRoleCommand`.
- `DeviceHandle::segment` and `DeviceHandle::open_stream` read the same device
  file, so both refuse the same files rather than paint hard-edged zones.
- `Govee::problems` reports an enabled mode whose credential the configuration
  does not carry, so a caller reads it before it sends.
- A mode with no credential stays a problem and not a startup error: a command
  over it fails with `Error::MissingCredential`.
- `DeviceHandle::segment` and `DeviceHandle::open_stream` wait for the segment
  channel to arm before the first paint, which the firmware needs to render it.
- The wait is `measurements.arm_settle_ms`, or a conservative default. A stream
  pays it once, at the open, and not per frame.

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
