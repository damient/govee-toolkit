# Roadmap

What is planned. What already works is in [`features.md`](features.md); why the
work is shaped this way is in [`architecture.md`](architecture.md).

The order below is a starting point, not a commitment — it follows what people
ask for. Open an issue if something matters to you.

| # | Milestone | Status |
| - | --------- | ------ |
| 1 | **Node** binding (napi-rs), with multi-arch builds | 🚧 Next |
| 2 | **Python** wheels for `armv7` and for musl | 🔜 Planned |
| 3 | Playground — backend, web UI, raw payload field | 🔜 Planned |
| 4 | Electron app around the playground | 🔜 Planned |
| 5 | Matter bridge — one integration, every controller | 🔜 Planned |
| 6 | Home Assistant custom component (LAN power + brightness first) | 🔜 Planned |
| 7 | Homebridge plugin | 🔜 Planned |
| 8 | Art-Net / DMX bridge | 🔜 Planned |
| — | **Python** binding (PyO3) | ✅ The `asyncio` API, and `abi3` wheels for Linux, macOS and Windows on `x86_64` and `aarch64` |
| — | `cloud` mode — the documented HTTPS API | ✅ Power, brightness, color, temperature, state, segments and music |

Undocumented LAN commands are documented and formalized continuously, in
[`protocol/lan.md`](protocol/lan.md) and `devices/*.yaml`, as they are
discovered — not as a numbered milestone.

## Device coverage

Per-SKU support grows with contributions and hardware access. See
[`../devices/README.md`](../devices/README.md) to add a SKU.
