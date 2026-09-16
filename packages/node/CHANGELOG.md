# Changelog

Changes to `govee-toolkit` (Node.js), the binding over the Rust core in
[`../rust`](../rust). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

## [0.1.0]

The first version that carries code. The version on npm before it is a `0.0.0`
placeholder that holds the name.

### Added

- A [napi-rs](https://napi.rs) binding over the core crate. It implements no
  command and no frame.
- `binding.cjs` loads the addon that matches the platform, and the TypeScript
  layer over it is what the package exports.
- `Govee`, the facade. `start()` reads the configuration file and the catalog
  the addon carries, and takes another of either.
- `Govee.scan()` and `scanOn()` discover devices, and `devices()`, `modes()`
  and `problems()` report what the facade holds.
- `Govee.device()` returns a handle, `events()` iterates the events, and
  `close()` shuts the facade down.
- `DeviceHandle`, one device. `send()` and `read()` take any command the device
  file declares.
- `power()`, `brightness()`, `color()`, `colorTemp()`, `music()`, `segment()`,
  `gradient()` and `provisionWifi()` name the common commands.
- `DeviceHandle.status()` and `lastStatus()` read the state, and
  `servingMode()` and `ensureKnown()` answer which mode a command takes.
- `DeviceHandle.spec()` returns what `devices/<SKU>.yaml` declares, and
  `describe()` returns the record `govee describe` prints.
- `send()` and `read()` read each value under the type the entry declares:
  `[0, 1, 2]` is zone indices, byte values or one color as the device file says.
- `segment()` and `openStream()` take `resolution` and `rate` as text — `app`,
  `native` or a zone count, and `measured` or a rate in hertz — or `null`.
- `music()` takes `sensitivity` and `soft`. `null` takes the core's default,
  which is sensitivity `50` and on the beat.
- `StatusStream` — `watchStatus()` reports one device's status as answers
  arrive, over the mode that serves it. It requests nothing of its own.
- `EventStream` and `StatusStream` are read with `for await`, and answer
  nothing more once the SDK that reported them is gone.
- `SegmentStream` — `openStream()` opens the segment channel, and `setAll()`,
  `setZone()`, `fill()` and `clear()` paint it. `await using` closes it.
- `SegmentStream.framesSent` and `SegmentStream.framesSuperseded` count the
  frames.
- `color()`, `music()`, `segment()`, `setAll()`, `setZone()` and `fill()` take
  a `Uint8Array` as well: three bytes for every color, read in one crossing.
- `SegmentStream.buffer()` answers a `Uint8Array` of three bytes for every
  zone, which `setAll()` takes back.
- `Config.load()` and `Config.loadFrom()` read the configuration, and
  `config.toJSON()` returns the whole of it. It carries no credential.
- `Catalog.embedded()` reads the catalog the addon carries, and
  `catalog.device()` returns a whole resolved device file.
- `Device`, `DeviceStatus`, `Health`, `Reply` and `Served` — the values the
  calls above return.
- An event is an object keyed by `event`, and a `status` event carries `on` and
  `brightness` directly. `govee watch --json` prints the same records.
- Every call that reaches a device returns a promise, and the core's Tokio
  runtime runs under it. There is no synchronous wrapper.
- Every failure is an `Error` carrying `code`, the stable identifier the core
  gives it, such as `unknown_command` or `no_mode_available`.
- An error's `name` is the family it belongs to: `CodecError`,
  `TransportError` or `ConfigError`.
- A value the binding refuses before the core sees it is a `TypeError`, with
  the code `invalid_argument`.
- A name that names no mode reports the names that work.
- `VERSION`, `CORE_VERSION` — the version of the core the addon was built from
  — and `MODES`, the three mode names.
- TypeScript types for the whole surface in `binding.d.cts`, generated from the
  binding and committed, plus the `Color`, `Arg`, `Resolution`, `Rate` and
  `GoveeEvent` aliases.
- Prebuilt addons for Linux and Windows on `x86_64` and `aarch64`, and for
  macOS on `aarch64`. npm resolves them through `optionalDependencies`.
- The addon embeds the device catalog, so an install needs no data file and no
  Rust toolchain.
- The release publishes no source build. `packages/rust/build.rs` reads
  `../../devices`, so a build outside the repository needs `GOVEE_DEVICES_DIR`.
- `package.json` declares the name `govee-toolkit`, ESM, `node >= 20` and no
  runtime dependency of its own.
