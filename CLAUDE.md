# CLAUDE.md

Working notes for Claude Code on this repository. Conventions here are decisions
already made — follow them rather than re-deriving them.

## What this project is

An unofficial, multi-language SDK to control Govee devices locally, built around
**undocumented LAN commands** found through reverse engineering. `lan`, `ble`
and `cloud` are three **modes** the user enables per device — never a fallback
chain, never implicit.

The protocol is implemented **once**, in Rust (`packages/rust`). Node and Python
bind to that core; PHP is the one hand-written port. Read
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
- **Comment only when the comment earns its place** — a non-obvious constraint, a
  measured value, a trap. Do not restate what the code already says.
- **English throughout**, including code comments.
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
  configuration and never lives here.
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
  out from the documented layout. Only the first is evidence.

## Protocol work

- A failed probe is indistinguishable from an unimplemented feature — the
  firmware answers nothing either way. Assume a malformed request before
  concluding a device lacks a capability.
- Firmware updates change behavior without notice. Ship probes rather than
  trusting a table.
- Two techniques that pay off, in order: decompile the vendor's desktop app, and
  capture its UDP traffic on port 4003. Both beat guessing frames.

## Packages

`packages/rust` is the reference implementation and the only place protocol
logic exists. Node and Python wrap it (napi-rs, PyO3); PHP is ported by hand and
is checked against `tests/fixtures/golden/`. Each package versions and releases
independently (`rust-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`, `php-vX.Y.Z`)
through the workflows in `.github/workflows/`.

Nothing is published yet — package names are reserved in the manifests but no
release has shipped.

In Rust: no `unsafe`, and no `panic` / `unwrap` / `expect` in library code. Out
of range is an error, never a clamp — the firmware clamps in silence, and an SDK
that did the same would report success for a value the device did not apply.

`ci.yml` runs on push and pull request. The release workflows stay
`workflow_dispatch` only until there is something to publish.

## Repository

- MIT, no copyleft dependencies.
- Commit and push only when asked.
