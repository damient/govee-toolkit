![Govee Toolkit](https://raw.githubusercontent.com/damient/govee-toolkit/main/docs/assets/banner.png)

# Govee Toolkit DMX

Your lighting desk patches a Govee strip the way it patches any other fixture.
The node receives DMX on the network you already run, and it writes every frame
straight to the device over Wi-Fi. No cloud, and no box between the desk and
the light.

[![status](https://img.shields.io/badge/status-beta-yellow)](https://github.com/damient/govee-toolkit/blob/main/docs/roadmap.md)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

[![govee-toolkit on crates.io](https://img.shields.io/crates/v/govee-toolkit?logo=rust&logoColor=white&label=govee-toolkit)](https://crates.io/crates/govee-toolkit)
[![govee-toolkit-cli on crates.io](https://img.shields.io/crates/v/govee-toolkit-cli?logo=rust&logoColor=white&label=govee-toolkit-cli)](https://crates.io/crates/govee-toolkit-cli)
[![govee-toolkit-dmx on crates.io](https://img.shields.io/crates/v/govee-toolkit-dmx?logo=rust&logoColor=white&label=govee-toolkit-dmx)](https://crates.io/crates/govee-toolkit-dmx)
[![govee-toolkit on PyPI](https://img.shields.io/pypi/v/govee-toolkit?logo=python&logoColor=white&label=govee-toolkit)](https://pypi.org/project/govee-toolkit/)
[![govee-toolkit on npm](https://img.shields.io/npm/v/govee-toolkit?logo=npm&logoColor=white&label=govee-toolkit)](https://www.npmjs.com/package/govee-toolkit)

<!-- TODO: demo GIF here — a desk fader running a strip. -->

## What it does

- Receive Art-Net, and announce the node, so a desk lists it and needs no
  address typed in by hand. One protocol is one cargo feature.
- Patch a fixture on 6 channels, on one color per zone, or on one color per
  addressable LED. The channel table comes from the device file, so no model
  carries a table of its own.
- Scan the network and write the patch file. `govee-dmx patch` adds one entry
  per device and moves no address an operator already set on the desk.
- Print the channel table of a model before a device is at risk, and print
  every packet the node reads without driving anything.
- Light one fixture at a time, so you see which patch entry drives which
  fixture in the room.
- Answer a sender that goes quiet, per fixture: hold the last look, go black,
  or power the rig down.

The node is an **input**, not a fourth mode. It reaches a device over your
Wi-Fi (`lan`), the way any other caller of the toolkit does, so a device it
drives must have `lan` enabled. It substitutes no other mode: a device that
stops answering is reported unreachable rather than driven over Bluetooth in
silence.

<br>

<p align="center">
  <a href="https://gvetk.com/docs/dmx/"><img alt="DMX guide" src="https://img.shields.io/badge/DMX%20guide-0b7285?style=for-the-badge"></a>
  <a href="https://gvetk.com/docs/start/"><img alt="Get started" src="https://img.shields.io/badge/Get%20started-3b444b?style=for-the-badge"></a>
  <a href="https://gvetk.com/devices/"><img alt="Devices" src="https://img.shields.io/badge/Devices-3b444b?style=for-the-badge"></a>
  <a href="https://gvetk.com/docs/modes/"><img alt="Modes" src="https://img.shields.io/badge/Modes-3b444b?style=for-the-badge"></a>
  <a href="https://github.com/damient/govee-toolkit/blob/main/devices/README.md"><img alt="Add a device" src="https://img.shields.io/badge/Add%20a%20device-3b444b?style=for-the-badge"></a>
</p>

## Where the package is

The node receives, resolves and drives, and its tests run the whole chain
against a simulated device. Nothing is published yet, so build the binary from
the repository:

```sh
cargo run -p govee-toolkit-dmx -- patch    # scan the network and write the rig
cargo run -p govee-toolkit-dmx -- run      # receive Art-Net and drive it
```

Art-Net is the one protocol the node carries. sACN is the second one people ask
for, and it takes the same channel model.
[`docs/dmx.md`](https://github.com/damient/govee-toolkit/blob/main/docs/dmx.md)
is the design reference: the channels, the patch file and the send policy.
[`docs/roadmap.md`](https://github.com/damient/govee-toolkit/blob/main/docs/roadmap.md)
tracks what the release needs.

## Contributing

The protocol is implemented once, in Rust; Python and Node.js bind to that core.
Command names, byte layouts and measured limits live in
[`devices/`](https://github.com/damient/govee-toolkit/tree/main/devices), never in code, so adding a model is editing one file.
The channel table is derived from those files too, so a new model reaches the
desk with no code at all.
[`docs/architecture.md`](https://github.com/damient/govee-toolkit/blob/main/docs/architecture.md) explains the shape of the code,
and [`CONTRIBUTING.md`](https://github.com/damient/govee-toolkit/blob/main/CONTRIBUTING.md) covers how to document a newly
discovered command.

Confirming whether your model works needs no code:
[`devices/README.md`](https://github.com/damient/govee-toolkit/blob/main/devices/README.md) walks through it.

## Legal notice

The "Govee" trademark is used descriptively only, to identify compatible
devices. This project is not affiliated with, sponsored by, or endorsed by
Govee.

## License

[MIT](https://github.com/damient/govee-toolkit/blob/main/LICENSE)

<!-- Absolute: this file is the package description on the registry, where a
     relative link out of the package directory is dead. -->
