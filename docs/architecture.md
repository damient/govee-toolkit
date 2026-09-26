# Architecture

One core, in Rust. Every other language reaches it through a binding rather than
re-implementing it.

## Why one core

The protocol is small and stable: UDP, a JSON envelope, and one
variable-length raw frame with an XOR checksum
([`protocol/lan.md`](protocol/lan.md)). What changes is the data — new SKUs,
new opcodes, new per-family capabilities. So the part that would be duplicated
barely moves, and the part that moves constantly is YAML anyone can write.
Three ports of one protocol means three places a frame can be built wrong.

Rust for what the fast path needs: no garbage collector pausing a segment
stream, no data races on a socket per device, and a single static binary.

## Layers

```
devices/*.yaml             data — what a SKU does, and the bytes for it
       │
src/codec/                 codec — no I/O, no SKU names, no command names
       │                   src/profile/ derives the DMX channel table from the
       │                   same data, and does no I/O either
       │
src/transport/             shared by every mode — the Transport trait, device
       │                   identity, errors, the breaker
       │
src/lan/                   transport — UDP, discovery, device cache
src/ble/                   transport — GATT, scan, one link, paced writes
src/cloud/                 transport — HTTPS, the account's device list,
       │                   one request per device per interval
       │
src/stream/                segment channel — armed once, fed frames on a clock
       │
src/exit.rs                what a binary writes to the process streams, and
       │                   the code it exits with
       │
src/ (crate root)          facade — modes, configuration, events
       │
  ┌────┴────┐
node       python
(napi-rs)  (PyO3)
```

**One crate, `govee-toolkit`**, at `packages/rust`. The layers are modules of
it, and what is optional is a cargo feature rather than a separate package:
`lan` is on by default, and `ble` and `cloud` are opt-in.
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

- `tools/check-no-io.sh` fails the build if anything under `src/codec/` or
  `src/profile/` imports `std::net`, `std::fs`, `std::thread`, `tokio` or
  `socket2`, or writes an `async fn` or an `.await`.
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
mode reports the mode as unavailable. Each mode is a module, and none of them
adds a match arm to a call site.

The trait does not make modes implicit. Which transports a device may use stays
the user's explicit list; the trait only removes the repetition. See
[`modes.md`](modes.md).

Discovery is two questions, so the trait asks two. `Transport::scan` enumerates
what is there and spends the whole window. `Transport::scan_for` looks for one
device and returns at its answer. The facade builds one precondition on the
second — `Govee::ensure_known` — so the CLI and every binding get it from the
crate rather than write it again.

A sequence over several devices is the facade's work too, and not a binary's.
`Govee::identify_walk` takes the rig off, lights one device at a time and
reports what failed; the caller passes an observer for the lines it prints.
The CLI and the bridge both call it, and neither one carries the walk.
`Govee::identify` reads the targets and runs the same walk. The CLI and the
bindings call it, so each one carries one call and not the sequence.

What a binary reports is the crate's work too. `exit::Failure` carries the name
a script reads, the line a person reads and the exit code, and `exit::Writer`
chooses between the record and the line. `govee` and `govee-dmx` both report
through it, so one exit code and one error record mean the same thing in both.
`src/exit.rs` is the one module of the crate that writes to the process
streams; everything else writes through `tracing`.

`ble` adds a second seam under that trait, `ble::wire`: an adapter and a
peripheral. Above it everything is protocol; below it is one platform's radio.
`Transport::with_adapter` takes another, which is how `crates/sim` runs the
whole send path with no Bluetooth on the machine.

## Bindings

A binding reads Python or JavaScript values, calls the crate and hands back
what the crate answered. Nothing else lives there: no command name, no mode
name, no default value, no record shape and no error class name. Where a
surface must print an answer, the crate builds it — `Event::to_json` for an
event, `describe` for a device file, `summary::Summary` for the line a binding
prints beside a value — so the CLI, the bindings and an application read one
shape. `Category::class_name` names the error class a binding raises, so one
failure carries one name in every language. Where a surface must read a value,
the crate reads it: `DeviceHandle::args` takes what a caller supplied and reads
it under the types the device file declares, and `codec::coerce::triple` reads
one color from three channels. A surface that reads values and then sends
holds one `DeviceHandle::resolve`, so the mode, the SKU and the device file are
read once and the bytes go over the mode the values were read for.

- **Node** — `napi-rs`, in `packages/node`: the facade as a promise-based API,
  in one prebuilt addon per platform. The runners build Linux and Windows on
  `x86_64` and `aarch64`, and macOS on `aarch64`. A thin TypeScript layer over
  the addon carries the asynchronous iterators and the disposer, which an addon
  cannot declare. It serves the playground, the Electron app and the Homebridge
  plugin.
- **Python** — `PyO3` / `maturin`, in `packages/python`: the facade as an
  `asyncio` API, in `abi3` wheels. The runners build Linux, macOS and Windows on
  `x86_64` and `aarch64`; containers cross-build the Linux `armv7`, musl
  `x86_64`, musl `aarch64` and musl `armv7` wheels. It serves the Home Assistant
  component, which runs on the last four.

## The catalog as an artifact

`devices/*.yaml` is the source of truth, one file per SKU. `cargo run -p xtask`
generates the whole directory into a single `catalog.json`, carrying the schema
revision it was generated at; CI builds it on every run and a release attaches
it. A third-party tool reads that one file instead of walking a directory and
parsing YAML. It is a build output, never committed.

`xtask` loads the device files through the crate and serializes the entries it
holds, so the catalog says what the SDK reads. It is flat: an `include:` and an
`overrides:` block are applied on load, so a reader of the catalog resolves
nothing.

The crate **compiles the catalog in**, at build time, which keeps the SDK a
single artifact: no data file to install, no path to configure. Loading
an external catalog at runtime is deferred — it ships a device fix without a
release, and it also lets a file nobody reviewed decide what bytes reach your
hardware ([`security.md`](security.md)).

`schema_version` is validated on the way in: an unknown version is a typed
error, not a file read as if it were v1.

## Roles: how the SDK acts on its own

Some operations the SDK starts by itself: it verifies a write, it opens a
segment stream, it puts a device on a network. Each one needs a command, and
no command name may live in code. A device file therefore marks the entry with
a `role:`, and the entry's arguments with an argument `role:`. The SDK looks
the entry up by role and fills what it must — `devices/schema.yaml` lists them.

That is what keeps a multi-step operation generic. Wi-Fi provisioning is four
writes in a fixed order, and the order does not change between devices even
where the frames do. `src/provision.rs` holds the order and the delay;
`devices/*.yaml` holds every byte. A new SKU tags its entries and adds no code,
and a family whose frames differ changes its file alone.

Where several SKUs speak one dialect to the byte, the table lives once under
`devices/families/` and each file names it in `include:`. The loader merges it
in, so the rest of the system sees one flat device. A fragment carries layout
only: what a unit answered stays in that unit's file, under `verified:`. A
clash between a fragment and the file that includes it is an error, never a
silent override.

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
