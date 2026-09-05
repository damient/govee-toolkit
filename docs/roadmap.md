# Roadmap

What is done and what is planned. The full feature list is in
[`features.md`](features.md); why the work is shaped this way is in
[`architecture.md`](architecture.md).

The order below is a starting point, not a commitment — it will follow what
people actually ask for. Open an issue if something matters to you.

| # | Milestone | Status |
| - | --------- | ------ |
| 1 | Repository scaffold — full tree, MIT license, CI skeleton | ✅ Done |
| 2 | `devices/schema.yaml` + the first device entry (H61A0) | ✅ Done |
| 3 | `docs/protocol/lan.md` — documented protocol + the undocumented commands found so far | ✅ Done |
| 4 | The protocol codec in **Rust** — device catalog, command encoding, raw frame codec, conformance vectors | ✅ Done |
| 5 | `lan` transport in Rust — scan, cache, reused socket, per-mode circuit breaker, mode selection | ✅ Done |
| 5b | Segment streaming — `razer` channel, native resolution, rate limiting from zone count | ✅ Done |
| 6 | **Node** binding (napi-rs) and **Python** binding (PyO3), with multi-arch wheels | 🚧 Next |
| 7 | Playground — backend, web UI, raw payload field | 🔜 Planned |
| 8 | Electron app around the playground | 🔜 Planned |
| 9 | Matter bridge — one integration, every controller | 🔜 Planned |
| 10 | Home Assistant custom component (LAN power + brightness first) | 🔜 Planned |
| 11 | Homebridge plugin | 🔜 Planned |
| 12 | Art-Net / DMX bridge | 🔜 Planned |
| — | `ble` and `cloud` modes | 🔜 Planned, after the `lan` core |

Undocumented LAN commands are documented and formalized continuously, in
[`protocol/lan.md`](protocol/lan.md) and `devices/*.yaml`, as they are
discovered — not as a numbered milestone.

## Device coverage

Per-SKU support grows with contributions and hardware access. See
[`../devices/README.md`](../devices/README.md) to add a SKU.
