# Versioning and compatibility

Three packages release independently off one shared core. This page is the rule
they follow, written before the first release rather than after the second one
breaks.

## Semver, and what pre-1.0 means

Every package follows [semantic versioning](https://semver.org/). All of them
are pre-1.0 today, and pre-1.0 has one specific meaning here:

- `0.x.y` → `0.(x+1).0` may break the public API. It is the breaking bump.
- `0.x.y` → `0.x.(y+1)` does not break it.

A breaking change still gets a changelog entry saying what broke and what to do
about it. Pre-1.0 is permission to move, not permission to move quietly.

1.0 arrives when the mode model, the `Transport` trait and the device schema
have survived `ble` and `cloud` landing — not on a date.

## Tags and packages

| Tag | Package | Registry |
| --- | ------- | -------- |
| `rust-vX.Y.Z` | `govee-toolkit` | crates.io |
| `python-vX.Y.Z` | `govee-toolkit` | PyPI |
| `node-vX.Y.Z` | `govee-toolkit` | npm |

Versions are **not** kept in lockstep. Three packages that move at different
speeds and share a version number would mean publishing two no-op releases every
time one of them changed.

## One crate, and what a feature means

The Rust side is a single published crate, `govee-toolkit`. The codec, the
transport and the facade are modules of it; `crates/sim` and `crates/xtask` are
never published.

A transport is a **cargo feature**, and a feature is public API. `lan` is on by
default. Adding a feature (`ble`, `cloud`) is a minor change. Removing one, or
moving an item out of the default set, is breaking — a user who builds with
`--no-default-features` is relying on exactly what is left, and the codec-only
build is a supported configuration, checked in CI.

`govee_toolkit::codec` is public because a binding needs it. It carries the same
promise as the rest of the crate.

## Which binding works with which core

Node, Python and the facade are built from the same workspace commit: a binding
release embeds the core it was built against, so there is no version pair to
match. The core version a binding was built from is recorded in its metadata and
reported at runtime.

## What counts as public API

Public:

- The facade's exported types, functions and traits.
- The configuration file format at `$XDG_CONFIG_HOME/govee-toolkit/config.yaml`.
- The event types an application subscribes to, and the error variants it
  matches on.
- The conformance vector format in `tests/fixtures/golden/`.
- The `devices/` schema and the generated `catalog.json` format.

Not public: the APIs of `crates/sim` and `crates/xtask`, the on-disk device
cache format (it is versioned and discarded when foreign), log and tracing
output, and anything marked `#[doc(hidden)]`.

### The device catalogue

The catalogue is data, and it is versioned as data rather than as code:

- `devices/*.yaml` declares `schema_version`. An unknown value is a **typed
  error** — a file is never read as if it were the version the build happens to
  understand.
- The generated [`catalog.json`](architecture.md#the-catalogue-as-an-artefact)
  carries its own version, independent of any package version.
- **Adding a SKU is additive** and never a breaking change. It ships in a patch
  release.
- **Changing what an existing command means is breaking** — a renamed command, a
  changed argument range, a payload that produces different bytes for the same
  arguments. It is a breaking bump, and it changes the conformance vector, which
  is how a port finds out.
- Correcting a command that was measurably wrong is a bug fix, not a break. The
  changelog says which device files changed and why.

## MSRV

The minimum supported Rust version is declared in `packages/rust/Cargo.toml` and
checked in CI.

- It is **raised in the same commit as the feature that needs it**, never after
  the fact, and the commit says which feature.
- **A raise is a minor bump** — `0.x` → `0.(x+1)` pre-1.0, and a minor release
  after 1.0. It is not a patch: someone pinned to an older toolchain finds out
  from the version number, not from a failed build.
- MSRV is not chased upward for its own sake.

## Deprecation

- A public item that is going away is marked `#[deprecated]` with the
  replacement named in the message, and stays for **at least one minor release**
  before it is removed.
- Removal happens in a breaking bump, and only for something that has been
  deprecated in a released version — never deprecated and removed in the same
  one.
- A configuration key follows the same path: it keeps working, and warns once at
  startup, for at least one minor release. It is never silently ignored — an
  ignored key reads as a setting that did not work.
- A device file command is deprecated in the device file, not in code.

## CI

`cargo-semver-checks` becomes a **required job at the first release**: before
anything is published there is no baseline to compare against, and after it
there is no excuse for an unintended break. Until then, the checks in
[`../CONTRIBUTING.md`](../CONTRIBUTING.md) are what CI runs.

Nothing has shipped yet — the manifests declare the package names, and no
release exists on any registry.
