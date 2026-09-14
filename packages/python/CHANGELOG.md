# Changelog

Changes to `govee-toolkit` (Python), the binding over the Rust core in
[`../rust`](../rust). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

## [0.1.0] — 2026-09-14

The first release that carries code. The version on PyPI before it was a
`0.0.0` placeholder that held the name.

### Added

- A [PyO3](https://pyo3.rs) binding over the core crate, built with
  [maturin](https://www.maturin.rs). The protocol stays in Rust: the package
  implements no command and no frame. The extension module is
  `govee_toolkit._govee_toolkit`, and the `govee_toolkit` package wraps it.
- `Govee`, the facade. `Govee.start()` reads the configuration file and the
  catalog the wheel carries, and takes another of either, `scan()`
  and `scan_on()` discover devices, `devices()`, `modes()` and `problems()`
  report what it holds, `device()` returns a handle, `events()` iterates the
  events, and `close()` shuts it down.
- `DeviceHandle`, one device. `send()` and `read()` take any command the device
  file declares; `power()`, `brightness()`, `color()`, `color_temp()`,
  `music()`, `segment()`, `gradient()` and `provision_wifi()` name the common
  ones. `status()` and `last_status()` read the state, `spec()` returns what
  `devices/<SKU>.yaml` declares, and `serving_mode()` and `ensure_known()`
  answer which mode a command takes.
- `StatusStream` — `watch_status()` reports one device's status as answers
  arrive, over the mode that serves it. It requests nothing of its own.
- `SegmentStream` — `open_stream()` opens the segment channel, `set_all()`,
  `set_zone()`, `fill()` and `clear()` paint it, and `frames_sent` and
  `frames_superseded` count the frames. It is an async context manager.
- `Config` and `Catalog`. `Config.load()` and `Config.load_from()` read the
  configuration; `Catalog.embedded()` reads the catalog the wheel carries, and
  `catalog.device()` returns a whole resolved device file.
- `Device`, `DeviceStatus`, `Health`, `Reply`, `Served` and `EventStream` — the
  values the calls above return.
- The API is `asyncio` only. Every call that reaches a device is awaited, and
  the core's Tokio runtime runs under it. There is no synchronous wrapper.
- `GoveeError`, and the subclasses `CodecError`, `TransportError` and
  `ConfigError`. Every one carries `.code`, the stable identifier the core gives
  the failure, such as `unknown_command` or `no_mode_available`. A value the
  binding refuses before the core sees it is a `ValueError`, which carries no
  code.
- `govee_toolkit.__version__`, the package version;
  `govee_toolkit.CORE_VERSION`, the version of the core the binding was built
  from; and `govee_toolkit.MODES`, the three mode names.
- Typed stubs for the whole surface, and the `py.typed` marker beside them, so
  an editor and a type checker read the extension module.
- Wheels for Linux, macOS and Windows on `x86_64` and `aarch64`, for Linux on
  `armv7`, and for musl on `x86_64`, `aarch64` and `armv7`. The wheel is `abi3`
  for Python 3.11 and up: one wheel per platform serves every version. CI
  imports the wheel on the floor and on the newest release, so `abi3` is
  checked and not assumed. The `armv7` and musl wheels are cross-built in
  manylinux and musllinux containers, and the D-Bus library that the `ble` mode
  needs is compiled from vendored sources, so the build needs no target
  sysroot.
- The release publishes no source distribution. `packages/rust/build.rs` reads
  `../../devices`, so a build outside the repository needs `GOVEE_DEVICES_DIR`
  and the device files it names.
- The wheel embeds the device catalog, so an install needs no data file.
- Package scaffolding: `pyproject.toml` declares the name `govee-toolkit`,
  `requires-python >= 3.11` and no dependencies, with `authors`, `keywords`,
  `classifiers` and `[project.urls]`.

### Changed

- The package README links to `docs/` and `devices/` by absolute URL: it is the
  description shown on PyPI, where a link out of the package directory is dead.

### Fixed

- `license = "MIT"` as an SPDX expression instead of a path to the repository's
  `LICENSE`. The build backend refuses a license file above the package
  directory, so no distribution could be built at all.
