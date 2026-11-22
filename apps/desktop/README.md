# Desktop app

Studio in a window, on Windows and macOS, built with Tauri 2. On Linux, run
`govee-server` and open Studio in a browser: see
[`govee-toolkit-server`](../../packages/rust/crates/server/README.md#linux).

- The web UI is the build of [`../studio/`](../studio/).
- The server is `govee-toolkit-server`, in the same process. It listens on a
  free loopback port and serves the web UI. The window opens the address of
  the server.
- A release build serves the UI that the server crate compiles in. A debug
  build reads `../studio/dist`.

## Requirements

- Rust and Node 26.
- Windows: the WebView2 runtime, which Windows 11 includes.

## Run

```sh
npm install
npm run dev
```

`dev` builds the web UI once, then opens the window. After a change to
`../studio/src`, run `npm run build` in `../studio` and reload the
window: View > Reload, or Cmd+R on macOS.

`npm run build` makes the installers of the current platform under
`src-tauri/target/release/bundle/`.

## Icons

`npm run icons` writes `src-tauri/icons/` from `icon.svg`. Keep the files
that `tauri.conf.json` lists and `icon.png`, and delete the mobile and Store
icons.

`icon.svg` follows the app icon grid of macOS: an 824 px body centered on a
1024 px canvas. Do not use the site favicon, which fills its canvas: the Dock
then shows it larger than the other apps.

## Planned

- Auto-discovery on launch
- Tray icon
