# govee-toolkit

**Control your Govee devices like never before.** Features the official app
never exposed, unlocked straight from your own network — no internet, no cloud
round-trip. Bluetooth and the cloud API are there if you need them.

[![status](https://img.shields.io/badge/status-early%20development-orange)](docs/roadmap.md)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![python](https://img.shields.io/badge/python%20SDK-planned-lightgrey)](packages/python)
[![node](https://img.shields.io/badge/node%20SDK-planned-lightgrey)](packages/node)
[![php](https://img.shields.io/badge/php%20SDK-planned-lightgrey)](packages/php)

An alternative to the official Govee API: a community, multi-language SDK
(Python / Node.js / PHP) that speaks to your devices **directly on your LAN**,
including undocumented commands — effects, scenes, per-segment control — found
through reverse engineering and unavailable anywhere else.

Three **modes** are available — `lan`, `ble`, `cloud` — and **you choose which
ones to enable, per device**: one mode for strict, predictable behavior, or
several when you want the SDK to switch. Nothing is implicit.

> ⚠️ **Community project, not affiliated with Govee.** Unofficial, unsupported
> and not endorsed by Govee.

> 🚧 **Early development.** The protocol is documented — including the
> undocumented commands — and the device schema is in place. The SDKs come next;
> nothing is published to PyPI / npm / Packagist yet. See the
> [roadmap](docs/roadmap.md) for what is coming, and
> [features](docs/features.md) for the full picture.

---

## What works today

- **Device database** — [`devices/`](devices/): the schema and per-SKU
  definitions every SDK reads, so protocol logic lives in one place.
- **Protocol documentation** — [`docs/protocol/`](docs/protocol/): the
  documented LAN protocol **and the undocumented commands** — the raw channel
  for per-segment color at a resolution the app never exposes, the real clamping
  behavior, and how to measure a device's headroom.
- **One verified device** — see [`docs/compatibility.md`](docs/compatibility.md).

Everything else is planned — the full list is in
[`docs/features.md`](docs/features.md), the order in
[`docs/roadmap.md`](docs/roadmap.md).

## Compatibility

Per-SKU support lives in [`docs/compatibility.md`](docs/compatibility.md) —
which devices work, in which modes, and how far. The authoritative data is in
[`devices/`](devices/), one YAML file per SKU.

Govee lists **271 models** that expose the LAN switch — the list is mirrored in
[`docs/lan-supported-devices.md`](docs/lan-supported-devices.md). One of them,
the H61A0, is verified end to end, segment channel included; the rest are
waiting for someone with the hardware.

Adding a device is mostly filling one YAML file and attaching a capture —
[`devices/README.md`](devices/README.md) walks through it.

## Installation

> 🔜 The interface the SDKs will ship with. Watch the repository for the first
> release.

```bash
# Python
pip install govee-toolkit

# Node.js
npm install govee-toolkit

# PHP
composer require govee/toolkit
```

## Quick start

> 🔜 Lands with milestone 4 of the [roadmap](docs/roadmap.md).

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

```yaml
devices:
  "AA:BB:CC:DD:EE:FF":
    modes: [lan]            # single mode — strict, never switches
  "11:22:33:44:55:66":
    modes: [lan, ble]       # preferred lan, may switch to ble
  "99:88:77:66:55:44":
    modes: [cloud]          # remote device, cloud only
```

- **One mode** → the device is only ever reached that way. If it becomes
  unreachable the command **fails and says so**; nothing switches behind your
  back.
- **Several modes** → the list is a preference order. A per-device, per-mode
  circuit breaker (`OK` / `DEGRADED` / `DOWN`) decides when to move to the next
  one, from state already known rather than a fresh timeout on each call. Every
  switch is reported.

`lan` alone is the default. `ble` and `cloud` are opt-in, and a command a mode
cannot serve fails explicitly instead of being approximated.

Full model: [`docs/modes.md`](docs/modes.md). Per-mode protocol notes:
[`lan.md`](docs/protocol/lan.md) · [`ble.md`](docs/protocol/ble.md) ·
[`cloud.md`](docs/protocol/cloud.md).

## Playground & desktop app

> 🔜 Milestones 6 and 7 of the [roadmap](docs/roadmap.md); the directories below
> are scaffolded and ready to build on.

- [`apps/playground/`](apps/playground/) — Node backend plus web UI: device list
  with per-mode state badges, controls (power, brightness, color, scenes), a per
  command latency log, and a **raw payload field** to try out a discovery before
  formalizing it in `devices/*.yaml`.
- [`apps/desktop/`](apps/desktop/) — Electron wrapper around the playground
  (auto-discovery on launch, tray icon). The web playground stays usable on its
  own.

## Integrations

> 🔜 Begins once the core SDKs are stable, so the integrations inherit a solid
> transport rather than carrying their own.

- [`integrations/matter/`](integrations/matter/) — Matter bridge, reachable from
  any Matter controller. First in line: one integration covers every ecosystem.
- [`integrations/home-assistant/`](integrations/home-assistant/) — custom
  component distributable through HACS (consumes `packages/python`). Carries the
  undocumented LAN scenes and segments that Matter cannot express.
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
