# Changelog

Changes to `govee-toolkit` (Python), the binding over the Rust core in
[`../rust`](../rust). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

## [0.1.0] — 2026-09-15

The first release that carries code. The version on PyPI before it was a
`0.0.0` placeholder that held the name.

### Added

- A [PyO3](https://pyo3.rs) binding over the core crate, built with
  [maturin](https://www.maturin.rs). It implements no command and no frame.
- The extension module is `govee_toolkit._govee_toolkit`, and the
  `govee_toolkit` package wraps it.
- `Govee`, the facade. `start()` reads the configuration file and the catalog
  the wheel carries, and takes another of either.
- `Govee.scan()` and `scan_on()` discover devices, and `devices()`, `modes()`
  and `problems()` report what the facade holds.
- `Govee.device()` returns a handle, `events()` iterates the events, and
  `close()` shuts the facade down.
- `DeviceHandle`, one device. `send()` and `read()` take any command the device
  file declares.
- `power()`, `brightness()`, `color()`, `color_temp()`, `music()`, `segment()`,
  `gradient()` and `provision_wifi()` name the common commands.
- `DeviceHandle.status()` and `last_status()` read the state, and
  `serving_mode()` and `ensure_known()` answer which mode a command takes.
- `DeviceHandle.spec()` returns what `devices/<SKU>.yaml` declares, and
  `describe()` returns the record `govee describe` prints.
- `send()` and `read()` read each value under the type the entry declares:
  `[0, 1, 2]` is zone indices, byte values or one color as the device file says.
- `segment()` and `open_stream()` take `resolution` and `rate` as text — `app`,
  `native` or a zone count, and `measured` or a rate in hertz — or `None`.
- `music()` takes `sensitivity=None` and `soft=None`. `None` takes the core's
  default, which is sensitivity `50` and on the beat.
- `StatusStream` — `watch_status()` reports one device's status as answers
  arrive, over the mode that serves it. It requests nothing of its own.
- `SegmentStream` — `open_stream()` opens the segment channel, and `set_all()`,
  `set_zone()`, `fill()` and `clear()` paint it. It is an async context manager.
- `SegmentStream.frames_sent` and `SegmentStream.frames_superseded` count the
  frames.
- `Config.load()` and `Config.load_from()` read the configuration, and
  `Config.to_dict()` returns the whole of it. It carries no credential.
- `Catalog.embedded()` reads the catalog the wheel carries, and
  `catalog.device()` returns a whole resolved device file.
- `Device`, `DeviceStatus`, `Health`, `Reply`, `Served` and `EventStream` — the
  values the calls above return.
- An event is a dict keyed by `event`, and a `status` event carries `on` and
  `brightness` directly. `govee watch --json` prints the same records.
- The API is `asyncio` only. Every call that reaches a device is awaited, and
  the core's Tokio runtime runs under it. There is no synchronous wrapper.
- `GoveeError`, and the subclasses `CodecError`, `TransportError` and
  `ConfigError`. A failure raises the subclass of its family.
- Every error carries `.code`, the stable identifier the core gives the
  failure, such as `unknown_command` or `no_mode_available`.
- A value the binding refuses before the core sees it is a `ValueError`, which
  carries no code.
- A name that names no mode reports the names that work.
- `govee_toolkit.__version__` — the package version, read from the distribution
  metadata on the first read of it.
- `govee_toolkit.CORE_VERSION`, the version of the core the binding was built
  from, and `govee_toolkit.MODES`, the three mode names.
- Typed stubs for the whole surface in `_govee_toolkit.pyi`, with the `py.typed`
  marker beside them. The type aliases live in `govee_toolkit/_types.py`.
- Wheels for Linux and Windows on `x86_64` and `aarch64`, for macOS on
  `aarch64`, for Linux on `armv7`, and for musl on `x86_64`, `aarch64` and
  `armv7`. macOS `x86_64` carries no wheel.
- The wheel is `abi3` for Python 3.11 and up: one wheel per platform serves
  every version. CI imports it on the floor and on the newest release.
- The `armv7` and musl wheels are cross-built in manylinux and musllinux
  containers, so the build needs no target sysroot. The D-Bus library that the
  `ble` mode needs is compiled from vendored sources.
- The wheel embeds the device catalog, so an install needs no data file.
- The release publishes no source distribution. `packages/rust/build.rs` reads
  `../../devices`, so a build outside the repository needs `GOVEE_DEVICES_DIR`.
- `pyproject.toml` declares the name `govee-toolkit`, `requires-python >= 3.11`
  and no dependencies.
