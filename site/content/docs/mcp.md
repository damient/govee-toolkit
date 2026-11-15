---
title: MCP server
slug: mcp
order: 8
description: Let an AI assistant read the device catalog and drive your lights, through a local MCP server.
---

# MCP server

`govee-toolkit-mcp` is a local MCP server. Your assistant reads the device
catalog through it and drives your lights over {{badge_lan}}, {{badge_ble}}
and {{badge_cloud}}, with the same configuration as the `govee` command.

## What you need

- An assistant installed on your computer that accepts MCP servers.
- For control, the `config.yaml` that [Configure]({{base}}docs/configure/)
  describes. The read tools work as soon as the server starts.

## Add the server

Pick your assistant. Each one starts the server under the name `govee`.

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copy</button>
</div>
@tab Claude Desktop
Add this to `claude_desktop_config.json`:

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
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copy</button>
</div>
@tab Cursor
Add this to `.cursor/mcp.json`:

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
@tab VS Code
Add this to `.vscode/mcp.json`:

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
:::

## Point it at your configuration

The server reads two variables:

- `GOVEE_CONFIG` names your `config.yaml`. The default is
  `~/.config/govee-toolkit/config.yaml`.
- `GOVEE_API_KEY` is the API key that {{badge_cloud}} needs.

Give them to your assistant:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copy</button>
</div>
@tab Claude Desktop
Add `env` to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml"
      }
    }
  }
}
```
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee --env GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee --env GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copy</button>
</div>
@tab Cursor
Add `env` to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml"
      }
    }
  }
}
```
@tab VS Code
Add `env` to `.vscode/mcp.json`:

```json
{
  "servers": {
    "govee": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml"
      }
    }
  }
}
```
:::

Write `GOVEE_CONFIG` as an absolute path: the client picks the working
directory. The client keeps its `env` block in plain text, so put the API key
in `~/.config/govee-toolkit/.env` instead.

## What the assistant can do

Read tools answer from the catalog that ships with the server, on your machine.

| Tool | Returns |
| ---- | ------- |
| `list_devices` | One row per model, with the support of each mode |
| `describe_device` | The full record of one model, with its DMX tables |
| `get_dmx_profile` | The DMX channel tables of one model |
| `get_api` | The methods that serve each role, in each language |
| `read_doc` | One page of the documentation |

Control tools drive the lights. The SDK starts on the first call.

| Tool | Does |
| ---- | ---- |
| `scan` | Finds the devices on your network |
| `list_known` | Lists the devices found, with the health of each mode |
| `status` | Reads the state of a device |
| `set` | Sets power, brightness, color, white temperature, a segment, a gradient or music mode |
| `send` | Sends one command that the device file declares |
| `identify` | Lights the devices one by one, so you see which is which |
| `doctor` | Lists the problems in your configuration |

A target is a device, a group from [Names and groups]({{base}}docs/targets/),
or every device of one model.

## Modes stay explicit

`mode` pins a call to one mode, which the device must enable. Otherwise, the
order in your `config.yaml` applies. A call that fails returns the error code
of the SDK, such as `mode_not_enabled`, and stays on the mode it used.

To put a device on your Wi-Fi or to stream segments, use the `govee` command
or a binding. The Wi-Fi password then stays between you and the device.
