# govee-toolkit (Python)

Control Govee devices over the LAN from Python, with the undocumented commands
observed on the wire.

**Documentation: [gvetk.com](https://gvetk.com)**

[![govee-toolkit on PyPI](https://img.shields.io/pypi/v/govee-toolkit?logo=python&logoColor=white&label=PyPI)](https://pypi.org/project/govee-toolkit/)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

The package is a [PyO3][pyo3] binding over the Rust core in
[`packages/rust`][rust], not a second implementation of the protocol. It builds
no frame of its own: the core builds every byte, and the conformance vectors in
the repository check the core. Arguments in, exact bytes out.

## Install

```console
pip install govee-toolkit
```

Python 3.10 and up. The wheel is `abi3`, so one wheel serves every version from
3.10. The release builds wheels for Linux, macOS and Windows on `x86_64` and
`aarch64`, for Linux on `armv7`, and for musl on `x86_64`, `aarch64` and
`armv7`.

The release publishes no source distribution. On a platform with no wheel, build
the wheel from a checkout of the repository with a Rust toolchain. The core
embeds the device files at build time, so a build needs the `devices/`
directory.

The wheel carries the device catalog, so there is no data file to install.
`govee_toolkit.CORE_VERSION` reports the version of the core that the wheel was
built from.

## A first command

The API is `asyncio` only.

```python
import asyncio
from govee_toolkit import Govee

async def main():
    # Reads $XDG_CONFIG_HOME/govee-toolkit/config.yaml.
    govee = await Govee.start()

    devices = await govee.scan()
    for device in devices:
        print(device.id, device.sku, device.modes)

    handle = govee.device(devices[0].id)
    served = await handle.send("power", on=True)
    print("served over", served.mode)

    await govee.close()

asyncio.run(main())
```

`handle.send()` takes any command the device file declares. The common ones also
have a method: `power()`, `brightness()`, `color()`, `color_temp()`, `music()`,
`segment()`, `gradient()` and `provision_wifi()`. `handle.read()` and
`handle.status()` ask the device instead. `handle.open_stream()` opens a segment
stream and paints frame by frame.

## Modes

A command travels over your Wi-Fi (`lan`), over Bluetooth (`ble`) or through
Govee's servers (`cloud`). You enable the modes per device, in the configuration
file. One enabled mode means one mode: an unreachable device fails with an error
and says so. The package never substitutes a mode.

`handle.serving_mode()` names the mode a command goes over now, from the state
the SDK recorded. `await handle.ensure_known()` scans first if no mode knows the
device yet, then answers the same question.
[gvetk.com/docs/modes](https://gvetk.com/docs/modes/) has the rules.

## Where command names come from

`power`, `brightness` and `color` are entries in the device's YAML file in
[`devices/`][devices], not identifiers in this package. `handle.spec()` returns
what that file declares for your device. A name the device does not define, or
an argument outside the declared range, is an error before anything reaches the
network.

## Errors

`GoveeError` is the base class; `CodecError`, `TransportError` and `ConfigError`
are its subclasses. Every one of them carries `.code`, the stable identifier
that the core gives the failure. Match on the code, not on the message: the
message is written for a person and can change.

A value the package cannot read at all, such as a color that is not three whole
numbers, is a `ValueError`. It carries no code, because the core never saw it.

```python
from govee_toolkit import GoveeError

try:
    await handle.brightness(50)
except GoveeError as err:
    print(err.code)  # for example "unknown_command" or "no_mode_available"
```

## Next

| You want to | Go to |
| ----------- | ----- |
| Install it and send a first command | [gvetk.com/docs/start](https://gvetk.com/docs/start/) |
| Know whether your model works | [gvetk.com/devices](https://gvetk.com/devices/) |
| Pick and configure the modes | [gvetk.com/docs/modes](https://gvetk.com/docs/modes/) |
| Read every command and method | [gvetk.com/reference](https://gvetk.com/reference/) |
| Report a device, or add one | [`devices/README.md`][devices-readme] |

## License

[MIT](https://github.com/damient/govee-toolkit/blob/main/LICENSE)

<!-- Absolute: this file is the package description on PyPI, where a relative
     link out of the package directory is dead. -->
[pyo3]: https://pyo3.rs
[rust]: https://github.com/damient/govee-toolkit/tree/main/packages/rust
[devices]: https://github.com/damient/govee-toolkit/tree/main/devices
[devices-readme]: https://github.com/damient/govee-toolkit/blob/main/devices/README.md
