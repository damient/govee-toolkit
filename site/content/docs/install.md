---
title: Install
slug: install
order: 2
description: The Rust crate, the command line, the Python package, and where the Node.js package stands.
---

# Install

The Rust crate is the reference implementation. The command line needs no
code. Python binds to that same core. Node.js is not released yet.

## Rust <span class="state ok">Available</span>

Async, on Tokio. The `lan` feature is on by default. `ble` and `cloud` are
opt-in.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo add govee-toolkit
<span class="prompt">$</span> cargo add govee-toolkit --features ble,cloud</code></pre>
<button class="copy" type="button" data-copy="cargo add govee-toolkit">Copy</button>
</div>

```rust
use govee_toolkit::{Config, Govee};

let govee = Govee::start(Config::load()?).await?;
for device in govee.scan().await? {
    govee.device(device.id()).power(true).await?;
}
```

On Linux the `ble` feature reaches the radio through BlueZ over D-Bus, so the
build needs the dbus headers (`apt install libdbus-1-dev`). macOS and Windows
need nothing.

[Crate documentation]({{repo}}/tree/main/packages/rust) ·
[crates.io](https://crates.io/crates/govee-toolkit)

## Command line <span class="state ok">Available</span>

The binary is `govee`. It holds no protocol logic: it reads the device files
through the crate.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-cli
<span class="prompt">$</span> govee scan
<span class="prompt">$</span> govee on DEVICE
<span class="prompt">$</span> govee color DEVICE "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Copy</button>
</div>

[Every command, with an example in each language]({{base}}reference/)

## Python <span class="state ok">Available</span>

A PyO3 binding over the Rust core. The API is `asyncio` only, and the wheel
carries the device catalog, so an install needs no data file and no Rust
toolchain.

<div class="terminal">
<pre><code><span class="prompt">$</span> pip install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="pip install govee-toolkit">Copy</button>
</div>

```python
from govee_toolkit import Govee

govee = await Govee.start()
for device in await govee.scan():
    await govee.device(device.id).power(True)
await govee.close()
```

The wheels are `abi3` for Python 3.11 and up: Linux, macOS and Windows on
`x86_64` and `aarch64`, Linux on `armv7`, and musl on `x86_64`, `aarch64` and
`armv7`. On a platform with no wheel, pip builds from the repository, which
needs a Rust toolchain.

[Package documentation]({{repo}}/tree/main/packages/python) ·
[PyPI](https://pypi.org/project/govee-toolkit/)

## Node.js <span class="state soon">Planned</span>

A napi-rs binding over the same core, with TypeScript types. The binding is
written and the addon builds for Linux and Windows on `x86_64` and `aarch64`
and for macOS on `aarch64`. The name `govee-toolkit` on npm still carries a
`0.0.0` placeholder: the first release is ahead.

```javascript
import { Govee } from "govee-toolkit"

const govee = await Govee.start()
for (const device of await govee.scan()) {
  await govee.device(device.id).power(true)
}
await govee.close()
```

[Where it sits in the order of work]({{repo}}/blob/main/docs/roadmap.md)
