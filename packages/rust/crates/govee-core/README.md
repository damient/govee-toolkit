# govee-core

Device catalog and protocol codec for [govee-toolkit]. No I/O: it turns
arguments into the exact bytes a device expects, and parses what comes back.

Protocol logic lives here and nowhere else. Per-SKU behavior is data, read from
`devices/*.yaml` — this crate holds no SKU name and no command name.

Unofficial and not affiliated with Govee.

[govee-toolkit]: https://github.com/damient/govee-toolkit
