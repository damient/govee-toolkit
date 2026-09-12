# govee-toolkit

Control Govee devices from Rust over the LAN, over Bluetooth or through the
cloud, including undocumented commands observed on the wire.

**Documentation: [gvetk.com](https://gvetk.com)**

[![govee-toolkit on crates.io](https://img.shields.io/crates/v/govee-toolkit?logo=rust&logoColor=white&label=crates.io)](https://crates.io/crates/govee-toolkit)
[![docs.rs](https://img.shields.io/docsrs/govee-toolkit?logo=docsdotrs&logoColor=white&label=docs.rs)](https://docs.rs/govee-toolkit)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

This is the reference implementation. Protocol logic lives here once, and every
other language reaches it through a binding rather than a port.

## Start here

| You want to | Go to |
| ----------- | ----- |
| Install it and send a first command | [gvetk.com/docs/start](https://gvetk.com/docs/start/) |
| Know whether your model works | [gvetk.com/devices](https://gvetk.com/devices/) |
| Pick and configure the modes | [gvetk.com/docs/modes](https://gvetk.com/docs/modes/) |
| Read every command and method | [gvetk.com/reference](https://gvetk.com/reference/) |
| Read the API item by item | [docs.rs/govee-toolkit](https://docs.rs/govee-toolkit) |

## What it does

- Switch a device on and off, and set the brightness, the color and the white
  temperature.
- Address every segment of a strip on its own, not only the preset effects.
- Drive a strip frame by frame in real time, over `lan` or over `ble`.
- Put a device out of the box on your Wi-Fi over Bluetooth.
- Read what a device answers, including the fields this crate does not model.

## Install

```bash
cargo add govee-toolkit
cargo add govee-toolkit --features ble,cloud
```

Async, on Tokio. `lan` is on by default. `ble` needs a Bluetooth adapter, and
the dbus headers on Linux (`apt install libdbus-1-dev`). `cloud` needs a Govee
API key. Turn every feature off and what is left is the codec alone — arguments
in, bytes out, no socket and no runtime.

## Quick start

```rust
use govee_toolkit::{Args, Config, Govee};

let govee = Govee::start(Config::load()?).await?;

let devices = govee.scan().await?;
let id = devices[0].id.clone();

let served = govee
    .device(&id)
    .send("power", &Args::new().int("on", 1))
    .await?;

println!("served over {}", served.mode);
```

Command names — `power`, `brightness`, `color` — are entries in the device's
YAML file in [`devices/`][devices], not identifiers in this crate. A name a
device does not define, or an argument outside its declared range, is an error
before anything reaches the network. A command carries the same name across
modes; its arguments do not, because the frames differ.

`govee.catalog()` reports what a SKU declares for a mode, so a user interface
builds its controls from the catalog rather than from a hardcoded list.

Three runnable examples send every command of one device file to a real device:
[`examples/lan_tour.rs`](examples/lan_tour.rs),
[`examples/ble_tour.rs`](examples/ble_tour.rs) and
[`examples/cloud_tour.rs`](examples/cloud_tour.rs).

## What this crate will not do to you

- **It never panics on your behalf.** No `unsafe`, and no `panic` / `unwrap` /
  `expect` in library code. Everything that can fail returns `Error`.
- **It never clamps.** An out-of-range argument is rejected. The firmware clamps
  in silence, and reporting success for a value the device did not apply would
  make the SDK lie about the state of your lights.
- **It never substitutes a mode.** With one mode enabled and the device
  unreachable, you get `NoModeAvailable` — not a slower path taken quietly, and
  not a segment animation approximated with a plain color change.
- **It never overrides a device file silently.** `Catalog::overlay` returns
  everything it replaced.

## Inside the crate

| Layer | Where | Contents |
| ----- | ----- | -------- |
| Codec | [`src/codec/`](src/codec) | Device catalog, command encoding, raw frame codec. No I/O. |
| Transport | [`src/transport/`](src/transport) | What every mode shares: the `Transport` trait, device identity, errors, the per-device circuit breaker |
| `lan` | [`src/lan/`](src/lan) | UDP: discovery, device cache, reused socket |
| `ble` | [`src/ble/`](src/ble) | GATT: scan, one connection per device, paced writes |
| Stream | [`src/stream/`](src/stream) | The segment channel: armed once, fed frames on a clock |
| Facade | [`src/`](src) | Configuration, mode selection, events |

The codec does no I/O, so every protocol decision is testable without hardware
and without a network. A transport carries bytes for one mode and never chooses
between modes. [`docs/architecture.md`][architecture] explains why, and covers
the compiled-in catalog and the device simulator that tests it.

## Working on it

```bash
../../tools/qa.sh                             # everything CI runs
cargo run -p govee-toolkit-sim -- --sku H61A0 # a fake device, no hardware
```

Two conventions to know before you open a pull request: no SKU and no command
name appears in Rust code, and every command in the catalog carries a
conformance vector under `tests/fixtures/golden/`.
[`CONTRIBUTING.md`][contributing] covers the rest.

## License

[MIT](https://github.com/damient/govee-toolkit/blob/main/LICENSE)

<!-- Absolute: this file is the crate description on crates.io, where a
     relative link out of the package directory is dead. -->
[architecture]: https://github.com/damient/govee-toolkit/blob/main/docs/architecture.md
[contributing]: https://github.com/damient/govee-toolkit/blob/main/CONTRIBUTING.md
[devices]: https://github.com/damient/govee-toolkit/tree/main/devices
