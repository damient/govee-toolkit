---
title: Install
slug: install
order: 2
description: The Rust crate, the command line, and where the Python and Node.js packages stand.
---

# Install

The Rust crate is the reference implementation. The command line needs no
code. Python and Node.js bind to that same core and are not released yet.

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
<span class="prompt">$</span> govee on living-room
<span class="prompt">$</span> govee color living-room "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Copy</button>
</div>

[Every command, with an example in each language]({{base}}reference/)

## Python <span class="state soon">Planned</span>

A PyO3 binding over the Rust core, with wheels for the usual platforms. The
name `govee-toolkit` is reserved on PyPI by a `0.0.0` placeholder: there is no
code behind it yet.

```python
# What it is meant to look like.
govee = await Govee.start()
for device in await govee.scan():
    await govee.device(device.id).power(True)
```

[Where it sits in the order of work]({{repo}}/blob/main/docs/roadmap.md)

## Node.js <span class="state soon">Planned</span>

A napi-rs binding over the same core, with TypeScript types. The name
`govee-toolkit` is reserved on npm by a `0.0.0` placeholder: there is no code
behind it yet.

```javascript
// What it is meant to look like.
const govee = await Govee.start()
for (const device of await govee.scan()) {
  await govee.device(device.id).power(true)
}
```

[Where it sits in the order of work]({{repo}}/blob/main/docs/roadmap.md)
