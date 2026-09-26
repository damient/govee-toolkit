# MCP server plan

This page is the work plan for `integrations/mcp`, a local MCP server that
describes the catalog and drives the devices. Several agents work on it in
parallel on the branch `feat/mcp`. Each work package names the files it owns,
what it depends on, and when it is done.

Delete this page, or fold what stays true into `docs/architecture.md` and
`integrations/mcp/README.md`, when the last work package lands.

## Decisions

These are made. Do not re-open them in a work package.

| Topic | Decision |
| ----- | -------- |
| SDK | `@modelcontextprotocol/server` v2, `McpServer`, zod v4 schemas. |
| Transport | stdio only. Nothing is hosted. |
| Scope | Read: catalog, modes, DMX, capabilities, the API of each language. Control: `lan`, `ble` and `cloud`, through the Node binding. |
| DMX | Read only. The DMX bridge stays the `govee-dmx` binary. |
| Source of truth | The Node binding, `govee-toolkit` on npm. The MCP holds no device data and no protocol logic. |
| Catalog updates | The catalog is the one embedded in the binding. No remote catalog: `payload:` and `frame:` are executable, so a downloaded catalog is a source of bytes on the user's network. A user gets a new device with a new package release. |
| Confirmation | No confirmation on control tools. They turn lights on, they do not destroy data. |
| Location | `integrations/mcp`, released independently as `mcp-vX.Y.Z`. |
| npm name | `govee-toolkit-mcp`, with a `bin` of the same name. |
| Python parity | Every addition to the Node `Catalog` also goes into the Python `Catalog`. |

## The rule against drift

The MCP reads three things, and each one comes from generated code:

1. The device data: `Catalog.describe()` and `Catalog.dmx()` in the binding.
2. The API of each language: `dist/api.json`, written by `xtask api`.
3. The docs: `docs/*.md`, copied into the package at build time.

The MCP source must not contain:

- a SKU name,
- a command name,
- a capability list, a mode list or a role list written by hand.

A tool that filters by mode or capability reads the valid values from the
catalog or from `api.json`. Constraint 3 of `AGENTS.md` applies to the MCP as
it applies to the CLI: a tool can name a role, never a command.

## Work packages

```text
WP0 core ─┬─> WP1 node ─┬─> WP4 mcp read ──> WP5 mcp control ─┐
          ├─> WP2 python │                                     ├─> WP7 release + docs
          └─> WP3 xtask api ──────────────┘                     │
                                          WP6 mcp tests ────────┘
```

WP1, WP2 and WP3 can run in parallel once WP0 lands. WP4 needs WP1 and WP3.
WP6 starts with WP4 and grows with WP5.

### WP0: describe and DMX in the core

**Owns:** `packages/rust/src/describe.rs`, `packages/rust/src/profile/`,
`packages/rust/src/codec/catalog/`, `packages/rust/crates/xtask/src/dmx.rs`,
`packages/rust/CHANGELOG.md`.

- Make the `describe` record available from a catalog and a SKU, without a
  started SDK.
- Move the logic of `xtask/src/dmx.rs::catalog_entry` into the core, in the
  `profile` module. It must stay I/O-free: `tools/check-no-io.sh` must pass.
- Make xtask call the core function, so `dist/catalog.json` does not change.

**Done when:**

- `tools/qa.sh -p rust` and `tools/qa.sh -p xtask` pass.
- `cargo run -p xtask -- catalog` writes a `dist/catalog.json` identical to the
  one before the change.

### WP1: Node `Catalog.describe()` and `Catalog.dmx()`

**Owns:** `packages/node/src/catalog.rs`, `packages/node/test/`,
`packages/node/CHANGELOG.md`. `binding.d.cts` is regenerated, not edited.

- `Catalog.describe(sku)` returns the record that `govee describe --json`
  prints. It throws `unknown_sku` when nothing declares the SKU.
- `Catalog.dmx(sku)` returns the channel tables of `dist/catalog.json` for
  that SKU. It throws `unknown_sku` in the same way.
- Both read no hardware.

**Done when:** `tools/qa-node.sh` passes, with one test per method and one test
for `unknown_sku`.

### WP2: Python `Catalog.describe()` and `Catalog.dmx()`

**Owns:** `packages/python/src/`, `packages/python/govee_toolkit/_govee_toolkit.pyi`,
`packages/python/tests/`, `packages/python/CHANGELOG.md`.

- The same two methods as WP1, with the same errors.
- Add both to the stub. `mypy.stubtest` and `tools/sync-stubs.py --check` catch
  a stub that differs from the module.

**Done when:** `tools/qa-python.sh` passes.

### WP3: `xtask api` and `dist/api.json`

**Owns:** `packages/rust/crates/xtask/src/api.rs`, `packages/rust/crates/xtask/src/main.rs`,
`tools/qa.sh` (one added check), `CHANGELOG.md`.

`dist/api.json` joins each role to the method that serves it on each surface:

```json
{
  "schema_version": 1,
  "roles": {
    "color_temp": {
      "cli": "govee color-temp <KELVIN>",
      "rust": "DeviceHandle::color_temp(kelvin: i64)",
      "node": "DeviceHandle.colorTemp(kelvin: number)",
      "python": "DeviceHandle.color_temp(kelvin: int)"
    }
  },
  "methods": { "node": [], "python": [], "rust": [], "cli": [] }
}
```

`methods` lists the full public surface of each language, and `roles` is the
join on it. Read each surface from what generates or checks it:

| Surface | Source |
| ------- | ------ |
| Roles | the `Role` enum in `codec/catalog/spec.rs` |
| Rust | `packages/rust/src/verbs/` and the public handle types, parsed with `syn` |
| CLI | the `Verb` and `Command` enums in `crates/cli/src/cli/`, parsed with `syn`, with the clap kebab-case rule |
| Node | `packages/node/binding.d.cts`, generated by napi |
| Python | `packages/python/govee_toolkit/_govee_toolkit.pyi`, checked by stubtest |

- `xtask api` writes the file. `xtask api --check` fails when a role has no
  method on a surface.
- A role that a surface leaves out on purpose goes in one short list in
  `api.rs`, with the reason next to each entry.
- Do not add a library target to the CLI crate for this. Its public surface is
  its command line.
- Add `xtask api --check` to `tools/qa.sh`, next to `compat` and `dmx`.

**Done when:** `tools/qa.sh -p xtask` passes, and `tools/qa.sh "api"` fails
when a verb is removed from one surface.

### WP4: MCP server, read tools

**Owns:** `integrations/mcp/` except `src/tools/control.ts`.

Layout:

```text
integrations/mcp/
  package.json        govee-toolkit-mcp; govee-toolkit at an exact version
  tsconfig.json
  CHANGELOG.md
  README.md
  scripts/copy-data.ts  copies dist/api.json and docs/*.md into data/
  data/               build output, gitignored
  src/server.ts       McpServer, serveStdio, shutdown
  src/sdk.ts          one Govee instance, started on the first control call
  src/resources.ts
  src/tools/catalog.ts
  src/tools/control.ts  (WP5)
  test/
```

Read tools. Each one carries `readOnlyHint: true`, an `outputSchema`, and
returns `structuredContent`. None of them starts the SDK or needs a
configuration.

| Tool | Reads | Returns |
| ---- | ----- | ------- |
| `list_devices({ mode?, capability?, support? })` | `Catalog.skus()`, `Catalog.describe()` | one short row per SKU: SKU, name, family, support per mode |
| `describe_device({ sku })` | `Catalog.describe()`, `Catalog.dmx()` | the full record, DMX tables included |
| `get_dmx_profile({ sku, personality? })` | `Catalog.dmx()` | the channel table |
| `get_api({ role?, language? })` | `data/api.json` | the methods that serve a role, per language |
| `read_doc({ topic })` | `data/*.md` | one doc page |

- Expose each doc page as a resource too: `gtk://docs/<topic>` and
  `gtk://devices/{sku}`. Many clients do not let the model read a resource on
  its own, so `read_doc` must stay.
- `read_doc` takes the topics that `scripts/copy-data.ts` copied. The list is
  read from `data/`, not written in the source.
- Never return the whole catalog in one answer. `list_devices` returns short
  rows; `describe_device` returns the detail.
- A tool description tells the model when to call the tool and what it does not
  do. It states that `?` and `unknown` mean nobody verified the value, not that
  the device supports it.
- stdout carries the JSON-RPC stream. Write every log to stderr.
- A source file stays under 300 lines, as on the site.

**Done when:** `tools/qa-mcp.sh` passes, and the WP6 read tests pass.

### WP5: MCP control tools

**Owns:** `integrations/mcp/src/tools/control.ts`, `integrations/mcp/src/sdk.ts`.

Each tool carries `readOnlyHint: false`, `destructiveHint: false` and
`openWorldHint: true`.

| Tool | Binding call |
| ---- | ------------ |
| `scan({ modes? })` | `Govee.scan()`, `Govee.scanOn()` |
| `list_known()` | `Govee.devices()`, with the health per mode |
| `status({ target })` | `DeviceHandle.status()` |
| `set({ target, mode?, power?, brightness?, color?, color_temp?, segment?, music?, gradient? })` | the verbs of `DeviceHandle`, or of `GroupHandle` when the target is a group |
| `send({ target, mode?, command, args? })` | `DeviceHandle.send()`: the command name comes from the device file |
| `identify({ targets?, mode? })` | `Govee.identify()` |
| `doctor()` | `Govee.problems()` |

- `sdk.ts` starts one `Govee` on the first control call. It calls `close()` when
  stdin closes and on `SIGINT` and `SIGTERM`: `ble` loses the last frame
  otherwise.
- The configuration is the one the CLI reads. `GOVEE_CONFIG` and the cloud key
  come from the `env` of the client's MCP configuration. The MCP reads no other
  file.
- `mode` pins the call with `deviceOn()` or `group(target, mode)`. Without it,
  the device's own preference order applies.
- A binding error returns `isError: true` with the binding's code
  (`mode_not_enabled`, `unknown_sku`, and so on). The tool never retries on
  another mode.
- The MCP does not expose `provisionWifi`. The password would go through the
  model, then in plaintext over `ble`.
- The MCP does not expose `openStream`. A frame stream does not fit a tool call.

**Done when:** `tools/qa-mcp.sh` passes, and the WP6 control tests pass
against the simulator.

### WP6: MCP tests

**Owns:** `integrations/mcp/test/`, `tools/qa-mcp.sh`.

- Connect a v2 client to the server in-process and list the tools. Compare
  names and schemas to a snapshot.
- Call every read tool on the embedded catalog. Assert the shape against the
  `outputSchema`.
- Assert that `list_devices` returns every SKU of `Catalog.skus()`, and that
  `get_api` returns every role of `api.json`.
- Drive the control tools against `crates/sim` over `lan`: `scan` finds the
  simulator, `set` with `power` and `brightness` returns the mode that served it.
- Assert that nothing is written to stdout except JSON-RPC.
- `tools/qa-mcp.sh` runs `tsc`, `oxlint` and `node --test`. `tools/qa.sh` runs
  it as one of its checks, and `tools/check-file-length.sh` covers
  `integrations/mcp/src/` at 300 lines.

### WP7: release and docs

**Owns:** `.github/workflows/mcp-release.yml`, `.github/workflows/ci.yml` (one
job), `tools/release-notes.sh`, `docs/versioning.md`, `docs/features.md`,
`docs/README.md`, `CHANGELOG.md`, `integrations/mcp/README.md`.

- `mcp-release.yml` copies `node-release.yml`: it waits for CI on the tagged
  commit, then publishes to npm through trusted publishing.
- The release fails when the `govee-toolkit` version in
  `integrations/mcp/package.json` differs from `packages/node/package.json` at
  the tagged commit. The docs and `api.json` in the package then describe the
  binding it depends on.
- `tools/release-notes.sh` accepts `mcp`. `docs/versioning.md` gains the row
  `mcp-vX.Y.Z` / `govee-toolkit-mcp` / npm.
- Push `node-vX.Y.Z` first, then `mcp-vX.Y.Z`. One tag per push.
- `integrations/mcp/README.md` gives the install line for each client, for
  example `claude mcp add govee -- npx -y govee-toolkit-mcp@latest`, and the
  `env` block for `GOVEE_CONFIG` and the cloud key.
- `AGENTS.md` already names the MCP, its sources and `tools/qa-mcp.sh`.
  Correct it where the shipped package differs from the plan.
- `docs/features.md`: one row for the MCP server.

## Rules for every agent

- Stay inside the files your work package owns. Where you must touch another
  package's file, say so in the commit body.
- Every change to `packages/*/src/` carries a changelog entry. `/changelog`
  writes it.
- Run the narrowest check from the table in `AGENTS.md` before you hand over,
  and `tools/qa.sh` before the branch is pushed.
- Commit only when asked. Commit subjects follow Conventional Commits, with
  `Signed-off-by`, and carry no tool attribution.
