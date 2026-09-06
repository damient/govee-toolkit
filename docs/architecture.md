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
devices/*.yaml             data — what a SKU does, and the bytes for it
       │
src/codec/                 codec — no I/O, no SKU names, no command names
       │
src/transport/             shared by every mode — the Transport trait, device
       │                   identity, errors, the breaker
       │
src/lan/                   transport — UDP, discovery, device cache
src/ble/                   transport — GATT, scan, one link, paced writes
       │
src/stream/                segment channel — armed once, fed frames on a clock
       │
src/ (crate root)          facade — modes, configuration, events
       │
  ┌────┴────┐
node       python
(napi-rs)  (PyO3)
```

**One crate, `govee-toolkit`**, at `packages/rust`. The layers are modules of
it, and what is optional is a cargo feature rather than a separate package:
`lan` is on by default, `ble` is opt-in, and `cloud` joins them when it lands.
Two crates live beside it and are never published: `crates/sim`, the device
simulator, and `crates/xtask`, which generates the distributable catalog.

The bare name `govee` on crates.io belongs to an unrelated project, which is why
this one carries the longer name.

### Why one crate and not four

One package is the simpler thing to publish, to version and to explain —
`cargo add govee-toolkit`, one version, one tag — and a Rust user gains nothing
from seeing three names on crates.io when two of them are transitive.

Splitting the layers into crates would buy one thing: a compiler-enforced
guarantee that the codec does no I/O. Two checks buy the same guarantee more
explicitly:

- `tools/check-no-io.sh` fails the build if anything under `src/codec/` imports
  `std::net`, `std::fs`, `std::thread`, `tokio` or `socket2`, or writes an
  `async fn` or an `.await`.
- `cargo check --no-default-features` is a CI job. With the transport feature
  off, the codec has to keep compiling on its own — no socket, no async runtime.

`src/codec/` interprets the device files; it contains no per-SKU logic and never
will. Adding a device is adding YAML — see
[`../devices/README.md`](../devices/README.md).

## Mode dispatch, and the `Transport` trait

A mode is one implementation of `Transport`: given a device and the bytes the
codec built, send them and report what came back. `src/transport/` holds what
every mode shares — the trait, the transport-neutral error, the circuit breaker,
device status and the event types. `src/lan/` and `src/ble/` implement it, and
each keeps the inherent surface that the trait cannot express.

The facade holds one transport per mode and looks the mode up rather than
matching on it. `Govee::attach` takes the transports a host built and refuses
two that claim the same mode. A build that carries no transport for an enabled
mode reports the mode as unavailable, and `cloud` adds a module rather than a
match arm in every call site.

The trait does not make modes implicit. Which transports a device may use stays
the user's explicit list; the trait only removes the repetition. See
[`modes.md`](modes.md).

## Bindings

- **Node** — `napi-rs`. Serves the playground, the Electron app and the
  Homebridge plugin.
- **Python** — `PyO3` / `maturin`. Serves the Home Assistant component. It needs
  wheels for aarch64, armv7 and musl, or Raspberry Pi users cannot install it;
  that CI is part of the work, not an afterthought.

## The catalog as an artifact

`devices/*.yaml` is the source of truth, one file per SKU. `cargo run -p xtask`
generates the whole directory into a single `catalog.json`, carrying the schema
revision it was generated at; CI builds it on every run and a release attaches
it. A third-party tool reads that one file instead of walking a directory and
parsing YAML. It is a build output, never committed.

The crate still **compiles the catalog in**, at build time. That is what keeps
the SDK a single artifact: no data file to install alongside it, no path to
configure, nothing to go missing on a Raspberry Pi. Loading an external
catalog at runtime is deliberately deferred — it is a way to ship a device fix
without a release, and it is also a way for a file nobody reviewed to decide
what bytes reach your hardware ([`security.md`](security.md)).

`schema_version` is validated on the way in: an unknown version is a typed
error, not a file read as if it were v1.

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

Adding a SKU still means a release of every package, because the catalog is
compiled in: a device file merged today reaches users when Rust, Python and
Node have each shipped. That is the known friction, and the generated
`catalog.json` is the first step away from it rather than the fix.
