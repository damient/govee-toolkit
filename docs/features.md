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
| ✅ | **Discovery** — multicast scan at startup, periodic background refresh, persistent on-disk cache |
| 🚧 | **Undocumented LAN commands** — the raw segment channel is documented and verified on one device. Each further command needs the same treatment once somebody finds it |
| ✅ | **Segment streaming** (`packages/rust/src/stream`) — the stream arms the raw segment channel once, then feeds it frames. The rate comes from the frame rate measured on the unit. A mode that paints whole frames reaches its native resolution. A mode that paints by zone mask reaches its zones |
| ✅ | **Per-device mode selection** — one mode for strict behavior, several for switching, see [`modes.md`](modes.md) |
| ✅ | **Per-device, per-mode circuit breaker** — `OK` / `DEGRADED` / `DOWN`. The breaker decides from state it already holds, not from a fresh timeout per call |
| ✅ | **Explicit failures** — a command a mode cannot serve fails and says so. The SDK never approximates one in silence |

## Modes

| | Feature |
| --- | ------- |
| ✅ | **`lan`** — power, brightness, color and color temperature over UDP, plus per-segment color |
| ✅ | **`ble`** — opt-in, works off-network within Bluetooth range, partial coverage per SKU family, one connection per device. A budget paces the writes. The transport ships behind the `ble` cargo feature, off by default. Power, brightness, color, color temperature, both per-zone channels and the reads are verified on one device, the H61A0. Wi-Fi provisioning is encoded from the layout and has never been sent to a device. Scenes are not implemented |
| 🔜 | **`cloud`** — opt-in, reaches a device from anywhere, throttled, reduced to power / brightness / color |

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
| 🔜 | **Web playground** — device list with per-mode state badges, power / brightness / color / scenes controls, latency log |
| 🔜 | **Raw payload field** — send a custom JSON command straight to a device, to try a discovery before you formalize it |
| 🔜 | **Desktop app (Electron)** — same backend and UI as the playground, auto-discovery on launch, tray icon |
| 🚧 | **Device simulator** (`packages/rust/crates/sim`) — fake Govee device on UDP with fault injection, so tests run without hardware. It serves `lan` only. It does not advertise a BLE peripheral, so a `ble` test still needs an adapter |
| 🔜 | **Art-Net / DMX bridge** — maps DMX channels to Govee devices and segments |

## Integrations

| | Feature |
| --- | ------- |
| 🔜 | **Matter bridge** — one integration, reachable from any Matter controller |
| 🔜 | **Home Assistant** — custom component distributable through HACS, carries the LAN scenes and segments Matter cannot express |
| 🔜 | **Homebridge** — HomeKit plugin |
