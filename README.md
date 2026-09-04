# govee-toolkit

**Control your Govee devices like never before.** Features the official app
never exposed, unlocked straight from your own network — no internet, no cloud
round-trip. Bluetooth and the cloud API are there if you need them.

[![status](https://img.shields.io/badge/status-early%20development-orange)](docs/roadmap.md)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![rust](https://img.shields.io/badge/rust%20core-in%20progress-yellow)](packages/rust)
[![python](https://img.shields.io/badge/python%20SDK-planned-lightgrey)](packages/python)
[![node](https://img.shields.io/badge/node%20SDK-planned-lightgrey)](packages/node)
[![php](https://img.shields.io/badge/php%20SDK-planned-lightgrey)](packages/php)

An alternative to the official Govee API: a community, multi-language SDK
(Python / Node.js / PHP) that speaks to your devices **directly on your LAN**,
including undocumented commands — effects, scenes, per-segment control — found
through reverse engineering and unavailable anywhere else.

The protocol is implemented once, in a Rust core; the other languages bind to it
rather than re-implementing it. See
[`docs/architecture.md`](docs/architecture.md).

Three **modes** are available — `lan`, `ble`, `cloud` — and **you choose which
ones to enable, per device**: one mode for strict, predictable behavior, or
several when you want the SDK to switch. Nothing is implicit.

> ⚠️ **Community project, not affiliated with Govee.** Unofficial, unsupported
> and not endorsed by Govee.

> 🚧 **Early development.** The protocol is documented — including the
> undocumented commands — the device schema is in place, and the Rust core
> builds the bytes for it. The transport comes next; nothing is published to
> crates.io / PyPI / npm / Packagist yet. See the
> [roadmap](docs/roadmap.md) for what is coming, and
> [features](docs/features.md) for the full picture.

---

## What works today

- **Device database** — [`devices/`](devices/): the schema and per-SKU
  definitions every SDK reads, so protocol logic lives in one place.
- **Protocol documentation** — [`docs/protocol/`](docs/protocol/): the
  documented LAN protocol and the undocumented commands — the raw per-segment
  color channel, the clamping behavior, and how to measure a device's headroom.
- **Protocol core** — [`packages/rust/crates/govee-core`](packages/rust/crates/govee-core):
  turns a device file plus arguments into the exact bytes to send, raw segment
  frames included. No I/O, no SKU-specific code, no network needed to test it.
- **Conformance vectors** — [`tests/fixtures/golden/`](tests/fixtures/golden/):
  arguments in, exact bytes out. Every implementation must match them, which is
  what keeps the ports from drifting.
- **One verified device** — see [`docs/compatibility.md`](docs/compatibility.md).

Everything else is planned — the full list is in
[`docs/features.md`](docs/features.md), the order in
[`docs/roadmap.md`](docs/roadmap.md).

## Compatibility

Per-SKU support lives in [`docs/compatibility.md`](docs/compatibility.md) —
which devices work, in which modes, and how far. The authoritative data is in
[`devices/`](devices/), one YAML file per SKU.

Govee lists 271 models that expose the LAN switch — the list is mirrored in
[`docs/lan-supported-devices.md`](docs/lan-supported-devices.md). One of them,
the H61A0, is verified end to end, segment channel included; the rest are
untested rather than unsupported.

Adding a device is mostly filling one YAML file and attaching a capture —
[`devices/README.md`](devices/README.md) walks through it.

## Installation

> 🔜 The interface the SDKs will ship with. Watch the repository for the first
> release.

```bash
# Rust
cargo add govee

# Python
pip install govee-toolkit

# Node.js
npm install govee-toolkit

# PHP
composer require govee/toolkit
```

## Quick start

> 🔜 Lands with milestone 5 of the [roadmap](docs/roadmap.md).

```python
# Python — discover devices, then turn a light on
# TODO
```

```js
// Node.js — discover devices, then turn a light on
// TODO
```

## Modes

> 🔜 Design settled; implementation lands with the SDKs.

`lan`, `ble` and `cloud` are three ways of reaching a device, with different
trade-offs — not a fixed chain. You enable the ones you want, per device.

| Mode | Latency | Range | Capabilities |
| ---- | ------- | ----- | ------------ |
| `lan` | lowest | same network | full, including undocumented scenes/segments |
| `ble` | low | Bluetooth range | partial, depends on SKU family |
| `cloud` | highest (internet round-trip) | anywhere | reduced: power / brightness / color |

With one mode enabled, an unreachable device makes the command fail; nothing
switches implicitly. With several, the list is a preference order and every
switch is reported. `lan` alone is the default.

Full model, including the circuit breaker: [`docs/modes.md`](docs/modes.md).
Per-mode protocol notes: [`lan.md`](docs/protocol/lan.md) ·
[`ble.md`](docs/protocol/ble.md) · [`cloud.md`](docs/protocol/cloud.md).

## Playground & desktop app

> 🔜 Milestones 8 and 9 of the [roadmap](docs/roadmap.md); the directories below
> are scaffolded and ready to build on.

- [`apps/playground/`](apps/playground/) — Node backend plus web UI: device
  list with per-mode state badges, controls, a per-command latency log, and a
  raw payload field to try a discovery before formalizing it in
  `devices/*.yaml`.
- [`apps/desktop/`](apps/desktop/) — Electron wrapper around the playground
  (auto-discovery on launch, tray icon). The web playground stays usable on its
  own.

## Integrations

> 🔜 Begins once the core SDKs are stable, so the integrations do not carry
> their own transport.

- [`integrations/matter/`](integrations/matter/) — Matter bridge, reachable from
  any Matter controller. First in line: one integration covers every ecosystem.
- [`integrations/home-assistant/`](integrations/home-assistant/) — custom
  component distributable through HACS (consumes `packages/python`, the PyO3
  binding over the Rust core). Carries the
  undocumented LAN scenes and segments Matter cannot express.
- [`integrations/homebridge/`](integrations/homebridge/) — HomeKit plugin
  (consumes `packages/node`).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) — in particular how to add a SKU or
document a newly discovered undocumented command.

## Legal notice

Protocol reverse engineering is carried out for **interoperability** purposes.
The "Govee" trademark is used descriptively only, to identify compatible
devices. This project is not affiliated with, sponsored by, or endorsed by
Govee.

## License

[MIT](LICENSE)
