![Govee Toolkit](https://raw.githubusercontent.com/damient/govee-toolkit/main/docs/assets/banner.png)

# Govee Toolkit

Your lights live on your network, and your commands stay there with them. Wi-Fi
or a Bluetooth link is all the toolkit needs, straight from your own machine.
The cloud waits as a third mode for the days you are away. And the toolkit
sends commands you have never seen before: the undocumented ones, read off the
wire.

[![status](https://img.shields.io/badge/status-beta-yellow)](https://github.com/damient/govee-toolkit/blob/main/docs/roadmap.md)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

[![govee-toolkit on crates.io](https://img.shields.io/crates/v/govee-toolkit?logo=rust&logoColor=white&label=govee-toolkit)](https://crates.io/crates/govee-toolkit)
[![govee-toolkit-cli on crates.io](https://img.shields.io/crates/v/govee-toolkit-cli?logo=rust&logoColor=white&label=govee-toolkit-cli)](https://crates.io/crates/govee-toolkit-cli)
[![govee-toolkit-dmx on crates.io](https://img.shields.io/crates/v/govee-toolkit-dmx?logo=rust&logoColor=white&label=govee-toolkit-dmx)](https://crates.io/crates/govee-toolkit-dmx)
[![govee-toolkit on PyPI](https://img.shields.io/pypi/v/govee-toolkit?logo=python&logoColor=white&label=govee-toolkit)](https://pypi.org/project/govee-toolkit/)
[![govee-toolkit on npm](https://img.shields.io/npm/v/govee-toolkit?logo=npm&logoColor=white&label=govee-toolkit)](https://www.npmjs.com/package/govee-toolkit)

<!-- TODO: demo GIF here — a strip running per-segment colors. -->

## What it does

- Switch a device on and off, and set the brightness, the color and the white
  temperature.
- Address every segment of a strip on its own, not only the preset effects.
- Drive a strip frame by frame in real time: music reactive, screen ambilight,
  or your own source.
- Put a device out of the box on your Wi-Fi over Bluetooth, which is what makes
  it reachable over `lan`.

A command travels over your Wi-Fi (`lan`), over Bluetooth (`ble`) or through
Govee's servers (`cloud`). You choose which modes to allow, per device. One
allowed mode means one mode: an unreachable device fails with an error rather
than taking a slower path in silence.

<br>

<p align="center">
  <a href="https://gvetk.com/docs/start/"><img alt="Get started" src="https://img.shields.io/badge/Get%20started-0b7285?style=for-the-badge"></a>
  <a href="https://gvetk.com/devices/"><img alt="Devices" src="https://img.shields.io/badge/Devices-3b444b?style=for-the-badge"></a>
  <a href="https://gvetk.com/docs/modes/"><img alt="Modes" src="https://img.shields.io/badge/Modes-3b444b?style=for-the-badge"></a>
  <a href="https://gvetk.com/reference/"><img alt="API reference" src="https://img.shields.io/badge/API%20reference-3b444b?style=for-the-badge"></a>
  <a href="https://github.com/damient/govee-toolkit/blob/main/devices/README.md"><img alt="Add a device" src="https://img.shields.io/badge/Add%20a%20device-3b444b?style=for-the-badge"></a>
</p>

## Where the project is

The engine works over `lan`, over `ble` and over `cloud`, verified on real
hardware: discovery, on/off, brightness, color, per-segment color, and live
animation over the two local modes. It is usable today from Rust, from Python
and from Node.js.

A DMX bridge drives the same devices from a lighting desk:
`govee-toolkit-dmx` receives DMX on the network. Art-Net carries the DMX in.
See [`docs/dmx.md`](https://github.com/damient/govee-toolkit/blob/main/docs/dmx.md).

Next come a desktop app, then Home Assistant, Homebridge and Matter.
[`docs/roadmap.md`](https://github.com/damient/govee-toolkit/blob/main/docs/roadmap.md) tracks the order.

## Contributing

The protocol is implemented once, in Rust; Python and Node.js bind to that core.
Command names, byte layouts and measured limits live in
[`devices/`](https://github.com/damient/govee-toolkit/tree/main/devices), never in code, so adding a model is editing one file.
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
