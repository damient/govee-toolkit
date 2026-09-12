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
  never heard of, and it carries no command name in this crate. The entry
  declares the type of every argument, so `--arg name=value` is read under that
  type and the range stays the codec's to check.
- The verbs — `on`, `brightness`, `color`, `colortemp`, `segment`, `music` —
  name one.
  Each verb reaches the device file through a `role:`, so that the Node and the
  Python packages get the same verb from the core and not from a second
  implementation. A device file that claims no entry for the role fails and
  names the role: no verb is served through another command.

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

`colortemp` ends the color the device showed: white and color are mutually
exclusive states. Where the mode carries the RGB rendering of the temperature
in the same frame, the SDK computes it and sends both. To send another
rendering, name the entry with `send`.

`music` starts an effect and sends nothing per beat: the device listens on its
own microphone. The effect identifiers are the mode's own, so the same number
names another effect over another mode, and `describe` reports the range each
mode takes. Nothing stops the effect — `on`, `off`, `color` or `colortemp`
ends it.

`segment --zones` paints the zones it names and leaves the rest alone, which
needs a mode whose device file marks `role: segment_color_masked`. Without
`--zones` every zone takes the color, over whichever painting role the file
marks. Nothing disarms the segment channel afterwards: a disarm ends the
channel, and the colors with it.

`scan` prints every device that answered. One the configuration does not
enable the scanned mode for is printed as answering over that mode: it is on
the air whatever the configuration says. Over `ble` that is every first scan,
since the device is reported under the handle the platform gives the
peripheral and nothing relates that handle to a Wi-Fi MAC — put it under
`devices:` to send it anything. `devices` lists only what a command can go to.

A command that names a device scans first where no transport knows it yet.
`ble` relates a device to a handle through an advertisement alone and keeps
nothing across runs, so a one-shot command has to discover it. A device already
known costs no scan: the `lan` cache answers from disk.

`provision` needs the `ble` feature. The password travels in plaintext, with no
key exchange: anything in Bluetooth range while it runs reads it. Supply it
with `--password`, or in `GOVEE_WIFI_PASSWORD`, or pass `--open` for a network
that has none. The network name comes from `--ssid`, or from `GOVEE_WIFI_SSID`.
The command line wins over the variables.

## Variables and `.env`

Every subcommand reads the `GOVEE_*` variables — `GOVEE_API_KEY` for `cloud`,
`GOVEE_WIFI_SSID` and `GOVEE_WIFI_PASSWORD` for `provision`. They come from the
environment first, then from the first `.env` the search finds: it starts in the
working directory and goes up, and it stops after the directory that holds
`.git` or after your home directory. `~/.config/govee-toolkit/.env` is read
last, which is where an installed command usually keeps them.

A missing file is not an error. `govee doctor` says which file the variables
came from.

```bash
govee --no-env doctor                 # the environment alone
govee --env-file ~/other.env doctor   # one named file; absent is an error
```

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

Every command above works. `provision` is built only with the `ble` feature,
and `cloud` joins `lan` and `ble` when it lands.
