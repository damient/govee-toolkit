# Changelog

Changes to `govee-toolkit-dmx`, the crate that builds the `govee-dmx` binary
from `packages/rust/crates/dmx`. It versions apart from `govee-toolkit` and
releases under `dmx-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md). The design
is [`../../../../docs/dmx.md`](../../../../docs/dmx.md).

Nothing is published yet: the manifest carries `publish = false`.

### Added

- The crate, the `govee-dmx` binary and the `artnet` and `sacn` cargo
  features. The binary answers `--version` and `--help`, and drives no device
  yet.
- `profile` — the channel table, derived from the device file and from nothing
  else. It answers the personalities a device serves over `lan`, and the
  channels of each: the offset, what the channel drives, and the pair a scaled
  channel writes into. A table wider than one universe is an error, never a
  truncation.
- `profile::Scale` — a slot scaled into what a device parameter takes, plus
  the step count the pair resolves to. Slot 0 carries no value: the dimmer
  powers the device off there, and the white channel sends no command.
