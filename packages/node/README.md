# govee-toolkit (Node.js)

Control Govee devices over the LAN from Node.js or TypeScript, including
undocumented commands observed on the wire.

**Documentation: [gvetk.com](https://gvetk.com)**

[![govee-toolkit on npm](https://img.shields.io/npm/v/govee-toolkit?logo=npm&logoColor=white&label=npm)](https://www.npmjs.com/package/govee-toolkit)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

> 🚧 **Being built.** The version on npm today is a `0.0.0` placeholder holding
> the name. The engine it binds to is working and verified on hardware, in
> [`packages/rust`][rust]. Watch the repository to hear when it lands.

## What it will be

A napi-rs binding over the Rust core — not a reimplementation. The protocol is
written once, so this package cannot drift from it: both are held to the same
conformance vectors, arguments in and exact bytes out. Prebuilt binaries per
platform, so there is no Rust toolchain to install. Types ship with the package.

```ts
// 🔜 Planned interface, mirroring the Rust API.
import { Govee } from "govee-toolkit";

const govee = await Govee.start();          // reads ~/.config/govee-toolkit/config.yaml

for (const device of await govee.scan()) {
  console.log(device.id, device.sku, device.modes);
}

const served = await govee.device(deviceId).send("power", { on: 1 });
console.log("served over", served.mode);
```

Command names — `power`, `brightness`, `color` — are entries in the device's
YAML file, not identifiers in this package. A name a device does not define, or
an argument outside its declared range, is an error before anything reaches the
network.

This package is also the transport for the rest of the JavaScript side:
`apps/playground`, `apps/desktop`, `packages/artnet-dmx-bridge` and
`integrations/homebridge`. None of them carry protocol code of their own.

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
[rust]: https://github.com/damient/govee-toolkit/tree/main/packages/rust
[devices-readme]: https://github.com/damient/govee-toolkit/blob/main/devices/README.md
