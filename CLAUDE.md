# CLAUDE.md

Working notes for Claude Code on this repository. Conventions here are decisions
already made — follow them rather than re-deriving them.

## What this project is

An unofficial, multi-language SDK to control Govee devices locally, built around
**undocumented LAN commands** found through reverse engineering. `lan`, `ble`
and `cloud` are three **modes** the user enables per device — never a fallback
chain, never implicit.

The protocol is implemented **once**, in Rust (`packages/rust`). Node and Python
bind to that core. Read
[`docs/architecture.md`](docs/architecture.md) before adding code to any
language, and [`docs/modes.md`](docs/modes.md) before touching anything that
chooses a transport.

## Non-negotiables

1. **The `lan` fast path wins.** Latency and reliability there come before any
   other mode, any integration, any convenience. A change that adds work to the
   send path needs a reason.
2. **Modes are explicit.** One enabled mode means one mode: if the device is
   unreachable, the command fails and says so. Never silently substitute a mode,
   never approximate a command a mode cannot serve — fail explicitly instead.
3. **`devices/*.yaml` is the single source of truth.** SDKs read it. They
   implement transports and generic parsing, never per-SKU protocol logic. No
   SKU name and no command name belongs in Rust code — if you are about to write
   one, the device file is missing something instead.
4. **Never invent verification.** Do not fill `verified`, a capability, a
   measurement or a compatibility row from inference. Unverified is `?` or a
   `TODO`, and that is a perfectly good answer.

## Where things go

| Content | Location |
| ------- | -------- |
| How the protocol works, for any device | `docs/protocol/*.md` |
| What a specific model does, and numbers measured on it | `devices/<SKU>.yaml` |
| Human-readable "does my device work" | `docs/compatibility.md` |
| Govee's own list of LAN-capable models | `docs/lan-supported-devices.md` |
| Full feature list | `docs/features.md` |
| Ordering of the work | `docs/roadmap.md` |
| Why the code is shaped this way | `docs/architecture.md` |
| Arguments in, exact bytes out | `tests/fixtures/golden/<mode>/<SKU>.json` |
| What a package's release changed | `packages/<pkg>/CHANGELOG.md` |
| Catalogue changes, and the release history | `CHANGELOG.md` |

The split between the first two rows matters and is easy to get wrong:
`docs/protocol/` describes the protocol generically — **no SKU names, no
firmware version numbers, no numbers measured on one unit**. Anything measured
belongs in the device file, alongside the physical length it was measured on,
since segment count and native resolution depend on the unit and not only on the
SKU.

## Writing

- **Plain and concise.** State the fact and move on. No lyrical framing, no
  selling, no filler adjectives. This applies to docs, commit messages and code
  alike.
- **A comment earns its place or it goes.** It survives by carrying what the
  code cannot state: a constraint the compiler does not enforce, a measured
  value and what it was measured on, a trap, or a pointer that saves a search.
  A doc comment adds what a caller must know — errors, units, what the item
  does not do. Everything else is deleted, not reworded: restatement, headings
  over code that names itself, narration of control flow, a private `///` that
  expands the item's name into a sentence, commented-out code. `missing_docs`
  is the one exception: a public item must carry a `///`, so give it payload —
  errors, units, what it does not do — or one short line, never a paragraph.
- **A kept comment states its fact and stops.** No preamble, no second sentence
  repeating the first, no re-describing the mechanism the reader is looking at.
- **English throughout**, including code comments.
- **Describe the code as it is, not as it was.** Docs and comments carry no
  trace of refactored, renamed or deleted code: no "no longer", "used to",
  "previously", "this replaces the old X". Rewrite in the present. History that
  is load-bearing stays — a check that exists to stop a named regression, a
  migration note the user must act on — and `docs/roadmap.md`, release notes and
  commit messages are history by design.
- **The README lists what works today.** Planned work lives in
  `docs/features.md` and `docs/roadmap.md`, marked ✅ / 🚧 / 🔜.
- **Avoid discouraging phrasing where it carries no technical information.**
  "Untested rather than unsupported" over "nothing works yet". Keep the blunt
  version when it is a real technical caveat: silent clamping, cloud-only
  features, and explicit failures must stay unambiguous.
- Roadmap ordering is a starting point that follows what people ask for — do not
  present it as a commitment.
- Do not claim affiliation with Govee anywhere; the trademark is used
  descriptively.

## Device files

- `aliases` holds SKUs **verified** to behave identically. A SKU that merely
  looks like the same product goes in `candidate_aliases` — different lengths of
  one product are not interchangeable.
- `modes:` declares what the hardware supports. What the user enables is runtime
  configuration and never lives here. `none` says somebody established the
  hardware cannot do it; a mode nobody probed stays `unknown`, which is the
  default.
- Undocumented commands get `documented: false`, plus a `notes:` line and a
  pointer to the matching section of `docs/protocol/lan.md`. This is enforced by
  `cargo test`.
- Attach a real capture under `tests/fixtures/lan-captures/<SKU>/` and reference
  it from `capture:`.
- `payload:` and `frame:` are **executable**, not descriptive — the core builds
  bytes straight from them. The two mini-languages are documented at the top of
  `devices/schema.yaml`.
- Add a conformance vector under `tests/fixtures/golden/` for every new command,
  and say in its `source` whether the bytes come from a capture or were worked
  out from the documented layout. Only the first is evidence. `cargo test` fails
  on a command that has none.
- Redact a capture before committing it — the checklist is in
  `tests/fixtures/README.md` and `tools/check-captures.sh` re-checks what it can.
  Git keeps a leaked capture after the fix.
- `docs/compatibility.md` holds two generated tables. After a device file
  changes, run `cargo run -p xtask -- compat`; CI fails on drift.

## Protocol work

- A failed probe is indistinguishable from an unimplemented feature — the
  firmware answers nothing either way. Assume a malformed request before
  concluding a device lacks a capability.
- Firmware updates change behavior without notice. Ship probes rather than
  trusting a table.
- Two techniques that pay off, in order: decompile the vendor's desktop app, and
  capture its UDP traffic on port 4003. Both beat guessing frames. What you
  learn that way is describable in your own words; **no decompiled output,
  extracted string, resource or firmware image is ever committed** — see
  `CONTRIBUTING.md`, "Legal and provenance".

## Packages

`packages/rust` is the reference implementation and the only place protocol
logic exists. It is **one crate**, `govee-toolkit`, with the layers as modules:
`src/codec/` (no I/O), `src/lan/` and `src/stream/` (behind the default `lan`
feature) and the facade at the crate root. `crates/sim` and `crates/xtask` sit beside it and
carry `publish = false`. A transport is a cargo feature — `ble` and `cloud` join
`lan` as they land.

The codec keeps building on its own (`cargo check --no-default-features`), and
`tools/check-no-io.sh` fails the build if anything under `src/codec/` imports
`std::net`, `std::fs`, `std::thread`, `tokio` or `socket2`, or goes async. That
check is what keeps the codec I/O-free in a single crate; do not weaken it.

Node and Python wrap the crate (napi-rs, PyO3). Each package versions and
releases independently (`rust-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`) through
the workflows in `.github/workflows/`. The policy is `docs/versioning.md`.

`govee-toolkit` `0.2.0` is on crates.io. The name is taken on PyPI and npm too,
by a `0.0.0` placeholder each: those two packages have no code yet. The bare
name `govee` on crates.io belongs to an unrelated project.

In Rust: no `unsafe`, and no `panic` / `unwrap` / `expect` in library code. Out
of range is an error, never a clamp — the firmware clamps in silence, and an SDK
that did the same would report success for a value the device did not apply.

Mode dispatch is a `match` in the facade. A `Transport` trait is the
prerequisite for the BLE pull request, not something to add early — see
`docs/architecture.md`.

Format with `cargo +nightly fmt` — `rustfmt.toml` uses nightly-only options and
stable rustfmt produces a different result. A Rust source file stays under 400
lines (`tools/check-file-length.sh`); split along responsibilities rather than
trimming to fit. The MSRV is checked in CI, so a feature that needs a newer
compiler raises `rust-version` in the same commit.

`tools/qa.sh` runs the CI checks locally — use it before pushing rather than
reading the result off a pull request. `ci.yml` runs on push to `main` and on
pull request, skipping every job while the pull request is a draft, and tests on
Linux, macOS and Windows because the multicast socket differs on each. Every
third-party action is pinned to a commit SHA; keep it that way. The release
workflows publish through trusted publishing — the registry trades the job's
OIDC identity for a short-lived token, so no registry token is stored in the
repository. A version already on the registry is skipped rather than failing
the run, since a tag moved to a new commit reruns the job.

## Repository

- MIT, no copyleft dependencies.
- Commit subjects are [Conventional Commits](https://www.conventionalcommits.org/)
  — `<type>(<scope>)!: <summary>`, types `feat`, `fix`, `perf`, `refactor`,
  `docs`, `test`, `build`, `ci`, `chore`, `revert`. The type decides the semver
  bump at release, so `feat` and `fix` are not interchangeable. The body carries
  the reasoning.
- Every commit carries `Signed-off-by` (`git commit -s`); CI enforces it.
- A change to `packages/*/src/` or `devices/*.yaml` carries a changelog entry —
  `/changelog` writes it. CI enforces that too.
- Releasing is `docs/versioning.md`: the changelog section is the source, a
  signed `<pkg>-vX.Y.Z` tag starts the workflow, and `tools/release-notes.sh`
  fails the run when the tag, the manifest and the changelog heading disagree.
- Commit and push only when asked.
