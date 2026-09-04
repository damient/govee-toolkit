# Tools

| Tool | Where |
| ---- | ----- |
| Device simulator | [`packages/rust/crates/govee-sim`](../packages/rust/crates/govee-sim) |

The simulator is a Rust crate rather than a tool of its own: the transport tests
drive it in-process on ephemeral loopback ports, and a second implementation of
the same wire protocol would be one more place for it to be wrong.
