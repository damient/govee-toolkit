# govee-toolkit-server

The local HTTP server of the dashboard. The `govee-server` binary runs it on
its own, and the desktop app in `apps/desktop` runs it in its process.

It holds no protocol logic: every answer comes from `govee-toolkit`.

## Install

```sh
cargo install govee-toolkit-server
govee-server
```

Open <http://127.0.0.1:4080>. The binary carries the web UI. On Linux the
`ble` mode needs `libdbus-1-dev` to build.

## Run from the repository

`build.rs` compiles `apps/dashboard/dist` into the binary. Build the UI
first, or the binary serves the API alone:

```sh
npm --prefix apps/dashboard install
npm --prefix apps/dashboard run build
cargo run -p govee-toolkit-server
```

`--web <directory>` serves the UI from a directory and not from the binary.
`npm --prefix apps/dashboard run serve` uses it on `dist/`, so a rebuild of
the UI shows on a reload.

`--listen` sets the address. The default is `127.0.0.1:4080`, the loopback
alone. `--listen 0.0.0.0:4080` opens the dashboard to the other devices on the
network. The API has no authentication, so anyone on that network can then
control the lights.

## Routes

| Route | Answer |
| ----- | ------ |
| `GET /api/hello` | `{ "name", "version", "skus" }`: the crate, its version and the number of SKUs in the embedded catalog |
| any other path | a file of the web UI: the copy in the binary, or the directory that `--web` names |

Unofficial and not affiliated with Govee.
