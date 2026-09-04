# Architecture

One core, in Rust. Every other language reaches it through a binding rather than
re-implementing it.

## Why one core

The protocol is small and stable: UDP, a JSON envelope, and one variable-length
raw frame with an XOR checksum ([`protocol/lan.md`](protocol/lan.md)). What
actually changes is the data — new SKUs, new opcodes, new per-family
capabilities. That split is what makes a single compiled core worth it: the part
that would be duplicated barely moves, and the part that moves constantly is
YAML anyone can write.

Three ports of the same protocol means three places a frame can be built wrong.
One core means one.

Rust for the reasons the fast path needs: no garbage collector pausing a
segment stream, a socket per device and a breaker per mode expressed without
data races, and a single static binary to embed or ship.

## Layers

```
devices/*.yaml          data — what a SKU does, and the bytes for it
       │
govee-core              codec — no I/O, no SKU names, no command names
       │
govee-lan               transport — UDP, discovery, breaker, rate limiting
       │
govee                   facade — modes, configuration, events
       │
  ┌────┴────┬──────────┬──────────┐
node       python      cli       php
(napi-rs)  (PyO3)              (port)
```

`govee-core` interprets the device files; it contains no per-SKU logic and never
will. Adding a device is adding YAML — see
[`../devices/README.md`](../devices/README.md).

## Bindings, and the one port

- **Node** — `napi-rs`. Serves the playground, the Electron app and the
  Homebridge plugin.
- **Python** — `PyO3` / `maturin`. Serves the Home Assistant component. It needs
  wheels for aarch64, armv7 and musl, or Raspberry Pi users cannot install it;
  that CI is part of the work, not an afterthought.
- **PHP** — a hand-written port. `ext-php-rs` produces an extension that has to
  be compiled on the host, which is not something to ask of a Composer install.
  PHP is the one implementation that can drift, which is exactly why the
  conformance vectors exist.

## Conformance vectors

`tests/fixtures/golden/` holds arguments-in, bytes-out vectors. Every
implementation runs them and must produce identical output — the same envelope,
the same frame, and the same failure code for a call that must fail. A port
that drifts fails there before it reaches hardware. See
[`../tests/fixtures/README.md`](../tests/fixtures/README.md).

## What this costs

Contributors adding a SKU write YAML and never touch Rust. Contributors adding a
*command shape* the frame language cannot express — an unusual checksum, a
conditional field — do have to touch the core. The language is deliberately
broader than the one device that exists today so that stays rare.
