# govee-toolkit (Node.js)

Control Govee devices over the LAN from Node.js or TypeScript, including
undocumented commands observed on the wire.

**Documentation: [gvetk.com](https://gvetk.com)**

[![govee-toolkit on npm](https://img.shields.io/npm/v/govee-toolkit?logo=npm&logoColor=white&label=npm)](https://www.npmjs.com/package/govee-toolkit)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

> 🚧 **The first release is ahead.** The binding below is written and tested,
> and the version on npm today is still the `0.0.0` placeholder that holds the
> name. Watch the repository to hear when it lands.

## What it is

A [napi-rs](https://napi.rs) binding over the Rust core — not a
reimplementation. The protocol is written once, so this package cannot drift
from it: both are held to the same conformance vectors, arguments in and exact
bytes out. One prebuilt addon per platform, so there is no Rust toolchain to
install. Types ship with the package.

```ts
import { Govee } from "govee-toolkit";

const govee = await Govee.start(); // reads ~/.config/govee-toolkit/config.yaml

for (const device of await govee.scan()) {
  console.log(device.id, device.sku, device.modes);
}

const served = await govee.device(deviceId).send("power", { on: 1 });
console.log("served over", served.mode);

await govee.close();
```

Command names — `power`, `brightness`, `color` — are entries in the device's
YAML file, not identifiers in this package. A name a device does not define, or
an argument outside its declared range, is an error before anything reaches the
network.

## Modes are explicit

`lan`, `ble` and `cloud` are modes the user enables per device. One enabled
mode means one mode: a device no enabled mode reaches fails and says so.
Nothing falls back.

## What a failure carries

Every failure is a JavaScript `Error` with two properties:

- `code` — the stable identifier the core gives the failure, such as
  `unknown_device` or `zone_count_mismatch`. Match on this.
- `name` — the family: `CodecError` where nothing was sent, `TransportError`
  where a mode failed to carry the command, `ConfigError` where the
  configuration cannot be applied.

A value this package refuses before the core sees it is a `TypeError`, with
the code `invalid_argument`.

## Platforms

Prebuilt addons for Linux and Windows on `x86_64` and `aarch64`, and for macOS
on `aarch64`. npm installs the one that matches. There is no source build: the
device catalog is compiled into the addon from the repository, so a platform
with no addon needs the checkout.

## It is also the JavaScript transport

`apps/playground`, `apps/desktop`, `packages/artnet-dmx-bridge` and
`integrations/homebridge` all go through this package. None of them carry
protocol code of their own.

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

<!-- Absolute: this file is the package description on npm, where a relative
     link out of the package directory is dead. -->
[devices-readme]: https://github.com/damient/govee-toolkit/blob/main/devices/README.md
