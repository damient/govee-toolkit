# Changelog

Changes to `govee-toolkit` (Python), the binding over the Rust core in
[`../rust`](../rust). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

### Added

- Package scaffolding: `pyproject.toml` (hatchling, `requires-python >= 3.10`,
  no dependencies) declaring the name `govee-toolkit`, and the `govee_toolkit`
  module.
- Package metadata: `authors`, `keywords` and `[project.urls]`.

### Changed

- The package README links to `docs/` and `devices/` by absolute URL: it is the
  description shown on PyPI, where a link out of the package directory is dead.

### Fixed

- `license = "MIT"` as an SPDX expression instead of a path to the repository's
  `LICENSE`. The build backend refuses a license file above the package
  directory, so no distribution could be built at all.
