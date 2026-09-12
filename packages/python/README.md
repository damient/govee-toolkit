# govee-toolkit (Python)

Control Govee devices over the LAN from Python, including undocumented commands
observed on the wire.

**Documentation: [gvetk.com](https://gvetk.com)**

[![govee-toolkit on PyPI](https://img.shields.io/pypi/v/govee-toolkit?logo=python&logoColor=white&label=PyPI)](https://pypi.org/project/govee-toolkit/)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

> 🚧 **Being built.** The version on PyPI today is a `0.0.0` placeholder holding
> the name. The engine it binds to is working and verified on hardware, in
> [`packages/rust`][rust]. Watch the repository to hear when the wheels land.

## What it will be

A PyO3 binding over the Rust core — not a reimplementation. The protocol is
written once, so this package cannot drift from it: both are held to the same
conformance vectors, arguments in and exact bytes out. Multi-arch wheels, so
there is no Rust toolchain to install.

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
YAML file, not identifiers in this package. A name a device does not define, or
an argument outside its declared range, is an error before anything reaches the
network.

## Meanwhile

| You want to | Go to |
| ----------- | ----- |
| Send a command today, from Rust or the terminal | [gvetk.com/docs/start](https://gvetk.com/docs/start/) |
| Know whether your model works | [gvetk.com/devices](https://gvetk.com/devices/) |
| Pick and configure the modes | [gvetk.com/docs/modes](https://gvetk.com/docs/modes/) |
| Read every command and method | [gvetk.com/reference](https://gvetk.com/reference/) |
| Report a device, or add one | [`devices/README.md`][devices-readme] |

## License

[MIT](https://github.com/damient/govee-toolkit/blob/main/LICENSE)

<!-- Absolute: this file is the package description on PyPI, where a relative
     link out of the package directory is dead. -->
[rust]: https://github.com/damient/govee-toolkit/tree/main/packages/rust
[devices-readme]: https://github.com/damient/govee-toolkit/blob/main/devices/README.md
