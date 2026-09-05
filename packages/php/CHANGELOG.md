# Changelog

All notable changes to `govee/toolkit` (PHP) are documented in this file.

The format is based on
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing has been released, and the package has no implementation yet: the
directory holds the manifest and empty `src/` and `tests/` directories. PHP is
the one hand-written port rather than a binding, and it is written once the
Python and Node SDKs are stable; the conformance vectors in
[`../../tests/fixtures/golden/`](../../tests/fixtures/golden/) are what keep it
from drifting.

### Added

- Package scaffolding: `composer.json` reserving the name `govee/toolkit`
  (PSR-4 `Govee\Toolkit\`, PHP >= 8.2, PHPUnit 10 for development).
