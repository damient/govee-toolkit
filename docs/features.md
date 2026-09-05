# Features

Everything the toolkit does or is meant to do, in one place. What ships when is
in [`roadmap.md`](roadmap.md).

Legend: ✅ available · 🚧 in progress · 🔜 planned

## Core

| | Feature |
| --- | ------- |
| ✅ | **Device database** (`devices/*.yaml`) — schema and per-SKU definitions, the single source of truth every SDK reads |
| ✅ | **Protocol documentation** (`docs/protocol/`) — `lan`, `ble`, `cloud`, including the undocumented findings |
| ✅ | **Official LAN-capable device list** mirrored offline ([`lan-supported-devices.md`](lan-supported-devices.md)) |
| ✅ | **Protocol core** (`packages/rust/src/codec`) — device files plus arguments in, exact bytes out; raw segment frames included, no I/O |
| ✅ | **Runtime configuration** — enabled modes per device, in YAML, see [`modes.md`](modes.md) |
| ✅ | **Conformance vectors** (`tests/fixtures/golden/`) — the contract every implementation must match, so a port cannot drift unnoticed |
| ✅ | **`lan` mode, low latency** — reused UDP socket, fire-and-verify, no cloud round-trip |
| ✅ | **Discovery** — multicast scan at startup, periodic background refresh, persistent on-disk cache |
| 🚧 | **Undocumented LAN commands** — the raw segment channel is documented and verified on one device; more to formalize as they are found |
| ✅ | **Segment streaming** (`packages/rust/src/stream`) — the raw segment channel armed once and fed frames, at the native resolution of the unit, paced from the frame rate measured on it |
| ✅ | **Per-device mode selection** — one mode for strict behavior, several for switching, see [`modes.md`](modes.md) |
| ✅ | **Per-device, per-mode circuit breaker** — `OK` / `DEGRADED` / `DOWN`, decided from known state rather than a fresh timeout per call |
| ✅ | **Explicit failures** — a command a mode cannot serve fails and says so; nothing is silently approximated |

## Modes

| | Feature |
| --- | ------- |
| ✅ | **`lan`** — power, brightness, color and color temperature over UDP, plus per-segment color |
| 🔜 | **`ble`** — opt-in, works off-network within Bluetooth range, partial coverage per SKU family |
| 🔜 | **`cloud`** — opt-in, reaches a device from anywhere, throttled, reduced to power / brightness / color |

## SDKs

One core, in Rust; the rest bind to it — [`architecture.md`](architecture.md).

| | Feature |
| --- | ------- |
| 🚧 | **Rust** (`packages/rust`) — the reference implementation and the only place protocol logic lives |
| 🔜 | **Python** (`packages/python`) — PyO3 binding, pip, `pytest`, multi-arch wheels |
| 🔜 | **Node.js / TypeScript** (`packages/node`) — napi-rs binding, npm |
| 🔜 | Each package versioned and released independently (`rust-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`) |

## Tools & apps

| | Feature |
| --- | ------- |
| 🔜 | **Web playground** — device list with per-mode state badges, power / brightness / color / scenes controls, latency log |
| 🔜 | **Raw payload field** — send a custom JSON command straight to a device, to try a discovery before formalizing it |
| 🔜 | **Desktop app (Electron)** — same backend and UI as the playground, auto-discovery on launch, tray icon |
| 🚧 | **Device simulator** (`packages/rust/crates/sim`) — fake Govee device on UDP with fault injection, so tests run without hardware; BLE still to come |
| 🔜 | **Art-Net / DMX bridge** — maps DMX channels to Govee devices and segments |

## Integrations

| | Feature |
| --- | ------- |
| 🔜 | **Matter bridge** — one integration, reachable from any Matter controller |
| 🔜 | **Home Assistant** — custom component distributable through HACS, carries the LAN scenes and segments Matter cannot express |
| 🔜 | **Homebridge** — HomeKit plugin |
