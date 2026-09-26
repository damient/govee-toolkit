# Changelog

Changes to `govee-toolkit-mcp`, the MCP server in `integrations/mcp`. It
versions apart from the binding it wraps and releases under `mcp-vX.Y.Z`. The
policy is [`../../docs/versioning.md`](../../docs/versioning.md).

### Added

- A local MCP server over stdio, with the bin `govee-toolkit-mcp`.
- Read tools over the embedded catalog: `list_devices`, `describe_device`,
  `get_dmx_profile`, `get_api` and `read_doc`. None of them starts the SDK
  or reaches the network.
- Control tools over the Node binding: `scan`, `list_known`, `status`,
  `set`, `send`, `identify` and `doctor`. The SDK starts on the first
  control call, reads the configuration that the CLI reads, and closes when
  stdin closes or on `SIGINT` and `SIGTERM`. `mode` pins a call to one mode,
  and a binding error returns its code with no retry on another mode. `send`
  refuses a command that takes a secret, such as a network password, with
  `secret_arg`. `set` sends its steps in the order the core fixes, and a
  member that fails a step takes no later step.
- Resources: each docs page at `gtk://docs/<topic>`, and each device record
  at `gtk://devices/{sku}`.
