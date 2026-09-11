# Changelog

Changes to `govee-toolkit-cli`, the crate that publishes the `govee` binary to
crates.io from `packages/rust/crates/cli`. It versions apart from
`govee-toolkit` and releases under `cli-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md).

## [Unreleased]

### Added

- The crate, the `govee` binary and the command surface: `scan`, `devices`,
  `describe`, `doctor`, `status`, `on`, `off`, `brightness`, `color`,
  `segment`, `send`, `stream`, `watch` and `provision`.
- The verbs reach the device file through a `role:`, so no command name lives
  in this crate and a binding gets the same verb from the core. A device whose
  file claims no entry for the role fails and names the role.
- `send` names an entry of the device file, and works for a SKU this build has
  never heard of. The entry declares the type of every argument, so
  `--arg name=value` is read under that type; the range stays the codec's to
  check.
- `describe` reports what a device file declares — the modes, the capabilities,
  the commands and their arguments — for a device identity or for a SKU typed
  directly. It reads no hardware.
- `status` asks the device for its state and reports the mode that answered.
  A field the reply leaves out reads `?` in the text form and `null` in JSON.
- `segment` paints zones one color. `--zones` names the zones to paint and
  leaves every other zone alone, which needs a mode whose file marks
  `role: segment_color_masked`; without it every zone takes the color. Nothing
  disarms the segment channel afterwards: a disarm ends the channel, and the
  colors with it.
- `stream` feeds the segment channel one frame per line of stdin: one
  `#RRGGBB`, which fills every zone, or one per zone. `--zones` takes `app`,
  `native` or a count, and `--rate` overrides the rate measured for the unit.
  The run reports the frames sent and the frames a later write replaced.
- `watch` prints events as they arrive. It scans once at the start, and
  `--rescan-ms` repeats the scan.
- `doctor` reports everything wrong with the configuration, including what can
  only be checked once devices are known. It reads no hardware.
- `provision` puts a device on a Wi-Fi network over `ble`, behind the `ble`
  cargo feature. The password comes from `--password`, from
  `GOVEE_WIFI_PASSWORD`, or is empty with `--open`; it travels in plaintext,
  so anything in Bluetooth range while the command runs reads it. Nothing
  acknowledges the transfer, so the report says what was sent.
- A device file that claims no entry for what a command needs exits with code
  5, as a refused argument does. Nothing was sent either way.
- A command scans first where no transport knows the device yet. `ble` relates
  a device to a handle through an advertisement alone and keeps nothing across
  runs, so a one-shot command must discover it. A device already known costs
  no scan, and the `lan` cache answers from disk.
- `--json` writes one object per line on stdout and an error object on stderr.
  That form is the contract, and the exit codes are in the README. The `kind`
  of an error is the core's own error code. The text form is for a person.
- `--mode` restricts a run to one mode. It enables no mode the configuration
  leaves out: a device that does not enable the mode asked for is refused,
  rather than served by another one.
