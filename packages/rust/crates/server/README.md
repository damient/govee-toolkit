# govee-toolkit-server

The local HTTP server of Studio, the app that controls the lights. The `govee-server` binary runs it on
its own, and the desktop app in `apps/desktop` runs it in its process.

## Install

```sh
cargo install govee-toolkit-server
govee-server
```

Open <http://127.0.0.1:4080>. The binary carries the web UI. On Linux the
`ble` mode needs `libdbus-1-dev` to build.

## Linux

The desktop app targets Windows and macOS. On Linux, `govee-server` serves
Studio to a browser on this machine, or to a phone or a tablet on the
network.

[`systemd/govee-server.service`](systemd/govee-server.service) runs the
server as a systemd user service, so it starts at boot and restarts after a
failure:

```sh
cargo install govee-toolkit-server
cp systemd/govee-server.service ~/.config/systemd/user/
systemctl --user enable --now govee-server
sudo loginctl enable-linger "$USER"
```

`enable-linger` starts the service at boot, before a login. Read the output
with `journalctl --user -u govee-server`.

On a machine with no screen, such as a Raspberry Pi, no browser on that
machine can switch sharing on. In the file, put a `#` before the first
`ExecStart` line and remove the `#` before the second. The second line
opens the server to the network with `--listen 0.0.0.0:4080`. Then
open `http://<address of the machine>:4080` on a phone or a computer of the
network. The API has no authentication: any device on the network can
control the lights.

## Run from the repository

`build.rs` compiles `apps/studio/dist` into the binary. Build the UI
first, or the binary serves the API alone:

```sh
npm --prefix apps/studio install
npm --prefix apps/studio run build
cargo run -p govee-toolkit-server
```

`--web <directory>` serves the UI from a directory and not from the binary.
`npm --prefix apps/studio run serve` uses it on `dist/`, so a rebuild of
the UI shows on a reload.

`--listen` sets the address. The default is `127.0.0.1:4080`, the loopback
alone. The API has no authentication, so a device that reaches the server
can control the lights. Two ways open Studio to a phone or a tablet:

- The Mobile page of Studio, or `POST /api/share`, adds a listener on the
  network address of this machine, on the same port when it is free.
  Sharing is off by default, and only a client on this machine can switch
  it. The server keeps the switch and shares again at its next start. When
  the network is not up yet, as at boot, it tries again every 5 s. When
  sharing stops, the server refuses the network again.
- `--listen 0.0.0.0:4080` opens the server to the network for as long as it
  runs.

## Files

The server keeps its files in the configuration folder of the system:
`~/Library/Application Support/govee-toolkit` on macOS,
`$XDG_CONFIG_HOME/govee-toolkit` on Linux and `%APPDATA%\govee-toolkit` on
Windows. See [`docs/modes.md`](../../../../docs/modes.md#where-the-files-are).

- `studio.json`: two parts, `server` and `studio`. Each part holds a
  `version` and `settings`. The `server` settings hold the share switch, the
  `studio` settings the theme, the language and the `advanced` switch. Each write goes to a temporary file, then a rename replaces the
  file. A file that does not parse stops the start.
- `rooms.json`: the rooms of the lights, with its own `version`. A room
  has an `id`, a `name` and the IDs of its `devices`. A room can be empty,
  and a device can be in more than one room. A file that does not parse
  stops the start.
- `config.yaml`: the configuration of the SDK, which `GOVEE_CONFIG` can
  name. The server reads it, and writes its device entries on
  `PUT /api/devices`, a reset and an import.
- `credentials.env`: the API key, written on an import with the
  permissions `0600`. The SDK reads it after every other source.

A backup from `GET /api/backup` holds all four, the API key included when
one is set. Keep the file private.

## Compatibility

`studio.json` and a backup carry two versions:

- `studio.version`: the Studio release that gave the Studio settings their
  shape, such as `0.1.0`.
- `server.version`: the `govee-server` that wrote the file.

An import accepts a Studio version that this table holds, and `src/compat.rs`
holds the same table. A file from a newer Studio or a newer server gets an
answer that says to update `govee-server`. A Studio release that changes the
shape of the settings adds a row.

| Studio settings | Read by `govee-server` |
| --------------- | ---------------------- |
| 0.1.0           | 0.1.0 and later        |

## Routes

| Route | Answer |
| ----- | ------ |
| `GET /api/devices` | `{ "devices": [{ "id", "sku", "name", "family" }] }`: the devices of `config.yaml`, at once, with no scan. `sku` and `family` are `null` when the entry gives no SKU. An error answers 500 and `{ "error" }` |
| `GET /api/scan` | `{ "devices": [...] }`: the devices that answer a `lan` scan, in the record of `govee list --json`. Each record adds `family`, `name` (from `config.yaml`) and `configured` (`config.yaml` has an entry). The call takes one scan window, 2 s by default. An error answers 500 and `{ "error" }` |
| `PUT /api/devices` | `{ "keep": [{ "id", "sku", "name" }], "remove": ["id"] }`, written to `config.yaml` in one go. An entry with no `sku` gets the one of the request, and a new entry also gets `modes: [lan]`. A blank `name` removes the name. A name is 64 characters at most, or 400. A removed device leaves its rooms. The SDK starts again on the next request. The other keys stay, without their comments |
| `GET /api/rooms` | `{ "rooms": [{ "id", "name", "devices" }] }`, in the order of `rooms.json` |
| `PUT /api/rooms` | `{ "rooms": [{ "id", "name", "devices" }] }` replaces every room, and answers as `GET`. A room with no `id` gets `room-<n>`. A blank name, a name longer than 64 characters, a name or an `id` that two rooms share, and a device that a room lists twice get 400 and change nothing |
| `GET /api/share` | `{ "fixed", "shared", "url", "control", "local" }`: `fixed` when `--listen` opens the network already, `url` while sharing is on, `control` when the caller can switch sharing, `local` when the caller is on this machine |
| `POST /api/share` | `{ "shared": true \| false }` opens or closes the network, keeps the choice in `studio.json`, and answers as `GET`. A client from the network gets 403, a server started with a network `--listen` 409 |
| `GET /api/settings` | `{ "theme", "language", "advanced", "server" }`: `theme` is `dark`, `light` or `auto`, `language` a code or `null`. `server` is `{ "version", "sdk" }`, the version of `govee-server` and of the `govee-toolkit` crate in it, and a client can read it only |
| `PUT /api/settings` | the keys to change, such as `{ "theme": "light" }`, and the answer of `GET`. A value that is not valid gets 400 and `{ "error" }` |
| `GET /api/backup` | one JSON file to download: `{ "format": "govee-toolkit-backup", "server", "studio", "config", "credentials", "rooms" }`. `server` and `studio` are the two parts of `studio.json`, each with its `version` (see [Compatibility](#compatibility)). The answer names no file: Studio names it `govee-toolkit-studio-<date>-<time>.json`. `config` is the text of `config.yaml` or `null`, `credentials` holds `GOVEE_API_KEY` or is `null`, `rooms` is the content of `rooms.json`. A client from the network gets 403 |
| `POST /api/backup` | that file. The server checks all of it, keeps a `.import.bak` copy of each file, replaces the files and starts the SDK again on the next request. A file that is not valid gets 400 and changes nothing. A client from the network gets 403 |
| `POST /api/backup/restore` | puts back the files from the `.import.bak` copies of the last import, then deletes the copies. With no copy, 409. A client from the network gets 403 |
| `POST /api/reset` | `{ "prefs": true, "devices": true }`, one key or both. `prefs` puts the settings of `GET /api/settings` back to the defaults. `devices` removes the `devices:` key of `config.yaml` and every room, and starts the SDK again on the next request. The other keys stay, without their comments. The API key and the sharing switch stay. With neither key, 400. A client from the network gets 403 |
| any other path | a file of the web UI: the copy in the binary, or the directory that `--web` names |

Not affiliated with Govee.
