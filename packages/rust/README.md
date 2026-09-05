# govee-toolkit

Control Govee devices over the LAN, including undocumented commands found
through reverse engineering. Unofficial, and not affiliated with Govee.

This is the reference implementation. Protocol logic lives here once, and every
other language reaches it through a binding rather than a port — see
[`../../docs/architecture.md`](../../docs/architecture.md).

Nothing is published to crates.io yet.

## One crate, three layers

| Layer | Where | Contents |
| ----- | ----- | -------- |
| Codec | [`src/codec/`](src/codec) | Device catalog, command encoding, raw frame codec. No I/O. |
| Transport | [`src/lan/`](src/lan) | UDP: discovery, device cache, reused socket, per-device circuit breaker |
| Facade | [`src/`](src) | Configuration, mode selection, events |

Two more live beside it and are never published:
[`crates/sim`](crates/sim), a fake device on UDP with fault injection, and
[`crates/xtask`](crates/xtask), which generates the distributable catalog.

The layering is deliberate even though it is one crate. The codec does no I/O,
so every protocol decision is testable without hardware and without a network —
`tools/check-no-io.sh` fails the build if anything under `src/codec/` imports a
socket, a runtime or the filesystem. The transport carries bytes for one mode
and never chooses between modes: with a single mode enabled and the device
unreachable, it returns an error and nothing else happens. Choosing is the
facade's job, and it chooses from breaker state already recorded, never by
trying a mode and waiting for a timeout.

## Features

| Feature | Default | What it adds |
| ------- | ------- | ------------ |
| `lan` | yes | The UDP transport, and the facade above it |

With default features off, what is left is the codec on its own: arguments in,
bytes out, no socket and no async runtime.

```bash
cargo check --no-default-features   # the codec alone
```

`ble` and `cloud` are declared modes with no transport yet. Enabling one is
reported as such — never silently skipped, never substituted.

## Testing without hardware

`govee-toolkit-sim` is a fake device on the loopback with ephemeral ports. It
answers discovery and status requests, records everything else, and can be told
to go silent, to answer late or to drop replies — which is how the breaker's
transitions are exercised end to end in CI.

```bash
cargo run -p govee-toolkit-sim -- --sku H61A0 --ip 192.0.2.10   # on the real ports
```

It plays the wire, not the firmware: it does not interpret writes, because
modelling what each command means would put per-SKU semantics in Rust, which is
the one thing this project keeps in `devices/*.yaml`.

## Working on it

```bash
../../tools/qa.sh                             # everything CI runs
```

Individually:

```bash
cargo test --workspace --all-features         # unit, conformance and doc tests
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --no-default-features             # the codec still builds alone
cargo +nightly fmt --all --check              # nightly: rustfmt.toml needs it
cargo deny check                              # licenses and advisories
```

`rust-toolchain.toml` pins the channel; `rustup` installs it on first build.
`rustfmt.toml` uses options only nightly implements, so stable `cargo fmt` will
disagree with CI.

## The device catalog is compiled in

`build.rs` embeds every `devices/*.yaml` into the binary, so an SDK ships as one
artifact with no data directory to install. This is deliberate, and the
trade-off is accepted: **a new SKU arrives with a release**, not with a file
someone dropped on their disk, so what one person measured on one unit does not
quietly become what everyone else's device is assumed to do.

A file declaring a `schema_version` this build does not implement is refused
outright — reading it under the rules of another revision is the silent kind of
wrong this project does not do.

`GOVEE_DEVICES_DIR` overrides where the files are read from at build time. It is
also how the crate is built once published: `cargo package` cannot reach above
the manifest, so the release vendors `devices/` in beside it first.

### The distributable catalog

```bash
cargo run -p xtask            # writes dist/catalog.json
```

One generated JSON file holding every device, for anything that wants the
catalog without a YAML parser. It is a build
output: never committed, produced by CI, attached to a release.

### The local escape hatch

`Catalog::overlay()` replaces catalog entries with locally supplied files —
someone probing a SKU that has not shipped yet, or correcting one that is wrong
on their unit:

```rust
let mut catalog = Catalog::embedded()?;
for replaced in catalog.overlay(local_files)? {
    tracing::warn!(%replaced.sku, was = %replaced.was, now = %replaced.now, "device file overridden");
}
```

Two properties matter and are tested. It is **opt-in**: nothing reads a local
directory on its own, and the layer that decides where such a directory lives is
above the codec. And it is **visible**: `overlay` returns what it replaced, so an
override can never be silent.

## Conventions

- No `unsafe`, and no `panic` / `unwrap` / `expect` in library code. A mode that
  cannot serve a command returns a typed error — the SDK never approximates and
  never aborts its host.
- Out-of-range arguments are rejected, not clamped. The firmware clamps in
  silence; reporting success for a value the device did not apply would make the
  SDK lie.
- No SKU and no command name appears in Rust code. Everything specific to a
  device is in `devices/*.yaml`.
- Every command in the catalog has a conformance vector under
  `tests/fixtures/golden/`; `cargo test` fails if one does not.
