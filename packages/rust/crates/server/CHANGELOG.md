# Changelog

Changes to `govee-toolkit-server`, the crate of the `govee-server` binary in
`packages/rust/crates/server`. It releases under `server-vX.Y.Z`. The policy
is [`../../../../docs/versioning.md`](../../../../docs/versioning.md).

## [0.1.0] — Unreleased

### Added

- `govee-server` serves Studio: the API under `/api`, and the web UI.
  The binary carries the build of `apps/studio`, and `--web <directory>`
  serves another copy. It listens on `127.0.0.1:4080` by default.
- `GET /api/hello` answers the name and the version of `govee-toolkit`, and the
  number of SKUs in the embedded catalog.
