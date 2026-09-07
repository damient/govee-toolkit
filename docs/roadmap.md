# Roadmap

What is planned. What already works is in [`features.md`](features.md); why the
work is shaped this way is in [`architecture.md`](architecture.md).

The order below is a starting point, not a commitment — it follows what people
ask for. Open an issue if something matters to you.

| # | Milestone | Status |
| - | --------- | ------ |
| 1 | **Node** binding (napi-rs) and **Python** binding (PyO3), with multi-arch wheels | 🚧 Next |
| 2 | Playground — backend, web UI, raw payload field | 🔜 Planned |
| 3 | Electron app around the playground | 🔜 Planned |
| 4 | Matter bridge — one integration, every controller | 🔜 Planned |
| 5 | Home Assistant custom component (LAN power + brightness first) | 🔜 Planned |
| 6 | Homebridge plugin | 🔜 Planned |
| 7 | Art-Net / DMX bridge | 🔜 Planned |
| — | `cloud` mode | 🔜 Planned, after the `lan` core |

Undocumented LAN commands are documented and formalized continuously, in
[`protocol/lan.md`](protocol/lan.md) and `devices/*.yaml`, as they are
discovered — not as a numbered milestone.

## Device coverage

Per-SKU support grows with contributions and hardware access. See
[`../devices/README.md`](../devices/README.md) to add a SKU.
