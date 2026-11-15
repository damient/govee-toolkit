---
title: Install
slug: install
order: 2
description: The Rust crate, the command line, the Python package, the Node.js package, and the DMX bridge.
---

# Install

The Rust crate is the reference implementation. The command line works straight
from a terminal. Python and Node.js bind to that same core. The DMX bridge is a
separate binary.

## Rust {{version_rust}} {{registry_rust}}

Async, on Tokio. The `lan` feature is on by default. `ble` and `cloud` are
opt-in.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo add govee-toolkit
<span class="prompt">$</span> cargo add govee-toolkit --features ble,cloud</code></pre>
<button class="copy" type="button" data-copy="cargo add govee-toolkit">Copy</button>
</div>

```rust
use govee_toolkit::{Config, Filter, Govee};

let govee = Govee::start(Config::load()?).await?;
let devices = govee.devices(Filter::all(), None).await?;
devices.power(true).await;
```

On Linux the `ble` feature reaches the radio through BlueZ over D-Bus, so the
build needs the dbus headers (`apt install libdbus-1-dev`). macOS and Windows
build as is.

## Command line {{version_cli}} {{registry_cli}}

The binary is `govee`. It reads the device files through the crate.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-cli
<span class="prompt">$</span> govee scan
<span class="prompt">$</span> govee on DEVICE
<span class="prompt">$</span> govee color DEVICE "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Copy</button>
</div>

[Every command, with an example in each language]({{base}}docs/reference/)

## Python {{version_python}} {{registry_python}}

A PyO3 binding over the Rust core. The API is `asyncio` only, and the wheel
carries the device catalog, so `pip install` is the whole install.

<div class="terminal">
<pre><code><span class="prompt">$</span> pip install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="pip install govee-toolkit">Copy</button>
</div>

```python
from govee_toolkit import Govee

govee = await Govee.start()
devices = await govee.devices()
await devices.power(True)
await govee.close()
```

The wheels are `abi3` for Python 3.11 and up: Linux, macOS and Windows on
`x86_64` and `aarch64`, Linux on `armv7`, and musl on `x86_64`, `aarch64` and
`armv7`. On any other platform, pip builds from the repository, which
needs a Rust toolchain.

## Node.js {{version_node}} {{registry_node}}

A napi-rs binding over the same core, with TypeScript types. The package
carries the device catalog, so `npm install` is the whole install.

<div class="terminal">
<pre><code><span class="prompt">$</span> npm install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="npm install govee-toolkit">Copy</button>
</div>

```javascript
import { Govee } from "govee-toolkit"

const govee = await Govee.start()
const devices = await govee.devices()
await devices.power(true)
await govee.close()
```

The package needs Node.js 20 or later. The addon ships for Linux and Windows on
`x86_64` and `aarch64`, and for macOS on `aarch64`. Another platform builds from
the repository, because the addon compiles the device catalog from
`devices/*.yaml`.

## DMX bridge {{version_dmx}} {{registry_dmx}}

The binary is `govee-dmx`. It receives Art-Net on the network and writes each
frame to a light over `lan`.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-dmx
<span class="prompt">$</span> govee-dmx patch
<span class="prompt">$</span> govee-dmx run</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-dmx">Copy</button>
</div>

Art-Net is the one input protocol the bridge carries. A light it drives must
have `lan` enabled.

[The DMX guide]({{base}}docs/dmx/)

## MCP server {{version_mcp}} {{registry_mcp}}

`govee-toolkit-mcp` lets an AI assistant read the device catalog and drive the
lights. Add it to your assistant, and `npx` fetches it on the first start:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copy</button>
</div>
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copy</button>
</div>
:::

[The MCP guide]({{base}}docs/mcp/)
