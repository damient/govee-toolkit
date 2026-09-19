# Changelog

Changes to `govee-toolkit-cli`, the crate that publishes the `govee` binary to
crates.io from `packages/rust/crates/cli`. It versions apart from
`govee-toolkit` and releases under `cli-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md).

### Added

- `identify [DEVICE]...` — takes every device off at once, then lights them one
  at a time in one color, so a person maps an identity to a fixture in the
  room. It walks the devices `lan` reaches, or the mode `--mode` names, and
  every device that scan finds when the command line names none.
- `identify --color`, `--wait-ms`, `--hold-ms` and `--keep` — the color each
  device shows, the interval between two steps, how long the last device holds
  it, and leaving the rig lit at the end.

## [0.5.0] — 2026-09-16

### Changed

- **Breaking:** `stream` and `segment` refuse a zone count past what the mode
  paints, and `--resolution native` where the mode's color list is shorter
  than the unit's pixels, with `zone_count_unsupported`. Ask for
  `--resolution app`.
- A `status` record under `watch --json` carries `color` as `#RRGGBB`,
  `color_temp_kelvin` and `raw`. It reports what `status --json` reports, so
  the two surfaces agree on one reply.
- `send`, `status` and `stream` resolve the mode, the SKU and the device file
  once per command. A command reads its arguments and sends over one mode.

## [0.4.0] — 2026-09-15

### Changed

- `send` reports an unknown argument as the crate does:
  ``<command>: unknown argument `x`; it declares y, z``. The exit code stays
  the usage one.

## [0.3.0] — 2026-09-14

### Changed

- **Breaking:** the minimum supported Rust version is 1.89. Update the
  toolchain: `cargo install` builds from source, an installed binary does not.
- `describe` reports no bound for an argument the device file does not bound.
  `--json` carries `null` under `bound`, and the text output carries nothing.

## [0.2.0] — 2026-09-13

### Changed

- **Breaking:** `provision` reports what the device answered. The JSON carries
  `result`, `accepted` or `sent`, in place of `sent`.
- The binary carries `lan`, `ble` and `cloud` by default, so a mode named at the
  command line answers. `--no-default-features --features lan` builds it with
  `lan` alone.
- `doctor` reports every enabled mode this build carries no transport for,
  before a command is sent.
- A command over a mode this build carries no transport for fails with
  `mode_not_implemented`. It reported `unknown_device`, which named the wrong
  cause.

### Removed

- **Breaking:** `describe --json` no longer carries `documented` on a command.
  A reader that keys off it reads the protocol docs instead.

### Fixed

- `send` reads back an entry whose `chunk:` block declares a `reply:`. It wrote
  the frames and reported nothing, because the test for a declared answer read
  the other three fields and not that block.

## [0.1.0] — 2026-09-12

The first release of the `govee` binary. It wraps `govee-toolkit` and holds
no protocol logic: a verb reaches the device file through a `role:`, so no
command name and no SKU name lives in this crate.

### Added

- The crate, the `govee` binary and its 17 subcommands, from `on` and `color` to
  `segment`, `stream` and `provision`. The README lists them.
- The verbs reach the device file through a `role:`, so no command name lives in
  this crate. A device whose file claims no entry fails and names the role.
- A subcommand that names a device discovers it first, and returns as soon as
  that device answers rather than after the whole scan window.
- A device no enabled mode finds fails with `unknown_device`, before the command
  runs.
- A command scans first where no transport of an enabled mode knows the device
  yet. The `lan` cache answers from disk; `ble` and `cloud` each cost a scan.
- Every subcommand reads the `GOVEE_*` variables from a `.env` file, so no
  wrapper script supplies `GOVEE_API_KEY` or the Wi-Fi credential.
- The search goes up from the working directory to the repository root, then
  reads `~/.config/govee-toolkit/.env`. The environment wins over a file.
- `--env-file <PATH>` names one file and replaces the search. A file named that
  way is an error when it is absent.
- `--no-env` reads no file: the process environment supplies the variables
  alone.
- `send` names an entry of the device file, and works for a SKU this build has
  never heard of. The entry declares the type of every `--arg name=value`.
- `describe` reports what a device file declares for a device or a SKU: the
  modes, the capabilities, the commands and the arguments. It reads no hardware.
- `describe` reports, per mode, how many zones one paint states, whether that
  reaches every addressable LED, and every count at which the unit refines.
- `status` asks the device for its state and reports the mode that answered. A
  field the reply leaves out reads `?` in the text form and `null` in JSON.
- `gradient <device> on|off` sets whether the firmware interpolates between
  zones, and paints nothing. The interpolation wraps around the last zone.
- Over a mode that carries the gradient inside its painting frame, `gradient`
  fails and names the role. Pass `--gradient` to `segment` there.
- `segment` paints zones. One `#RRGGBB` fills every zone; a comma-separated list
  states one zone each, which paints a per-LED mode pixel by pixel.
- `segment -` reads the color list from one line of stdin, so a long list
  reaches the device from a file or a pipe.
- `segment --resolution` takes `app`, `native` or a count, and says how many
  zones the frame states. The list must be that long.
- A `--resolution` count the unit renders as a smaller one is refused, and the
  message names the counts it refines at.
- `segment --zones` paints the zones named and leaves the rest alone, which
  needs a file marking `role: segment_color_masked`, and takes one color.
- Nothing disarms the segment channel after a paint: a disarm ends the channel,
  and the colors with it.
- `colortemp` sets the white temperature, in kelvin, and ends the color the
  device showed. A value outside the declared range is an error, never a clamp.
- Where the mode carries the RGB rendering of the temperature in the same frame,
  the core computes it and sends both halves.
- `music` plays an effect the device renders from its own microphone. The device
  listens, and nothing streams from the host.
- `--sensitivity` says how loud the sound must be, `--soft` renders in fades
  rather than on the beat, and `--color` imposes one color.
- The music effect identifiers belong to the mode, and `describe` reports the
  range each mode takes. `on`, `off`, `color` or `colortemp` ends the effect.
- `stream` feeds the segment channel one frame per line of stdin: one `#RRGGBB`,
  which fills every zone, or one per zone.
- `stream --resolution` takes the same word `segment` uses, `--rate` overrides
  the rate measured for the unit, and `--gradient` interpolates between zones.
- `stream --gradient` fails and names the role over a mode whose device file
  carries the setting nowhere.
- `stream` reports the frames sent and the frames a later write replaced.
- `scan` reports every device that answered, including one the configuration
  does not enable the scanned mode for, which carries `enabled: false` in JSON.
- `ble` reports a device under the handle the platform gives the peripheral, so
  a first scan always finds one the configuration cannot name yet.
- `devices` lists only what a command can go to.
- `watch` prints events as they arrive. It scans once at the start, and
  `--rescan-ms` repeats the scan.
- `doctor` reports everything wrong with the configuration, an enabled mode
  whose credential is missing included. It reads no hardware.
- `doctor` reports which file the `GOVEE_*` variables came from, as `env_file`
  in JSON.
- `provision` puts a device on a Wi-Fi network over `ble`, behind the `ble`
  cargo feature. The network name comes from `--ssid` or `GOVEE_WIFI_SSID`.
- The `provision` password comes from `--password`, from `GOVEE_WIFI_PASSWORD`,
  or is empty with `--open`. The command line wins over the environment.
- The `provision` password travels in plaintext, so anything in Bluetooth range
  while the command runs reads it.
- Nothing acknowledges the Wi-Fi transfer, so the `provision` report says what
  was sent.
- A device file that claims no entry for what a command needs exits with code 5,
  as a refused argument does. Nothing was sent either way.
- `--json` writes one object per line on stdout and an error object on stderr.
  The `kind` of an error is the core's own error code.
- The JSON form is the contract, and the exit codes are in the README. The text
  form is for a person.
- `--mode` restricts a run to one mode. It enables no mode the configuration
  leaves out: a device that does not enable that mode is refused.
- `--mode` restricts the wire as well as the report: `scan` and `watch` touch
  that mode alone, and `devices` lists the devices that enable it.
