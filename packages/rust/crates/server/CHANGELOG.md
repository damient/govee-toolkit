# Changelog

Changes to `govee-toolkit-server`, the crate of the `govee-server` binary in
`packages/rust/crates/server`. It releases under `server-vX.Y.Z`. The policy
is [`../../../../docs/versioning.md`](../../../../docs/versioning.md).

## [0.1.0] — Unreleased

### Added

- `govee-server` serves Studio: the API under `/api`, and the web UI.
  The binary carries the build of `apps/studio`, and `--web <directory>`
  serves another copy. It listens on `127.0.0.1:4080` by default.
- `GET /api/hello` answers the name and the version of `govee-toolkit`, and the
  number of SKUs in the embedded catalog.
- `GET /api/devices` runs a `lan` scan and answers the devices that replied.
  The SDK starts on the first call and reads the configuration file.
- `GET` and `POST /api/share` open the server to the network and close it
  again, with a second listener on the network address of this machine.
  Sharing is off by default, and only a loopback client can switch it.
  `studio.json` keeps the switch, and the server shares again at its next
  start. When the network is not up yet, it tries again every 5 s. While sharing is off, the server answers 403 to a client from the
  network. `local` in the answer says that the client is on this machine.
- `studio.json` in the configuration folder keeps the settings of the server
  and the settings of Studio, in a `server` and a `studio` part. Each part
  holds a `version` and `settings`. The server writes it to
  a temporary file and renames it. A file that does not parse stops the
  start.
- `GET` and `PUT /api/settings` read and change the theme, the language and the
  `advanced` switch of Studio. Every client that the server admits can
  change them.
- `GET /api/backup` exports `studio.json`, `config.yaml` and the API key in
  one JSON file. Studio names the file with the local date and time.
- `POST /api/backup` imports that file. It checks the whole file, keeps a
  `.import.bak` copy of each file, then replaces them. The API key goes to
  `credentials.env` with the permissions `0600`. An import accepts a version
  of the Studio settings that the table of `src/compat.rs` holds, and tells
  the user to update `govee-server` for a newer one.
- `POST /api/backup/restore` puts back the files from before the last
  import.
- Only a loopback client can export, import or restore.
- `Options::storage` and `Storage` choose the folder of these files.
- `serve` takes a `std::net::TcpListener`, and `WEB_DIR` names the folder
  that the build embeds. A build with no web UI prints a warning.
