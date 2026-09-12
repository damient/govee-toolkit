# Changelog

Changes to `govee-toolkit-cli`, the crate that publishes the `govee` binary to
crates.io from `packages/rust/crates/cli`. It versions apart from
`govee-toolkit` and releases under `cli-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md).

## [Unreleased]

### Added

- The crate, the `govee` binary and the command surface: `scan`, `devices`,
  `describe`, `doctor`, `status`, `on`, `off`, `brightness`, `color`,
  `colortemp`, `segment`, `gradient`, `music`, `send`, `stream`, `watch` and
  `provision`.
- The verbs reach the device file through a `role:`, so no command name lives
  in this crate and a binding gets the same verb from the core. A device whose
  file claims no entry for the role fails and names the role.
- `send` names an entry of the device file, and works for a SKU this build has
  never heard of. The entry declares the type of every argument, so
  `--arg name=value` is read under that type; the range stays the codec's to
  check.
- `describe` reports what a device file declares — the modes, the capabilities,
  the commands and their arguments — for a device identity or for a SKU typed
  directly. Per mode, it reports how many zones one paint states and whether
  that reaches every addressable LED, and for the unit, every count at which it
  refines. It reads no hardware.
- `status` asks the device for its state and reports the mode that answered.
  A field the reply leaves out reads `?` in the text form and `null` in JSON.
- `gradient <device> on|off` sets whether the firmware interpolates between
  zones, and paints nothing. The interpolation wraps from the last zone back to
  the first, so one lit zone at one end also lights the other. Over a mode that
  carries the setting inside its painting frame it fails and names the role:
  nothing here holds the colors the device shows, so they cannot be painted
  again under the other setting. Pass `--gradient` to `segment` there.
- `segment` paints zones. One `#RRGGBB` fills every zone; a comma-separated
  list states one zone each, which is how a mode that addresses every LED is
  painted pixel by pixel. `-` reads that list from one line of stdin, so a
  long list reaches the device from a file or a pipe. `--resolution` takes
  `app`, `native` or a count, and says how many zones the frame states; the
  list must be that long. A count the unit renders as a smaller one is refused,
  and the message names the counts it refines at. `--zones` names the zones to
  paint and leaves every other zone alone, which needs a mode whose file marks
  `role: segment_color_masked` and takes one color. Nothing disarms the segment
  channel afterwards: a disarm ends the channel, and the colors with it.
- `colortemp` sets the white temperature, in kelvin. White and color are
  mutually exclusive states, so it ends the color the device showed. Where the
  mode carries the RGB rendering of the temperature in the same frame, the core
  computes it and sends both. A value outside the declared range is an error,
  never a clamp.
- `music` plays an effect the device renders from its own microphone. The
  device listens, and nothing streams from the host. `--sensitivity` says how
  loud the sound must be, `--soft` renders in fades rather than on the beat,
  and `--color` imposes a color the firmware would otherwise choose. The effect
  identifiers belong to the mode, and `describe` reports the range each mode
  takes. Nothing stops the effect: `on`, `off`, `color` or `colortemp` ends it.
- `stream` feeds the segment channel one frame per line of stdin: one
  `#RRGGBB`, which fills every zone, or one per zone. `--resolution` takes
  `app`, `native` or a count, the same word `segment` uses, and `--rate`
  overrides the rate measured for the unit. The run reports the frames sent and
  the frames a later write replaced.
- `scan` reports every device that answered, including one the configuration
  does not enable the scanned mode for. Such a device is printed as answering
  over that mode rather than with its modes, and the JSON form carries
  `enabled: false`. `ble` reports a device under the handle the platform gives
  the peripheral, so a first scan always finds one the configuration cannot
  name yet. `devices` keeps listing only what a command can go to.
- `watch` prints events as they arrive. It scans once at the start, and
  `--rescan-ms` repeats the scan.
- `doctor` reports everything wrong with the configuration, including an
  enabled mode whose credential is missing and what can only be checked once
  devices are known. It reads no hardware.
- `provision` puts a device on a Wi-Fi network over `ble`, behind the `ble`
  cargo feature. The network name comes from `--ssid` or from
  `GOVEE_WIFI_SSID`. The password comes from `--password`, from
  `GOVEE_WIFI_PASSWORD`, or is empty with `--open`; the command line wins over
  the environment. The password travels in plaintext, so anything in Bluetooth
  range while the command runs reads it. Nothing acknowledges the transfer, so
  the report says what was sent.
- A device file that claims no entry for what a command needs exits with code
  5, as a refused argument does. Nothing was sent either way.
- A command scans first where no transport of an enabled mode knows the device
  yet. `ble` relates a device to a handle through an advertisement alone, and
  `cloud` lists the account at startup, so neither keeps anything across runs
  and a one-shot command must discover the device. The test is per mode: the
  `lan` cache answers from disk for `lan`, and costs a scan on the modes that
  need one.
- `--json` writes one object per line on stdout and an error object on stderr.
  That form is the contract, and the exit codes are in the README. The `kind`
  of an error is the core's own error code. The text form is for a person.
- `--mode` restricts a run to one mode. It enables no mode the configuration
  leaves out: a device that does not enable the mode asked for is refused,
  rather than served by another one. It restricts the wire as well as the
  report: `scan` and `watch` touch that mode alone, `devices` lists the devices
  that enable it, and `watch` prints the events that mode raises.
