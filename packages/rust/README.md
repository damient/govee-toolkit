# govee-toolkit

Control Govee devices over the LAN from Rust, including undocumented commands
found through reverse engineering. Unofficial, and not affiliated with Govee.

This is the reference implementation. Protocol logic lives here once, and every
other language reaches it through a binding rather than a port — see
[`docs/architecture.md`][architecture].

## Install

```bash
cargo add govee-toolkit
```

Async, on Tokio. The `lan` feature is on by default and brings the UDP
transport with it; turn it off and what is left is the codec alone — arguments
in, bytes out, no socket and no runtime.

## Quick start

Discover what is on the network, then turn something on:

```rust
use govee_toolkit::{Args, Config, Govee};

let govee = Govee::start(Config::load()?).await?;

let devices = govee.scan().await?;
for device in &devices {
    println!("{} — {} — modes {:?}", device.id, device.sku, device.modes);
}

let id = devices[0].id.clone();
let served = govee
    .device(&id)
    .send("power", &Args::new().int("on", 1))
    .await?;

println!("served over {}", served.mode);
```

`Served` carries the mode that ran the command, the device file entry that was
sent and the `cmd` that went on the wire. With several modes enabled, which one
served a command is not something a caller should have to guess.

### The commands you can send

Command names are entries in the device's YAML file, not identifiers in this
crate. For the H61A0:

```rust
govee.device(&id).send("power",      &Args::new().int("on", 0)).await?;
govee.device(&id).send("brightness", &Args::new().int("level", 60)).await?;
govee.device(&id).send("color",      &Args::new().int("r", 255).int("g", 40).int("b", 0)).await?;
govee.device(&id).send("colortemp",  &Args::new().int("kelvin", 4000)).await?;
```

`govee.catalog()` lists what a SKU declares, so a UI can build its controls from
the catalog rather than hardcoding them. A name the device file does not define,
or an argument outside its declared range, is an error before anything reaches
the network.

### Reading state

```rust
let status = govee.device(&id).status().await?;   // round-trips to the device
println!("{:?} at {:?}%", status.on, status.brightness);

let cached = govee.device(&id).last_status();     // no I/O, may be None
```

`status.raw` keeps the device's whole `msg.data`, so undocumented fields stay
reachable without this crate modelling them.

### Segment streaming

The segment channel is armed once and then fed frames. Writes never block: a
source faster than the device replaces its own pending frame rather than backing
up behind it.

```rust
use govee_toolkit::{Rate, StreamOptions, Zones};

let stream = govee
    .device(&id)
    .open_stream(StreamOptions { zones: Zones::App, rate: Rate::Measured, gradient: false })
    .await?;

for step in 0.. {
    let frame: Vec<[u8; 3]> = (0..stream.zones())
        .map(|z| wheel(z + step))
        .collect();
    stream.set_all(&frame)?;
}

stream.close().await?;
```

- `Zones::App` matches what the Govee app exposes. `Zones::Native` addresses
  every LED, and fails when nobody has measured that number on the unit — it
  belongs to the physical strip, not to the SKU. `Zones::Exact(n)` picks a
  count.
- `Rate::Measured` paces from `measurements.frame_rate` in the device file,
  falling back to 10 Hz when it records none. That fallback is a starting point,
  not a finding: measure your unit and record it.
- `frames_sent()` and `frames_superseded()` tell you whether your source is
  outrunning the device.

## Configuration

`Config::load()` reads `~/.config/govee-toolkit/config.yaml`
(`$XDG_CONFIG_HOME` and `GOVEE_CONFIG` override the location). Devices are keyed
by the MAC they report in a discovery reply, so a renewed DHCP lease does not
lose them.

```yaml
defaults:
  modes: [lan]

devices:
  "AA:BB:CC:DD:EE:FF":
    modes: [lan]
    name: "desk strip"
```

`Config::load_from(path)` takes an explicit file, and `govee.problems()` returns
what the configuration got wrong without failing the whole load. The full model
is [`docs/modes.md`][modes].

`ble` and `cloud` are declared modes with no transport yet. Enabling one is
reported as `ModeNotImplemented` — never silently skipped, never substituted.

## What this crate will not do to you

- **It never panics on your behalf.** No `unsafe`, and no `panic` / `unwrap` /
  `expect` in library code. Everything that can fail returns `Error`.
- **It never clamps.** An out-of-range argument is rejected. The firmware clamps
  in silence, and reporting success for a value the device did not apply would
  make the SDK lie about the state of your lights.
- **It never substitutes a mode.** With one mode enabled and the device
  unreachable, you get `NoModeAvailable` — not a slower path taken quietly, and
  not a segment animation approximated with a plain color change.
- **It never overrides a device file silently.** `Catalog::overlay` returns
  everything it replaced.

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

## Inside the crate

| Layer | Where | Contents |
| ----- | ----- | -------- |
| Codec | [`src/codec/`](src/codec) | Device catalog, command encoding, raw frame codec. No I/O. |
| Transport | [`src/lan/`](src/lan) | UDP: discovery, device cache, reused socket, per-device circuit breaker |
| Stream | [`src/stream/`](src/stream) | The segment channel: armed once, fed frames on a clock |
| Facade | [`src/`](src) | Configuration, mode selection, events |

Two more live beside it and are never published:
[`crates/sim`](crates/sim), a fake device on UDP with fault injection, and
[`crates/xtask`](crates/xtask), which generates the distributable catalog.

The layering is deliberate even though it is one crate. The codec does no I/O,
so every protocol decision is testable without hardware and without a network —
`tools/check-no-io.sh` fails the build if anything under `src/codec/` imports a
socket, a runtime or the filesystem. The transport carries bytes for one mode
and never chooses between modes. Choosing is the facade's job, and it chooses
from breaker state already recorded, never by trying a mode and waiting for a
timeout.

```bash
cargo check --no-default-features   # the codec alone, no lan feature
```

### The device catalog is compiled in

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

```bash
cargo run -p xtask            # writes dist/catalog.json
```

One generated JSON file holding every device, for anything that wants the
catalog without a YAML parser. It is a build output: never committed, produced
by CI, attached to a release.

### Running a device file that has not shipped

`Catalog::overlay()` replaces catalog entries with locally supplied files —
someone probing a SKU that has not shipped yet, or correcting one that is wrong
on their unit:

```rust
let mut catalog = Catalog::embedded()?;
for replaced in catalog.overlay(local_files)? {
    tracing::warn!(%replaced.sku, was = %replaced.was, now = %replaced.now, "device file overridden");
}
let govee = Govee::start_with(Config::load()?, catalog).await?;
```

It is **opt-in** — nothing reads a local directory on its own — and **visible**:
`overlay` returns what it replaced, so an override can never go unnoticed.

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

Two conventions to know before opening a pull request: no SKU and no command
name appears in Rust code — everything specific to a device is in
`devices/*.yaml` — and every command in the catalog has a conformance vector
under `tests/fixtures/golden/`, with `cargo test` failing if one does not.

<!-- Absolute: this file is the crate description on crates.io, where a
     relative link out of the package directory is dead. -->
[architecture]: https://github.com/damient/govee-toolkit/blob/main/docs/architecture.md
[modes]: https://github.com/damient/govee-toolkit/blob/main/docs/modes.md
