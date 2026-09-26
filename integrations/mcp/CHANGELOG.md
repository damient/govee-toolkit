# Changelog

Changes to `govee-toolkit-mcp`, the MCP server in `integrations/mcp`. It
versions apart from the binding it wraps and releases under `mcp-vX.Y.Z`. The
policy is [`../../docs/versioning.md`](../../docs/versioning.md).

### Added

- A local MCP server over stdio, with the bin `govee-toolkit-mcp`.
- Read tools over the embedded catalog: `list_devices`, `describe_device`,
  `get_dmx_profile`, `get_api` and `read_doc`. None of them starts the SDK
  or reaches the network.
- Resources: each docs page at `gtk://docs/<topic>`, and each device record
  at `gtk://devices/{sku}`.
