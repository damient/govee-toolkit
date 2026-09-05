# govee-toolkit (Python)

Control Govee devices over the LAN from Python, including undocumented commands
found through reverse engineering. Unofficial, and not affiliated with Govee.

> 🚧 **Being built.** The version on PyPI today is a `0.0.0` placeholder holding
> the name. What is described below is the interface this package ships with;
> the engine it binds to is working and verified on hardware, in
> [`packages/rust`][rust]. Watch the repository to hear when the wheels land.

## What it will be

A PyO3 binding over the Rust core — not a reimplementation. The protocol is
written once, so this package cannot drift from it: both are held to the same
conformance vectors, arguments in and exact bytes out.

Concretely that means no Python-side protocol code, no `asyncio` UDP handling to
maintain, and the same undocumented segment channel the Rust crate has.

```bash
pip install govee-toolkit   # 🔜 not yet — placeholder version on PyPI today
```

Multi-arch wheels, so there is no Rust toolchain to install.

## The shape it ships with

```python
# 🔜 Planned interface, mirroring the Rust API.
import asyncio
from govee_toolkit import Govee

async def main():
    govee = await Govee.start()             # reads ~/.config/govee-toolkit/config.yaml

    for device in await govee.scan():
        print(device.id, device.sku, device.modes)

    served = await govee.device(device_id).send("power", on=1)
    print("served over", served.mode)

asyncio.run(main())
```

Command names — `power`, `brightness`, `color` — are entries in the device's
YAML file in [`devices/`][devices], not identifiers in this package. A name a
device does not define, or an argument outside its declared range, is an error
before anything reaches the network.

## Modes

`lan` is the default. `ble` and `cloud` are opt-in and enabled per device by the
user, in the shared config file — see [`docs/modes.md`][modes]. Enabling a mode
that has no transport yet is reported as such, never silently skipped.

## Meanwhile

- The same operations work today from Rust: [`packages/rust`][rust].
- Confirming whether your own device model works needs no code at all:
  [`devices/README.md`][devices-readme].

<!-- Absolute: this file is the package description on PyPI, where a relative
     link out of the package directory is dead. -->
[rust]: https://github.com/damient/govee-toolkit/tree/main/packages/rust
[modes]: https://github.com/damient/govee-toolkit/blob/main/docs/modes.md
[devices]: https://github.com/damient/govee-toolkit/tree/main/devices
[devices-readme]: https://github.com/damient/govee-toolkit/blob/main/devices/README.md
