# Changelog

Changes to `govee-toolkit-mcp`, the MCP server in `integrations/mcp`. It
versions apart from the binding it wraps and releases under `mcp-vX.Y.Z`. The
policy is [`../../docs/versioning.md`](../../docs/versioning.md).

### Changed

- `status` after a command from the server waits until the device reports
  the new state, for the time that the device file records.

## [0.2.0] — 2026-11-15

### Added

- `list_devices` and `describe_device` cover the models of Govee's LAN list
  that no device file describes. The record carries `listing`.

## [0.1.0] — 2026-11-14

### Added

- A local MCP server over stdio, with the bin `govee-toolkit-mcp`.
- Read tools over the embedded catalog: `list_devices`, `describe_device`,
  `get_dmx_profile`, `get_api` and `read_doc`. None of them starts the SDK
  or reaches the network.
- Control tools over the Node binding: `scan`, `list_known`, `status`,
  `set`, `send`, `identify` and `doctor`.
- The SDK starts on the first control call, reads the configuration that the
  CLI reads, and closes when stdin closes or on `SIGINT` and `SIGTERM`.
- `mode` pins a control call to one mode. A binding error returns its code,
  with no retry on another mode.
- `send` refuses a command that takes a secret, such as a network password,
  with `secret_arg`.
- `set` takes a device, a group or a SKU, and sends its steps in the order the
  core fixes. A member that fails a step takes no later step.
- A failed `set` names the error code and the identities that failed.
- `scan`, `list_known` and `status` answer the records that `govee --json`
  prints: a device carries one `{mode, health}` per mode, and a status
  carries `color` as `#RRGGBB`.
- `list_known` scans once, over each mode that no scan covered yet.
- Resources: each docs page at `gtk://docs/<topic>`, and each device record
  at `gtk://devices/{sku}`.
