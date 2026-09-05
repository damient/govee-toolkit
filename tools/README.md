# Tools

| Tool | Where |
| ---- | ----- |
| Device simulator | [`packages/rust/crates/govee-sim`](../packages/rust/crates/govee-sim) |
| Local CI mirror | [`qa.sh`](qa.sh), or `/qa` in Claude Code |

The simulator is a Rust crate rather than a tool of its own: the transport tests
drive it in-process on ephemeral loopback ports, and a second implementation of
the same wire protocol would be one more place for it to be wrong.

`qa.sh` runs the checks of `.github/workflows/ci.yml` in the same order and
prints a pass/fail summary. It reports a check whose tool is missing as skipped
rather than passed, and names the install command. The workflow stays the
authority; this is a mirror of it kept in step by hand.
