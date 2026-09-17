# Roadmap

What is planned. What already works is in [`features.md`](features.md); why the
work is shaped this way is in [`architecture.md`](architecture.md).

The order below is a starting point, not a commitment — it follows what people
ask for. Open an issue if something matters to you.

| # | Milestone | Status |
| - | --------- | ------ |
| 1 | Playground — backend, web UI, raw payload field | 🔜 Next |
| 2 | Electron app around the playground | 🔜 Planned |
| 3 | Matter bridge — one integration, every controller | 🔜 Planned |
| 4 | Home Assistant custom component (LAN power + brightness first) | 🔜 Planned |
| 5 | Homebridge plugin | 🔜 Planned |
| 6 | Art-Net / DMX bridge — [`dmx.md`](dmx.md) | 🔜 Planned |
| — | **Node** binding (napi-rs) | ✅ A promise-based API, TypeScript types, and prebuilt addons for Linux and Windows on `x86_64` and `aarch64` and for macOS on `aarch64` |

Undocumented LAN commands are documented and formalized continuously, in
[`protocol/lan.md`](protocol/lan.md) and `devices/*.yaml`, as they are
discovered — not as a numbered milestone.

## Device coverage

Per-SKU support grows with contributions and hardware access. See
[`../devices/README.md`](../devices/README.md) to add a SKU.
