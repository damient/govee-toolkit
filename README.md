# Govee Toolkit

Control your Govee lights from your own machine, over your own network — no
internet, no Govee account, no cloud round-trip. It also sends commands the
official app does not expose.

**Documentation: [gvetk.com](https://gvetk.com)**

[![status](https://img.shields.io/badge/status-alpha-orange)](docs/roadmap.md)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

[![govee-toolkit on crates.io](https://img.shields.io/crates/v/govee-toolkit?logo=rust&logoColor=white&label=govee-toolkit)](https://crates.io/crates/govee-toolkit)
[![govee-toolkit-cli on crates.io](https://img.shields.io/crates/v/govee-toolkit-cli?logo=rust&logoColor=white&label=govee-toolkit-cli)](https://crates.io/crates/govee-toolkit-cli)
[![govee-toolkit on PyPI](https://img.shields.io/pypi/v/govee-toolkit?logo=python&logoColor=white&label=govee-toolkit)](https://pypi.org/project/govee-toolkit/)
[![govee-toolkit on npm](https://img.shields.io/npm/v/govee-toolkit?logo=npm&logoColor=white&label=govee-toolkit)](https://www.npmjs.com/package/govee-toolkit)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

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

## Start here

| You want to | Go to |
| ----------- | ----- |
| Install it and send a first command | [gvetk.com/docs/start](https://gvetk.com/docs/start/) |
| Know whether your model works | [gvetk.com/devices](https://gvetk.com/devices/) |
| Pick and configure the modes | [gvetk.com/docs/modes](https://gvetk.com/docs/modes/) |
| Read the API | [gvetk.com/reference](https://gvetk.com/reference/) |
| Report a device, or add one | [`devices/README.md`](devices/README.md) |

## Which devices work

Find your model number — it looks like `H61A0`, on the box and in the Govee app
under your device's settings. 271 models ship a "LAN Control" switch, and
everything over `lan` needs that switch on.

One model is confirmed end to end so far, the **H61A0**, segments included. The
other 270 are untested rather than unsupported, so a test on yours moves the
project forward. [gvetk.com/devices](https://gvetk.com/devices/) tracks what is
known.

## Where the project is

The engine works over `lan`, over `ble` and over `cloud`, verified on real
hardware: discovery, on/off, brightness, color, per-segment color, and live
animation over the two local modes. It is usable today from Rust.

Next comes the packaging around it — the Python and Node.js packages, then a
desktop app, then Home Assistant, Homebridge and Matter.
[`docs/roadmap.md`](docs/roadmap.md) tracks the order.

## Contributing

The protocol is implemented once, in Rust; Python and Node.js bind to that core.
Command names, byte layouts and measured limits live in
[`devices/`](devices/), never in code, so adding a model is editing one file.
[`docs/architecture.md`](docs/architecture.md) explains the shape of the code,
and [`CONTRIBUTING.md`](CONTRIBUTING.md) covers how to document a newly
discovered command.

Confirming whether your model works needs no code:
[`devices/README.md`](devices/README.md) walks through it.

## Legal notice

Protocol reverse engineering is carried out for **interoperability** purposes.
The "Govee" trademark is used descriptively only, to identify compatible
devices. This project is not affiliated with, sponsored by, or endorsed by
Govee.

## License

[MIT](LICENSE)
