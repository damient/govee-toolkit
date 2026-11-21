# Desktop app

The dashboard in a window, on Windows, macOS and Linux, built with Tauri 2.

- The web UI is the build of [`../dashboard/`](../dashboard/).
- The server is `govee-toolkit-server`, in the same process. It listens on a
  free loopback port and serves the web UI. The window opens the address of
  the server.
- A release build serves the UI that the server crate compiles in. A debug
  build reads `../dashboard/dist`.

`src-tauri` is a Cargo project of its own, outside the workspace of
`packages/rust`, because on Linux it links to WebKitGTK.

## Requirements

- Rust and Node 26.
- Linux: `libwebkit2gtk-4.1-dev`, `libdbus-1-dev` and the other packages of
  the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
- Windows: the WebView2 runtime, which Windows 11 includes.

## Run

```sh
npm install
npm run dev
```

`dev` builds the web UI once, then opens the window. After a change to
`../dashboard/src`, run `npm run build` in `../dashboard` and reload the
window.

`npm run build` makes the installers of the current platform under
`src-tauri/target/release/bundle/`.

## macOS and Bluetooth

`src-tauri/Info.plist` declares the Bluetooth usage. Without it, macOS kills
the app the first time the `ble` mode opens the Bluetooth stack.

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
