# Changelog

Changes to `govee-toolkit-server`, the crate of the `govee-server` binary in
`packages/rust/crates/server`. It releases under `server-vX.Y.Z`. The policy
is [`../../../../docs/versioning.md`](../../../../docs/versioning.md).

### Added

- `POST /api/apply` takes `scene`: a name or a code of the scene library
  that `govee scenes fetch <SKU>` caches for each model.
- `POST /api/apply` takes `draw`: one color per LED, stored on the device
  through its `role: draw` entry. `features.draw` of
  `GET /api/devices/{id}/status` gives the LED count, and the grid
  `[columns, rows]` that `measurements.segment_grid` records, or `null`.
- `GET /api/devices/{id}/status` adds `drawing`: the last `draw` that the
  server sent to the device, kept in memory. A solid color, a white, a
  segment or a scene that the device takes sets it back to `null`.

### Changed

- `GET /api/devices/{id}/status` answers 200 and
  `{ "online": false, "reason", "code" }` for a device that does not answer,
  in place of 502. A device that answers adds `"online": true`. Studio shows
  the light offline, and the browser logs no error.
- An SDK error gets the HTTP status of its `code`, and the body adds
  `code`: 503 for a device that does not answer, 404, 400, 409, 422, 429,
  502 only for a refusal from the device or the API, and 500 for the rest.
  The README lists the table under "Errors". `POST /api/devices/{id}/identify`
  answers 503 in place of 502 when the device does not answer.
- The server drives every light over `lan` alone. `GET /api/devices` leaves
  out an entry of `config.yaml` that does not enable `lan`, such as a `ble`
  entry that the CLI wrote. `GET /api/devices/{id}/status` and
  `POST /api/apply` use `lan`, whatever other mode the entry enables.

### Fixed

- `features.segments` gives the zone count on `lan` for the H6022 and the
  H61A0. `segment` paints there by mask and arms no channel.

## [0.1.0] — 2026-11-23

### Added

- `govee-server` serves Studio: the API under `/api`, and the web UI.
  The binary carries the build of `apps/studio`, and `--web <directory>`
  serves another copy. It listens on `127.0.0.1:4080` by default.
  A path outside `/api` with no dot in its last segment is a page of
  Studio: the server answers `index.html` for it, so a reload of `/settings`
  keeps the page.
- `govee-server` writes a warning to stderr at startup while the deprecated
  configuration folder `~/.config/govee-toolkit` is in use or ignored, on
  macOS and Windows.
- `GET /api/devices` answers the devices of `config.yaml` at once, with no
  scan. Each device carries its SKU, its name and the `family` of its
  device file.
- `GET /api/scan` runs a `lan` scan and answers the devices that replied.
  Each device carries the `family` of its device file, the name that
  `config.yaml` gives it now, and `configured`. The SDK starts on the
  first call of either route and reads the configuration file.
- `POST /api/devices/{id}/identify` runs the walk of `govee identify` over
  `lan` for one device: off, green at full brightness for some seconds,
  then off. The call answers when the walk ends.
- `GET /api/devices/{id}/status` asks one device for its state over the
  modes that `config.yaml` enables, and adds `brightness_range`, the
  bounds of brightness that the device file gives for the mode that
  answered, and `features`: whether that mode takes a color, the bounds
  of its white temperature, how many
  zones it paints, and whether it takes the gradient. The zones come from
  the entry `role: segment_color_masked`, and a mode that arms a segment
  channel gets none. The first call for a device that the SDK does not
  know scans until the device answers. A device that does not answer gets
  502.
- `POST /api/apply` changes a list of devices in one call: power,
  brightness, one color, a white temperature, one color over a list of zones, and the gradient.
  Power on goes first and power off last. A level out of the bounds is an
  error, never a clamp. The answer gives one outcome per device. When a
  device fails, the others go ahead.
- The Studio settings keep `favorites`, a list of at most 32 colors.
- `PUT /api/devices` writes the device entries of `config.yaml` in one
  go: `keep` adds or renames, `remove` deletes. An entry with no SKU gets
  the SKU of the request, and a new entry also gets `modes: [lan]`. A name is 64 characters at most. The other keys of
  the file stay, but its comments do not survive the write. A request that
  changes no entry writes nothing.
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
  change them. The answer adds `server`, the version of the server and of
  `govee-toolkit`, which a client can read only.
- `GET` and `PUT /api/rooms` read and replace the rooms of the lights, in
  `rooms.json`. A room can be empty, and a device can be in more than one
  room. A device that `PUT /api/devices` removes leaves its rooms.
- `GET /api/backup` exports `studio.json`, `rooms.json`, `config.yaml` and
  the API key in one JSON file. Studio names the file with the local date and time.
- `POST /api/backup` imports that file. It checks the whole file, keeps a
  `.import.bak` copy of each file, then replaces them. The API key goes to
  `credentials.env` with the permissions `0600`. An import accepts a version
  of the Studio settings that the table of `src/compat.rs` holds, and tells
  the user to update `govee-server` for a newer one.
- `POST /api/backup/restore` puts back the files from before the last
  import. `GET /api/backup/restore` says whether there is an import to undo.
- `POST /api/reset` puts the Studio settings back to the defaults, removes
  the `devices:` key of `config.yaml` and the rooms, or both.
- Only a loopback client can export, import, restore or reset.
- `Options::storage` and `Storage` choose the folder of these files.
- A write to `config.yaml` (an import, a restore, a reset or `PUT
  /api/devices`) reaches the SDK through `Govee::reconfigure`: the sockets
  and the known devices stay. A change to `catalog`, `lan`, `cloud` or the
  API key starts the SDK again on the next request.
- The server writes its files on the blocking pool of Tokio, away from the
  threads that send to the devices.
- `rooms.json` keeps each device ID in upper case, as the SDK gives it.
- The web UI in the binary carries an `ETag` per file and
  `Cache-Control: no-cache`. A file that did not change gets 304.
- `serve` takes a `std::net::TcpListener`, and `WEB_DIR` names the folder
  that the build embeds. A build with no web UI prints a warning.
