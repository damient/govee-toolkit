# Tools

| Tool | Where |
| ---- | ----- |
| Device simulator | [`packages/rust/crates/sim`](../packages/rust/crates/sim) |
| Local CI mirror | [`qa.sh`](qa.sh), or `/qa` in Claude Code |
| Redaction check | [`check-captures.sh`](check-captures.sh) |
| Rust file length | [`check-file-length.sh`](check-file-length.sh) |
| Codec layering | [`check-no-io.sh`](check-no-io.sh) |
| Release notes from the changelog | [`release-notes.sh`](release-notes.sh) |
| Generated catalog and tables | [`packages/rust/crates/xtask`](../packages/rust/crates/xtask) |

The simulator is a Rust crate rather than a tool of its own: the transport tests
drive it in-process on ephemeral loopback ports, and a second implementation of
the same wire protocol would be one more place for it to be wrong.

`qa.sh` runs the checks of `.github/workflows/ci.yml` in the same order and
prints a pass/fail summary. It reports a check whose tool is missing as skipped
rather than passed, and names the install command. The three it leaves out —
sign-off, commit convention and changelog entry — walk a pull request's commit
range, which does not exist locally. The workflow stays the authority; this is
a mirror of it kept in step by hand.

`check-captures.sh` scans every tracked file under `tests/fixtures/` and
`devices/` for what a packet capture carries out of a home network: a MAC that
is not one of the documented placeholders, an IPv4 address outside the RFC 5737
documentation ranges, a credential, an `ssid` or `bssid` with a value. It prints
the file and line, and is quiet when there is nothing to report. The
placeholders it accepts, and what to substitute for what, are in
[`../tests/fixtures/README.md`](../tests/fixtures/README.md).

Its patterns are narrow on purpose — a false positive that blocks a legitimate
capture costs more than a miss — so it is a backstop for the checklist, not a
replacement for reading the capture.

`check-no-io.sh` fails when anything under `packages/rust/src/codec/` imports
`std::net`, `std::fs`, `std::thread`, `tokio` or `socket2`, or writes an
`async fn` or an `.await`. The codec
does no I/O, and with the Rust side a single crate this script is what
guarantees it. `cargo check --no-default-features` is the other half of it.

`release-notes.sh` takes a package and a release tag, and prints the changelog
section for that version:

```bash
tools/release-notes.sh rust rust-v0.3.0
```

It fails when the tag, the version in the package manifest and the changelog
heading are not the same number, or when the section exists with no entries
under it. The release workflows run it as their first step, so a tag pushed
past a manifest nobody bumped stops there instead of publishing.

`xtask` generates what is derived from `devices/*.yaml`:

```bash
cd packages/rust
cargo run -p xtask                    # dist/catalog.json, the release artifact
cargo run -p xtask -- compat          # the tables in docs/compatibility.md
cargo run -p xtask -- compat --check  # fails when they have drifted
```
