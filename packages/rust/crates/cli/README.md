# govee-toolkit-cli

Discover Govee devices and send commands from your terminal, over the LAN, over
Bluetooth or through the cloud. The binary is `govee`.

**Documentation: [gvetk.com](https://gvetk.com)**

[![govee-toolkit-cli on crates.io](https://img.shields.io/crates/v/govee-toolkit-cli?logo=rust&logoColor=white&label=crates.io)](https://crates.io/crates/govee-toolkit-cli)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/damient/govee-toolkit/blob/main/LICENSE)
[![ci](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/damient/govee-toolkit/actions/workflows/ci.yml)

> Community project. Not affiliated with, sponsored by or endorsed by Govee.

```sh
cargo install govee-toolkit-cli
govee scan
govee color living-room "#ff3d00"
```

`ble` and `cloud` are opt-in features: `cargo install govee-toolkit-cli
--features ble,cloud`.

## Start here

| You want to | Go to |
| ----------- | ----- |
| Install it and send a first command | [gvetk.com/docs/start](https://gvetk.com/docs/start/) |
| Know whether your model works | [gvetk.com/devices](https://gvetk.com/devices/) |
| Pick and configure the modes | [gvetk.com/docs/modes][modes] |
| Read every command, with an example | [gvetk.com/reference](https://gvetk.com/reference/) |

## Commands

| Command | What it does |
| ------- | ------------ |
| `scan` | Discover devices and report what answered. |
| `devices` | List the devices already known, without touching the network. |
| `describe <target>` | Report what a device file declares. Reads no hardware. |
| `doctor` | Report everything wrong with the configuration. |
| `status <device>` | Ask the device for its state. |
| `on`, `off`, `brightness`, `color` | The verbs, by `role:`. |
| `colortemp <device> <kelvin>` | Set the white temperature. |
| `segment <device> [--zones] <color>` | Paint zones one color. |
| `music <device> <effect>` | Play an effect from the device's own microphone. |
| `send <device> <command> --arg n=v` | One device file entry, by name. |
| `stream <device>` | Feed the segment channel, one frame per line of stdin. |
| `watch` | Print events as they arrive. |
| `provision <device> --ssid` | Put a device on a Wi-Fi network over `ble`. |

The CLI holds no protocol logic. `send` names an entry of the device file, so
it works for a SKU this build has never heard of. Each verb reaches the device
file through a `role:`, so the Node and the Python packages get the same verb
from the core. A device file that claims no entry for the role fails and names
the role: no verb is served through another command.

Every command above works. `provision` needs the `ble` feature, and it sends
the Wi-Fi password in plaintext — anything in Bluetooth range while it runs
reads it. [gvetk.com/docs/configure](https://gvetk.com/docs/configure/) covers
the credentials and the `.env` file every subcommand reads.

## Modes

`--mode` restricts the run to one mode. It never enables a mode the
configuration leaves out, and it never falls back to another one. A device that
no enabled mode reaches is an error — see [gvetk.com/docs/modes][modes].

## Output

`--json` writes one JSON object per line on stdout, and an error object on
stderr. That form is the contract, and an error's `kind` is the core's own
error code. The text form is for a person, and its layout can change at any
release.

| Exit code | Meaning |
| --------- | ------- |
| 0 | Success. |
| 1 | A failure no other code names. |
| 2 | The command line is wrong. |
| 3 | The configuration cannot be read or cannot be applied. |
| 4 | No enabled mode reaches the device. |
| 5 | The command or an argument is refused. Nothing was sent. |
| 6 | This build does not carry what the command line names. |

## License

[MIT](https://github.com/damient/govee-toolkit/blob/main/LICENSE)

<!-- Absolute: this file is the crate description on crates.io, where a
     relative link out of the package directory is dead. -->
[modes]: https://gvetk.com/docs/modes/
