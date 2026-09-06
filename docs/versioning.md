# Versioning and compatibility

Three packages release independently off one shared core. This page is the rule
they follow, written before the first release rather than after the second one
breaks.

## Where the changelogs are

Each package keeps its own —
[`rust`](../packages/rust/CHANGELOG.md),
[`python`](../packages/python/CHANGELOG.md),
[`node`](../packages/node/CHANGELOG.md). The root
[`CHANGELOG.md`](../CHANGELOG.md) carries what belongs to no package — the
device catalog, the documentation, the tooling and CI — and indexes the three.

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

### The device catalog

The catalog is data, and it is versioned as data rather than as code:

- `devices/*.yaml` declares `schema_version`. An unknown value is a **typed
  error** — a file is never read as if it were the version the build happens to
  understand.
- The generated [`catalog.json`](architecture.md#the-catalog-as-an-artifact)
  carries its own version, independent of any package version.
- **Adding a SKU is additive** and never a breaking change. It ships in a patch
  release.
- **Changing what an existing command means is breaking** — a renamed command, a
  changed argument range, a payload that produces different bytes for the same
  arguments. It is a breaking bump, and it changes the conformance vector, which
  is how a port finds out.
- Correcting a command that was measurably wrong is a bug fix, not a break. The
  changelog says which device files changed and why.

## Releasing

The changelog is the source, and the release is derived from it:

1. The pull request puts the `## [X.Y.Z] — YYYY-MM-DD` heading above the
   entries that have accumulated at the top of the changelog, and bumps the
   version in the package manifest. Both are reviewed there.
2. A signed tag names the package and the version — `git tag -s rust-v0.3.0 -m
   rust-v0.3.0 && git push --tags`.
3. The tag starts the package's release workflow. Its first step runs
   `tools/release-notes.sh <pkg> <tag>`, which compares the tag, the manifest
   version and the changelog heading, and prints that section. Three numbers
   that disagree fail the run before anything is built.
4. The section becomes the body of the GitHub release, and the workflow
   publishes to the registry.

No registry token is stored in the repository: each workflow publishes through
the registry's trusted publishing, which trades the job's OIDC identity for a
short-lived token. It is configured once per package, on the registry, against
this repository and the workflow file name — crates.io under the crate's
settings, PyPI and npm under the project's. A publish from a workflow the
registry does not know about is refused.

A version already on the registry is skipped rather than failing the run: a tag
moved to a new commit reruns the whole job, and a registry never takes the same
version twice.

The release history is the repository's releases page, written by that
workflow. The root [`../CHANGELOG.md`](../CHANGELOG.md) carries the version each
manifest declares, and the catalog's own changelog — dated rather than
numbered, since a package embeds the catalog at build time and the date is
what a release pins.

Which number to bump comes from the commit types in the range: a `feat` is a
minor bump, a `fix` a patch, and a `!` or a `BREAKING CHANGE:` trailer the
breaking one — pre-1.0, that is the minor bump above. `CONTRIBUTING.md` has the
convention.

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

`cargo-semver-checks` runs on every pull request, against the crate on
crates.io as the baseline: a change to the public API that the version in the
manifest does not account for fails there, not on a registry that never takes a
version back. The rest of what CI runs is in
[`../CONTRIBUTING.md`](../CONTRIBUTING.md).

PyPI and npm hold the name under a `0.0.0` placeholder each; the first real
release of either is still ahead.
