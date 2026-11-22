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
  Sharing is off at each start, and only a client on this machine can
  switch it. When sharing stops, the server refuses the network again.
- `--listen 0.0.0.0:4080` opens the server to the network for as long as it
  runs.

## Routes

| Route | Answer |
| ----- | ------ |
| `GET /api/hello` | `{ "name", "version", "skus" }`: the crate, its version and the number of SKUs in the embedded catalog |
| `GET /api/devices` | `{ "devices": [...] }`: the devices that answer a `lan` scan, in the record of `govee list --json`. The call takes one scan window, 2 s by default. An error answers 500 and `{ "error" }` |
| `GET /api/share` | `{ "fixed", "shared", "url", "control" }`: `fixed` when `--listen` opens the network already, `url` while sharing is on, `control` when the caller can switch sharing |
| `POST /api/share` | `{ "shared": true \| false }` opens or closes the network, and answers as `GET`. A client from the network gets 403, a server started with a network `--listen` 409 |
| any other path | a file of the web UI: the copy in the binary, or the directory that `--web` names |

Not affiliated with Govee.
