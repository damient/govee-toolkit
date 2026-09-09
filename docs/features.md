# Features

Everything the toolkit does, and everything it is meant to do. What ships when
is in [`roadmap.md`](roadmap.md).

Legend: ✅ available · 🚧 in progress · 🔜 planned

## Core

| | Feature |
| --- | ------- |
| ✅ | **Device database** (`devices/*.yaml`) — schema and per-SKU definitions, the single source of truth every SDK reads |
| ✅ | **Protocol documentation** (`docs/protocol/`) — `lan`, `ble` and `cloud`, with the undocumented findings |
| ✅ | **Official LAN-capable device list** mirrored offline ([`lan-supported-devices.md`](lan-supported-devices.md)) |
| ✅ | **Protocol core** (`packages/rust/src/codec`) — device files and arguments in, exact bytes out. Raw segment frames included. It does no I/O |
| ✅ | **Runtime configuration** — enabled modes per device, in YAML, see [`modes.md`](modes.md) |
| ✅ | **Conformance vectors** (`tests/fixtures/golden/`) — the contract every implementation must match, so a port cannot drift unnoticed |
| ✅ | **`lan` mode, low latency** — reused UDP socket, fire-and-verify, no cloud round-trip |
| ✅ | **Per-device throttle on `cloud`** — one request per device per interval, and an explicit failure rather than an invisible queue |
| ✅ | **Discovery** — multicast scan at startup, periodic background refresh, persistent on-disk cache |
| 🚧 | **Undocumented LAN commands** — the raw segment channel is documented and verified on one device. Each further command needs the same treatment once somebody finds it |
| ✅ | **Segment streaming** (`packages/rust/src/stream`) — frames over the raw segment channel, at the rate measured on the unit |
| ✅ | **Per-device mode selection** — one mode for strict behavior, several for switching, see [`modes.md`](modes.md) |
| ✅ | **Per-device, per-mode circuit breaker** — `OK` / `DEGRADED` / `DOWN`. The breaker decides from state it already holds, not from a fresh timeout per call |
| ✅ | **Explicit failures** — a command a mode cannot serve fails and says so. The SDK never approximates one in silence |
| ✅ | **Wi-Fi provisioning** (`provision_wifi()`) — puts a device out of the box on a network over `ble`, which is what makes `lan` reachable |

## Modes

| | Feature |
| --- | ------- |
| ✅ | **`lan`** — power, brightness, color and color temperature over UDP, plus per-segment color |
| ✅ | **`ble`** — power, brightness, color and color temperature over GATT, plus per-zone color. Off-network, within radio range, behind the `ble` cargo feature |
| 🚧 | **`cloud`** — opt-in, the documented HTTPS API. Reaches any device the account owns from anywhere, throttled, reduced to power / brightness / color / color temperature. Behind the `cloud` cargo feature |

## SDKs

One core, in Rust. The other packages bind to it — [`architecture.md`](architecture.md).

| | Feature |
| --- | ------- |
| 🚧 | **Rust** (`packages/rust`) — the reference implementation and the only place protocol logic lives |
| 🔜 | **Python** (`packages/python`) — PyO3 binding, pip, `pytest`, multi-arch wheels |
| 🔜 | **Node.js / TypeScript** (`packages/node`) — napi-rs binding, npm |
| 🔜 | Each package versioned and released independently (`rust-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`) |

## Tools & apps

| | Feature |
| --- | ------- |
| 🔜 | **Web playground** — device list with per-mode state badges, power / brightness / color controls, latency log |
| 🔜 | **Raw payload field** — send a custom JSON command straight to a device, to try a discovery before you formalize it |
| 🔜 | **Desktop app (Electron)** — same backend and UI as the playground, auto-discovery on launch, tray icon |
| ✅ | **Device simulator** (`packages/rust/crates/sim`) — a fake `lan` device on UDP and a fake `ble` peripheral on GATT, both with fault injection, so tests run without hardware or an adapter |
| 🔜 | **Art-Net / DMX bridge** — maps DMX channels to Govee devices and segments |

## Integrations

| | Feature |
| --- | ------- |
| 🔜 | **Matter bridge** — one integration, reachable from any Matter controller |
| 🔜 | **Home Assistant** — custom component distributable through HACS, carries the LAN segment channel Matter cannot express |
| 🔜 | **Homebridge** — HomeKit plugin |
