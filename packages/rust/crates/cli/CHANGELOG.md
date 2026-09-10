# Changelog

Changes to `govee-toolkit-cli`, the crate that publishes the `govee` binary to
crates.io from `packages/rust/crates/cli`. It versions apart from
`govee-toolkit` and releases under `cli-vX.Y.Z`. The policy is
[`../../../../docs/versioning.md`](../../../../docs/versioning.md).

## [Unreleased]

### Added

- The crate, the `govee` binary and the command surface: `scan`, `devices`,
  `describe`, `send`, `status`, `on`, `off`, `brightness`, `color` and
  `segment`. `scan`, `devices`, `on`, `off`, `brightness` and `color` run;
  every other subcommand is declared and answers with exit code 6.
- The verbs reach the device file through a `role:`, so no command name lives
  in this crate and a binding gets the same verb from the core. A device whose
  file claims no entry for the role fails and names the role.
- `--json` writes one object per line on stdout and an error object on stderr.
  That form is the contract, and the exit codes are in the README. The `kind`
  of an error is the core's own error code. The text form is for a person.
- `--mode` restricts a run to one mode. It enables no mode the configuration
  leaves out: a device that does not enable the mode asked for is refused,
  rather than served by another one.
