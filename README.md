# govee-toolkit

Control your Govee lights from your own machine, over your own network — no
internet, no Govee account, no cloud round-trip. Including a few things the
official app never exposed.

[![status](https://img.shields.io/badge/status-early%20development-orange)](docs/roadmap.md)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

<!-- TODO: demo GIF here — a strip running per-segment colors. Five seconds of
     that is worth more than anything written below. -->

---

## What you can do with it

Your Govee device already listens on your local network — that is how the
official app reaches it when your phone is on the same Wi-Fi. This project
speaks to it the same way, directly, and adds commands the app keeps to itself.

| | |
| --- | --- |
| **On/off, brightness, color** | From a script, a keyboard shortcut, a home automation — anything that can run code. |
| **Every segment of a strip, individually** | The app offers a set of preset effects. Here you address the LED zones yourself. |
| **Animation in real time** | Feed a stream of frames and drive the strip like a screen: music reactive, screen ambilight, whatever you write. |
| **Entirely on your network** | Commands go straight from your machine to the device. Govee's servers are not in the path. |

The per-segment channel is not in Govee's documentation. It was found through
reverse engineering and written up in
[`docs/protocol/lan.md`](docs/protocol/lan.md).

## Where the project is

The engine is working and verified on real hardware: discovery, on/off,
brightness, color, per-segment color and live animation. It is usable today
from Rust.

What comes next is the packaging around it — first the Python and Node.js
packages, then a web page and a desktop app with actual buttons, then Home
Assistant, Homebridge and Matter. [`docs/roadmap.md`](docs/roadmap.md) tracks
the order; watch the repository to hear when a piece lands.

## Will it work with my device?

**1. Find your model number.** It looks like `H61A0` — on the box, and in the
Govee app under your device's settings.

**2. Check Govee's LAN list.** 271 models ship a "LAN Control" switch;
they are mirrored in
[`docs/lan-supported-devices.md`](docs/lan-supported-devices.md). If yours is
there, the basics have a good chance of working.

**3. Turn LAN Control on.** Govee app → your device → settings → **LAN
Control**. That switch is what opens the door everything here walks through.

One model is confirmed end to end so far, the **H61A0**, segments included. The
other 270 are untested rather than unsupported — nobody has had one in hand yet,
so trying yours genuinely moves the project forward.
[`docs/compatibility.md`](docs/compatibility.md) tracks what is known.

## Getting started

Pick the line that sounds like you.

| You want to | Start here | |
| ----------- | ---------- | --- |
| Control your lights from **Rust** | [`packages/rust`](packages/rust) — install, first commands, segment streaming | ✅ |
| Control them from **Python** | [`packages/python`](packages/python) — what it will look like, and where it stands | 🚧 |
| Control them from **Node.js / TypeScript** | [`packages/node`](packages/node) — same | 🚧 |
| Tell us whether **your model works** | [`devices/README.md`](devices/README.md) — mostly filling in one file, no code | ✅ |
| Understand the **protocol** itself | [`docs/protocol/lan.md`](docs/protocol/lan.md) | ✅ |
| Click buttons instead of writing code | The web page and the desktop app are on the [roadmap](docs/roadmap.md) | 🔜 |

Whichever you pick, the commands you can send — `power`, `brightness`, `color`,
the segment channel — come from your device's file in [`devices/`](devices/),
not from names baked into an SDK. That is why adding support for a model is
editing one file rather than writing code in three languages.

## Three ways to reach a device

A command can travel to your light over your Wi-Fi (`lan`), over Bluetooth
(`ble`), or through Govee's servers (`cloud`). **You choose which ones to allow,
per device.**

| Mode | Speed | Reaches the device from | What it carries | |
| ---- | ----- | ----------------------- | --------------- | --- |
| `lan` | fastest | the same Wi-Fi | everything, segments included | ✅ |
| `ble` | fast | Bluetooth range, no Wi-Fi needed | depends on the model | 🚧 |
| `cloud` | slowest | anywhere with internet | on/off, brightness, color | 🔜 |

Every command reports which mode served it. Allow several and it switches
between them; allow one and it stays on that one — if the device is out of
reach, you get a clear error instead of a silent detour through a slower path,
and a segment animation is never approximated with a plain color change.

Details in [`docs/modes.md`](docs/modes.md).

<details>
<summary><b>Under the hood</b> — if you want to contribute code</summary>

The protocol is implemented **once**, in Rust; Python and Node.js bind to that
core rather than re-implementing it. Reasoning in
[`docs/architecture.md`](docs/architecture.md).

- [`devices/`](devices/) — one YAML file per model, the single source of truth.
  Command names, byte layouts and measured limits live here, never in code.
- [`packages/rust/src/codec`](packages/rust/src/codec) — device file plus
  arguments in, exact bytes out. No networking, so it is testable on its own.
- [`packages/rust/src/lan`](packages/rust/src/lan) — discovery, a device cache
  so a command never waits for a scan, one reused socket, and a per-device
  health state.
- [`packages/rust/src/stream`](packages/rust/src/stream) — the segment channel,
  armed once and fed frames at the resolution and rate measured on the unit.
- [`packages/rust/crates/sim`](packages/rust/crates/sim) — a fake Govee device
  with fault injection, so CI exercises all of the above without hardware.
- [`tests/fixtures/golden/`](tests/fixtures/golden/) — arguments in, exact bytes
  out. Every binding matches these, which is what keeps the ports aligned.

Full feature list: [`docs/features.md`](docs/features.md).

</details>

## FAQ

**Do I need a Govee account or an API key?**
Not for `lan`. Discovery and commands stay on your network. A key comes in only
with `cloud`.

**Can I still use the Govee app?**
Yes. Nothing here changes the device's configuration or locks anyone out.

**Is it safe for my lights?**
It sends the same kind of commands the official app sends. One deliberate
difference: a value outside the device's range is refused rather than adjusted.
The firmware clamps such values in silence, and reporting success for a setting
your device never applied would be worse than an error.

**My device is not showing up.**
Check LAN Control is on, and that your computer and the device are on the same
network — guest Wi-Fi and some mesh setups separate them.
[`docs/protocol/lan.md`](docs/protocol/lan.md) covers what discovery sends.

## Contributing

Confirming whether your model works needs no code —
[`devices/README.md`](devices/README.md) walks through it. For everything else,
[`CONTRIBUTING.md`](CONTRIBUTING.md), in particular how to document a newly
discovered command.

## Legal notice

Protocol reverse engineering is carried out for **interoperability** purposes.
The "Govee" trademark is used descriptively only, to identify compatible
devices. This project is not affiliated with, sponsored by, or endorsed by
Govee.

## License

[MIT](LICENSE)
