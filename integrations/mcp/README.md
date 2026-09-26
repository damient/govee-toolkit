# govee-toolkit-mcp

A local MCP server over stdio. It describes the device catalog, the DMX
channel tables and the API of each language, and serves the documentation.

It wraps the Node binding, `govee-toolkit`. It holds no device data and no
protocol logic: every value it returns comes from the binding, from
`dist/api.json` or from `docs/*.md`.

## Tools

| Tool | Returns |
| ---- | ------- |
| `list_devices({ mode?, capability?, support? })` | One short row per SKU: name, family, the support of each mode |
| `describe_device({ sku })` | The record of `govee describe --json`, with the DMX tables under `dmx` |
| `get_dmx_profile({ sku, personality? })` | The DMX channel tables of one SKU |
| `get_api({ role?, language? })` | The methods that serve each role, per language |
| `read_doc({ topic })` | One page of `docs/`, as Markdown |

The read tools start no SDK, need no configuration and reach no network. The
values that a filter accepts come from the catalog and from `api.json`.

## Resources

- `gtk://docs/<topic>`: one docs page, such as `gtk://docs/protocol/lan`.
- `gtk://devices/{sku}`: one device record, as `describe_device` returns it.

## Build from the repository

The server needs the binding built from this checkout, and `dist/api.json`.

```sh
(cd ../../packages/node && npm ci && npm run build:debug)
(cd ../../packages/rust && cargo run -p xtask -- api)
npm ci
npm run link:binding   # node_modules/govee-toolkit -> packages/node
npm run build          # data/ from dist/api.json and docs/, then dist/
npm run lint
```

`npm run build` writes `data/`, which the package ships. Run it again after a
change to `docs/` or to a surface that `xtask api` reads.

stdout carries the JSON-RPC stream. The server writes every log to stderr.
