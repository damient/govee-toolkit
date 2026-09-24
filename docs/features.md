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
| ✅ | **Undocumented LAN commands** — the raw segment channel is documented and verified on one device. Each further command needs the same treatment once somebody finds it |
| ✅ | **Segment streaming** (`packages/rust/src/stream`) — frames over the raw segment channel, at the rate measured on the unit |
| ✅ | **Per-device mode selection** — one mode for strict behavior, several for switching, see [`modes.md`](modes.md) |
| ✅ | **Per-device, per-mode circuit breaker** — `OK` / `DEGRADED` / `DOWN`. The breaker decides from state it already holds, not from a fresh timeout per call |
| ✅ | **Explicit failures** — a command a mode cannot serve fails and says so. The SDK never approximates one in silence |
| ✅ | **White temperature** (`color_temp()`) — one call sets the kelvin value and, where the frame carries it, the RGB rendering the firmware does not compute |
| ✅ | **Music** (`music()`) — plays an effect the device renders from its own microphone, over `ble` and `cloud`. `lan` carries no music command |
| ✅ | **Identify** (`identify()`) — powers one device on and paints one color, so a person maps an identity to a fixture in the room. `govee identify` and `govee-dmx identify` walk a whole rig, or the devices a target names: an identity, a SKU, a name, a group, and on the bridge a universe and a channel |
| ✅ | **Names and groups** — `name:` and `groups:` in `config.yaml` give targets in place of an identity. A verb on a group goes to every member at once, and a member that fails stops no other one (`Govee::group()`, `govee on <group>`) |
| ✅ | **Wi-Fi provisioning** (`provision_wifi()`) — puts a device out of the box on a network over `ble`, which is what makes `lan` reachable |

## Modes

| | Feature |
| --- | ------- |
| ✅ | **`lan`** — the undocumented UDP protocol on the local network. Reaches a device on the same network, with no cloud round-trip |
| ✅ | **`ble`** — GATT over Bluetooth Low Energy, behind the `ble` cargo feature. Reaches a device off the network, within radio range |
| ✅ | **`cloud`** — the documented HTTPS API, behind the `cloud` cargo feature. Reaches any device the account owns |

## SDKs

One core, in Rust. The other packages bind to it — [`architecture.md`](architecture.md).

| | Feature |
| --- | ------- |
| ✅ | **Rust** (`packages/rust`) — the reference implementation and the only place protocol logic lives |
| ✅ | **Python** (`packages/python`) — PyO3 binding, an `asyncio` API, `abi3` wheels for Linux, macOS and Windows on `x86_64` and `aarch64`, and Linux wheels for `armv7` and for musl |
| ✅ | **Node.js / TypeScript** (`packages/node`) — napi-rs binding, a promise-based API and TypeScript types; prebuilt addons for Linux and Windows on `x86_64` and `aarch64` and for macOS on `aarch64` |
| ✅ | Each package versioned and released independently (`rust-vX.Y.Z`, `cli-vX.Y.Z`, `dmx-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`) |

## Tools & apps

| | Feature |
| --- | ------- |
| 🔜 | **Web playground** — device list with per-mode state badges, power / brightness / color controls, latency log |
| 🔜 | **Desktop app (Electron)** — same backend and UI as the playground, auto-discovery on launch, tray icon |
| ✅ | **Device simulator** (`packages/rust/crates/sim`) — a fake `lan` device on UDP and a fake `ble` peripheral on GATT, both with fault injection, so tests run without hardware or an adapter |
| ✅ | **DMX bridge** (`packages/rust/crates/dmx`) — the `govee-dmx` node receives DMX on the network and drives devices and segments over `lan`. Art-Net carries the DMX in. See [`dmx.md`](dmx.md) |

## Integrations

| | Feature |
| --- | ------- |
| 🔜 | **Matter bridge** — one integration, reachable from any Matter controller |
| 🔜 | **Home Assistant** — custom component distributable through HACS, carries the LAN segment channel Matter cannot express |
| 🔜 | **Homebridge** — HomeKit plugin |
