# Contributing to govee-toolkit

A community project, not affiliated with Govee.

## Guiding principle

**LAN path latency and reliability come first.** A contribution that adds
another mode or a third-party integration must not slow down or complicate the
`lan` fast path.

## Legal and provenance

This project reverse engineers an undocumented protocol **for
interoperability**: so that hardware someone already owns can be driven by
software they choose. That purpose sets the boundary of what may be committed
here, and the boundary is not negotiable.

**What you observe is welcome. What you copy is not.**

- Describe behavior in your own words: what a frame looks like on the wire,
  what a device does when it receives it, which field changed what. That is an
  observation, and observations are the substance of this repository.
- Do **not** commit, quote at length, or attach to an issue or pull request any
  artefact taken out of Govee software: decompiler or disassembler output,
  extracted strings, resources, assets, firmware images, or source recovered by
  any means. Knowledge gained from an application may be written down; the
  contents of that application may not be pasted.
- Captures must come from **hardware you own**, on **your own network**. Do not
  submit traffic captured from someone else's device or network.
- No credential, account token, API key, or personally identifying network
  detail in anything committed. Redact before committing — the checklist is in
  [`tests/fixtures/README.md`](tests/fixtures/README.md#redaction-checklist),
  and `tools/check-captures.sh` enforces the shapes it can. A pull request that
  carries one is **closed, not fixed up in place**: git history keeps what a
  later commit removes, so the branch has to go rather than be amended.

A pull request that crosses these lines is closed on sight, whatever it
contains otherwise. It is not a judgement of the contributor — it is that the
repository cannot hold that content at all.

The "Govee" trademark is used descriptively, to identify compatible devices.
This project is not affiliated with, sponsored by, or endorsed by Govee.

## Sign-off (DCO)

There is no CLA. Contributions stay [MIT](LICENSE), and are certified with the
**Developer Certificate of Origin 1.1**: every commit carries a `Signed-off-by:`
line matching the author.

```bash
git commit -s              # adds the line for you
git commit -s --amend      # adds it to the commit you just made
```

The line is checked in CI. A pull request whose commits are missing it does not
merge; `git rebase --signoff main` fixes a branch.

<details>
<summary>Developer Certificate of Origin 1.1</summary>

```
By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I have the
    right to submit it under the open source license indicated in the file; or

(b) The contribution is based upon previous work that, to the best of my
    knowledge, is covered under an appropriate open source license and I have
    the right under that license to submit that work with modifications,
    whether created in whole or in part by me, under the same open source
    license (unless I am permitted to submit under a different license), as
    indicated in the file; or

(c) The contribution was provided directly to me by some other person who
    certified (a), (b) or (c) and I have not modified it.

(d) I understand and agree that this project and the contribution are public
    and that a record of the contribution (including all personal information
    I submit with it, including my sign-off) is maintained indefinitely and
    may be redistributed consistent with this project or the open source
    license(s) involved.
```

Full text: <https://developercertificate.org/>.

</details>

## Where to ask

- **Issue forms** — bug, new device, undocumented command, docs. Pick the form
  that matches; each one asks for what is actually needed to act on it (SKU,
  firmware version, capture, mode). Prefer a form over a blank issue.
- **Questions and ideas** — GitHub Discussions.
- **Security** — never a public issue. Report through a **private security
  advisory**: [`SECURITY.md`](SECURITY.md).
- **Conduct** — [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) applies everywhere
  in the project.
- **How a change gets decided** — [`GOVERNANCE.md`](GOVERNANCE.md): who merges,
  and what a device file needs before it does.

## Monorepo layout

| Directory | Contents |
| --------- | -------- |
| `devices/` | Source of truth: per-SKU YAML definitions |
| `docs/` | Mode model (`modes.md`), architecture and per-protocol documentation |
| `packages/rust/` | The protocol core, and the reference implementation |
| `packages/` | SDKs (python, node, php) plus the Art-Net bridge |
| `apps/` | Web playground and Electron app (not published) |
| `integrations/` | Matter bridge, Home Assistant, Homebridge |
| `tools/` | Local CI mirror and pointers to the development tools |
| `tests/fixtures/` | Real UDP / BLE captures per SKU, and the conformance vectors |

The Rust side is one published crate, `govee-toolkit`, rooted at
`packages/rust`:

| Path | What it is |
| ---- | ---------- |
| `src/codec/` | Codec: device file in, bytes out, no I/O |
| `src/lan/` | Transport: UDP, discovery, breaker. Behind the `lan` feature |
| `src/` | Facade: modes, configuration, events |
| `crates/sim/` | Device simulator. Never published |
| `crates/xtask/` | Generates `dist/catalog.json`. Never published |

`lan` is a default feature; `ble` and `cloud` join it as they land. With default
features off the codec builds on its own, and CI checks that it still does.
`tools/check-no-io.sh` is what keeps the codec free of sockets — under
`src/codec/`, do not import `std::net`, `std::fs`, `std::thread`, `tokio` or
`socket2`, and do not write an `async fn` or an `.await`.

Registry names elsewhere are `govee-toolkit` on PyPI and npm, and
`govee/toolkit` on Packagist.

SDKs **read** `devices/*.yaml`: they implement the transports and generic
parsing, and no protocol logic of their own. The protocol is implemented once,
in `packages/rust` — Node and Python bind to that core, PHP is the one
hand-written port. See [`docs/architecture.md`](docs/architecture.md).

`lan`, `ble` and `cloud` are **modes**, not a fallback chain: the user enables
one or several per device. A contribution must not make a mode implicit, and
must not silently substitute one mode for another — see
[`docs/modes.md`](docs/modes.md).

## Adding a SKU

1. Copy an existing file from `devices/` (or start from `devices/schema.yaml`).
2. Fill in capabilities, per-mode support level and the command tables.
3. Add a real capture under `tests/fixtures/lan-captures/<SKU>/`. Redact it
   first: [checklist](tests/fixtures/README.md#redaction-checklist).
4. Update the tables in [`docs/compatibility.md`](docs/compatibility.md).

Fill `verified` only from a device you ran the command on. Inference is not
verification: `?` or a `TODO` is a good answer, a guessed row is not.

Details: [`devices/README.md`](devices/README.md).

## Documenting an undocumented command

1. Try the payload through the **raw payload field** in the playground
   (`apps/playground/`).
2. Add the observed frame to `tests/fixtures/lan-captures/`, redacted:
   [checklist](tests/fixtures/README.md#redaction-checklist).
3. Document it in `docs/protocol/lan.md`: payload structure, compatible SKUs,
   sample frame, observed behavior.
4. Formalize the command in the relevant `devices/*.yaml` files.
5. Add a conformance vector under `tests/fixtures/golden/`, and say in its
   `source` whether the bytes come from a capture or were worked out from the
   documented layout. Only the first is evidence.

A failed probe and an unimplemented feature look identical — the firmware
answers nothing either way. Assume a malformed request before concluding a
device lacks a capability, and say which of the two you observed.

## Tests

Run the whole thing before pushing:

```bash
tools/qa.sh              # every check ci.yml runs, in the same order
tools/qa.sh clippy       # or one of them, by name
```

`qa.sh` reports a check whose tool is not installed as **skipped**, and names
the install command — a skip is not a pass, and it exits non-zero for one, so
a missing toolchain is discovered here rather than on the pull request. The
workflow stays the authority; `qa.sh` is a mirror of it kept in step by hand.

The individual commands, from `packages/rust`, for running one directly:

```bash
cargo test --all-features                             # unit, conformance, doc
cargo clippy --all-targets --all-features -- -D warnings
cargo +nightly fmt --all                              # nightly: see below
cargo deny check                                      # licenses and advisories
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
../../tools/check-file-length.sh                      # 400 lines per Rust file
../../tools/check-captures.sh                         # capture redaction
```

`rustfmt.toml` uses options that only nightly rustfmt implements — import
grouping and comment wrapping. CI formats with nightly, so `cargo fmt` on stable
will disagree with it. Use `cargo +nightly fmt`.

A Rust source file stays under **400 lines**. Rust has no such convention and
rustfmt enforces nothing, so this is a repository rule: it is a prompt to split
along responsibilities, not a target to hit. Per-function size is handled
separately by `clippy::too_many_lines`.

The MSRV in `packages/rust/Cargo.toml` is checked on every push. Raise it in the
same commit as the feature that needs it, never after the fact.

CI runs the test suite on **Linux, macOS and Windows**. This is not
box-ticking: multicast socket behavior differs on each — interface selection,
loopback of one's own datagrams, what binding to the wildcard address actually
joins — and discovery is the part that breaks. A change to the socket setup is
not done until the three of them are green.

`cargo test` also validates every `devices/*.yaml` and replays the conformance
vectors in `tests/fixtures/golden/`. Both directions fail the suite: a vector
naming a SKU no device file declares, and a command in the catalogue that no
vector covers. Add the vector in the same pull request as the command.

`packages/rust/crates/sim` lets you test without hardware. `cargo test` drives
it in-process on loopback ports; `cargo run -p govee-toolkit-sim` runs one on
the real ports for manual testing.

The Python, Node and PHP suites arrive with their packages. PHP, the one
hand-written port, runs the same conformance vectors as the core — that is the
whole reason the vectors exist.

## Releases

Each package is versioned and released independently through tags:
`rust-vX.Y.Z`, `python-vX.Y.Z`, `node-vX.Y.Z`, `php-vX.Y.Z`. What a version
number promises, and what counts as a breaking change, is in
[`docs/versioning.md`](docs/versioning.md).

`ci.yml` runs on push to `main` and on pull request, and skips every job while
the pull request is a draft. The release workflows are still `workflow_dispatch`
only — nothing is publishable yet; restore the tag trigger at the top of one
with its first real release.

## License

All contributions are accepted under the [MIT](LICENSE) license.

