# Deprecations

The items that the next breaking release removes, for every package. The
policy is [`versioning.md`](versioning.md#deprecation).

Add a row in the commit that deprecates an item. Delete the row in the
commit that removes the item. A breaking release removes every row whose
"Deprecated in" version is released.

| Item | Package | Deprecated in | Replacement | How a user sees it |
| ---- | ------- | ------------- | ----------- | ------------------ |
| On macOS and Windows, the configuration folder `~/.config/govee-toolkit` | `rust`, `cli`, `dmx`, `server`, `python`, `node`, `mcp` | `rust` 1.18.0 | The folder of the system, see [`modes.md`](modes.md#where-the-files-are) | `paths::legacy_warning()`: a line on stderr from each binary, a `FutureWarning` in Python, a `DeprecationWarning` in Node |
| `Segments::armed`, always `false` | `rust` | `rust` 1.19.0 | None: `segment()` arms no channel on any mode | A `#[deprecated]` warning at build time |
