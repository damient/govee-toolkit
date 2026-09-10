# govee-toolkit-cli

The command line over [`govee-toolkit`](../../README.md). Unofficial, and not
affiliated with Govee.

The binary is `govee`.

```sh
cargo install govee-toolkit-cli
govee scan
```

## What it carries

The CLI holds no protocol logic. It reads `devices/*.yaml` through the crate,
and every byte it sends is built by the codec.

Two kinds of subcommand:

- `send` names an entry of the device file. It works for a SKU this build has
  never heard of, and it carries no command name in this crate.
- The verbs — `on`, `brightness`, `color` — name one. Each verb reaches the
  device file through a `role:`, so that the Node and the Python packages get
  the same verb from the core and not from a second implementation. A device
  file that claims no entry for the role fails and names the role: no verb is
  served through another command.

## Modes

`--mode` restricts the run to one mode. It never enables a mode the
configuration leaves out, and it never falls back to another one. A device that
no enabled mode reaches is an error — see [`docs/modes.md`](../../../../docs/modes.md).

## Output

`--json` writes one JSON object per line on stdout, and an error object on
stderr. That form is the contract, and an error's `kind` is the core's own
error code. The text form is for a person, and its layout can change at any
release.

Exit codes:

| Code | Meaning |
| ---- | ------- |
| 0 | Success. |
| 1 | A failure no other code names. |
| 2 | The command line is wrong. |
| 3 | The configuration cannot be read or cannot be applied. |
| 4 | No enabled mode reaches the device. |
| 5 | The command or an argument is refused. Nothing was sent. |
| 6 | This build does not carry what the command line names. |

## State

`scan`, `devices`, `on`, `off`, `brightness` and `color` work. `describe`,
`send`, `status` and `segment` are declared and answer with exit code 6.
