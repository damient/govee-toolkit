# Tools

| Tool | Where |
| ---- | ----- |
| Device simulator | [`packages/rust/crates/sim`](../packages/rust/crates/sim) |
| Local CI mirror | [`qa.sh`](qa.sh), or `/qa` in Claude Code |
| Redaction check | [`check-captures.sh`](check-captures.sh) |
| Rust file length | [`check-file-length.sh`](check-file-length.sh) |
| Codec layering | [`check-no-io.sh`](check-no-io.sh) |
| Generated catalog and tables | [`packages/rust/crates/xtask`](../packages/rust/crates/xtask) |

The simulator is a Rust crate rather than a tool of its own: the transport tests
drive it in-process on ephemeral loopback ports, and a second implementation of
the same wire protocol would be one more place for it to be wrong.

`qa.sh` runs the checks of `.github/workflows/ci.yml` in the same order and
prints a pass/fail summary. It reports a check whose tool is missing as skipped
rather than passed, and names the install command. The DCO job is the one it
leaves out — that one walks a pull request's commit range, which does not exist
locally. The workflow stays the authority; this is a mirror of it kept in step
by hand.

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
`std::net`, `std::fs`, `tokio` or `socket2`, or writes an `async fn`. The codec
does no I/O; that used to be guaranteed by a crate boundary and is guaranteed by
this now that the Rust side is one crate. `cargo check --no-default-features` is
the other half of it.

`xtask` generates what is derived from `devices/*.yaml`:

```bash
cd packages/rust
cargo run -p xtask                    # dist/catalog.json, the release artifact
cargo run -p xtask -- compat          # the tables in docs/compatibility.md
cargo run -p xtask -- compat --check  # fails when they have drifted
```
