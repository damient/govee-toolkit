# CLAUDE.md

Working notes for Claude Code on this repository. Conventions here are decisions
already made — follow them rather than re-deriving them.

## What this project is

An unofficial, multi-language SDK to control Govee devices locally, built around
**undocumented LAN commands** found through reverse engineering. `lan`, `ble`
and `cloud` are three **modes** the user enables per device — never a fallback
chain, never implicit.

Read [`docs/modes.md`](docs/modes.md) before touching anything that chooses a
transport.

## Non-negotiables

1. **The `lan` fast path wins.** Latency and reliability there come before any
   other mode, any integration, any convenience. A change that adds work to the
   send path needs a reason.
2. **Modes are explicit.** One enabled mode means one mode: if the device is
   unreachable, the command fails and says so. Never silently substitute a mode,
   never approximate a command a mode cannot serve — fail explicitly instead.
3. **`devices/*.yaml` is the single source of truth.** SDKs read it. They
   implement transports and generic parsing, never per-SKU protocol logic.
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
  pointer to the matching section of `docs/protocol/lan.md`.
- Attach a real capture under `tests/fixtures/lan-captures/<SKU>/` and reference
  it from `capture:`.

## Protocol work

- A failed probe is indistinguishable from an unimplemented feature — the
  firmware answers nothing either way. Assume a malformed request before
  concluding a device lacks a capability.
- Firmware updates change behavior without notice. Ship probes rather than
  trusting a table.
- Two techniques that pay off, in order: decompile the vendor's desktop app, and
  capture its UDP traffic on port 4003. Both beat guessing frames.

## Packages

Python and Node are the reference implementations; PHP is ported from them once
they are stable. Each package versions and releases independently
(`python-vX.Y.Z`, `node-vX.Y.Z`, `php-vX.Y.Z`) through the workflows in
`.github/workflows/`.

Nothing is published yet — package names are reserved in the manifests but no
release has shipped.

GitHub Actions is disabled on the repository and the workflows only accept
`workflow_dispatch`, because empty runs email a failure on every push. Re-enable
both together with the first real test step.

## Repository

- MIT, no copyleft dependencies.
- Commit and push only when asked.
