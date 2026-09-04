# Rust core

The reference implementation. Protocol logic lives here once, and every other
language reaches it through a binding rather than a port — see
[`../../docs/architecture.md`](../../docs/architecture.md).

Nothing is published to crates.io yet.

## Crates

| Crate | Contents | Status |
| ----- | -------- | ------ |
| [`govee-core`](crates/govee-core) | Device catalog, command encoding, raw frame codec. No I/O. | ✅ |
| `govee-lan` | UDP transport: discovery, reused socket, per-device circuit breaker, segment streaming | 🔜 |
| `govee` | Public facade: modes, configuration, events | 🔜 |
| `govee-cli` | Dogfooding binary — drive a SKU without an SDK | 🔜 |

The split is deliberate: `govee-core` does no I/O, so every protocol decision is
testable without hardware and without a network.

## Working on it

```bash
cargo test                                    # unit, conformance and doc tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check                              # licenses and advisories
```

`rust-toolchain.toml` pins the channel; `rustup` installs it on first build.

## The device catalog is compiled in

`build.rs` embeds every `devices/*.yaml` into the binary, so an SDK ships as one
artifact with no data directory to install. This is deliberate, and the
trade-off is accepted: **a new SKU arrives with a release**, not with a file
someone dropped on their disk, so what one person measured on one unit does not
quietly become what everyone else's device is assumed to do.

`GOVEE_DEVICES_DIR` overrides where the files are read from at build time, which
is also how the crate will be built from a vendored copy once it is published.

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
above this crate. And it is **visible**: `overlay` returns what it replaced, so
an override can never be silent.

## Conventions

- No `unsafe`, and no `panic` / `unwrap` / `expect` in library code. A mode that
  cannot serve a command returns a typed error — the SDK never approximates and
  never aborts its host.
- Out-of-range arguments are rejected, not clamped. The firmware clamps in
  silence; reporting success for a value the device did not apply would make the
  SDK lie.
- No SKU and no command name appears in Rust code. Everything specific to a
  device is in `devices/*.yaml`.
