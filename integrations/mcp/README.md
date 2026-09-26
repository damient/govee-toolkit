# govee-toolkit-mcp

A local MCP server over stdio. It describes the device catalog, the DMX
channel tables and the API of each language, serves the documentation, and
drives the devices over `lan`, `ble` and `cloud`.

It wraps the Node binding, `govee-toolkit`. It holds no device data and no
protocol logic: every value it returns comes from the binding, from
`dist/api.json` or from `docs/*.md`.

## Install

The server needs Node.js 20 or newer. `npx` fetches the package on the first
start. Each client below starts the server under the name `govee`.

Claude Code:

```sh
claude mcp add govee -- npx -y govee-toolkit-mcp@latest
```

Codex:

```sh
codex mcp add govee -- npx -y govee-toolkit-mcp@latest
```

Claude Desktop (`claude_desktop_config.json`) and Cursor (`.cursor/mcp.json`)
read an `mcpServers` object:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"]
    }
  }
}
```

VS Code (`.vscode/mcp.json`) reads a `servers` object:

```json
{
  "servers": {
    "govee": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"]
    }
  }
}
```

Replace `@latest` with a version to pin the server. The binding it installs is
the one at the version that the server declares.

## Configuration

The read tools need no configuration. The control tools read the
configuration that the `govee` command reads:

- `GOVEE_CONFIG` names the `config.yaml` that enables the modes of each
  device. Without it, the server reads
  `~/.config/govee-toolkit/config.yaml`. A file that is not there gives the
  default configuration, which is `lan` alone.
- `GOVEE_API_KEY` is the cloud API key, for `cloud` alone.

Give both in the `env` of the client's MCP configuration:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml",
        "GOVEE_API_KEY": "..."
      }
    }
  }
}
```

With Claude Code, give each variable with `-e`:

```sh
claude mcp add govee -e GOVEE_CONFIG=/home/me/govee.yaml -- npx -y govee-toolkit-mcp@latest
```

Give `GOVEE_CONFIG` as an absolute path. A relative path starts at the working
directory of the client, which the client chooses.

The client keeps the `env` block in plaintext in its configuration file. To
keep the key out of that file, put `GOVEE_API_KEY` in
`~/.config/govee-toolkit/.env`. The SDK reads that file last. See
[`../../docs/security.md`](../../docs/security.md).

## Tools

Read tools. They start no SDK, need no configuration and reach no network.
The values that a filter accepts come from the catalog and from `api.json`.

| Tool | Returns |
| ---- | ------- |
| `list_devices({ mode?, capability?, support? })` | One short row per SKU: name, family, the support of each mode |
| `describe_device({ sku })` | The record of `govee describe --json`, with the DMX tables under `dmx` |
| `get_dmx_profile({ sku, personality? })` | The DMX channel tables of one SKU |
| `get_api({ role?, language? })` | The methods that serve each role, per language |
| `read_doc({ topic })` | One page of `docs/`, as Markdown |

Control tools. The SDK starts on the first control call and closes when stdin
closes, or on `SIGINT` and `SIGTERM`.

| Tool | Does |
| ---- | ---- |
| `scan({ modes? })` | Runs a discovery scan and lists the devices that answered |
| `list_known()` | Lists the devices that the SDK knows, with the health of each mode |
| `status({ target, mode? })` | Asks a device for its state |
| `set({ target, mode?, power?, brightness?, color?, color_temp?, segment?, music?, gradient? })` | Sets the state of a device or of a group |
| `send({ target, mode?, command, args? })` | Sends one command that the device file declares |
| `identify({ targets?, mode? })` | Lights the devices one by one, so that a person can find each one |
| `doctor()` | Lists the problems in the configuration |

`mode` pins a call to one mode, which the device must enable. Without it, the
preference order of the device applies. A binding error returns
`isError: true` and the code of the binding, such as `mode_not_enabled`. A
tool never retries on another mode.

`set` sends its steps in the order that the core fixes: power on first and
power off last. A member that fails a step takes no later step, and the other
members go on. `send` reads a command whose entry declares an answer, and
sends any other one. It refuses a command that takes a secret, such as a
network password, with `secret_arg`.

The server does not provision Wi-Fi, because the password goes through the
model. It does not open a segment stream, because a stream does not fit a tool
call.

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
../../tools/qa-mcp.sh  # the build, the types, the lint and the tests
```

`npm run build` writes `data/`, which the package ships. Run it again after a
change to `docs/` or to a surface that `xtask api` reads.

stdout carries the JSON-RPC stream. The server writes every log to stderr.

## Release

The package releases under `mcp-vX.Y.Z`, through
`.github/workflows/mcp-release.yml`. It depends on `govee-toolkit` at the
version in `../../packages/node/package.json`, and the release fails when the
two differ. Push `node-vX.Y.Z` first. See
[`../../docs/versioning.md`](../../docs/versioning.md).
