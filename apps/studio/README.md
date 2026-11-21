# Studio

The web UI of Studio, the app that controls the lights, in TypeScript with no framework. Two hosts serve
the same build:

- `govee-server` (`packages/rust/crates/server`), in a browser;
- the desktop app (`../desktop/`), in its webview.

The server that serves the page also serves the API, so the UI calls `/api`
on its own origin.

## Run

```sh
npm install
npm run serve
```

`serve` builds `dist/` and starts `govee-server` on <http://127.0.0.1:4080>.
In a second terminal, `npm run dev` rebuilds `dist/` on every change. Reload
the page to see it.

The fonts and the icon come from `site/src/assets` at build time.

## Planned UI

- Detected device list with a state badge per mode
- Per device: power toggle, brightness slider, color picker, effects
  dropdown (including undocumented commands)
- Bottom log: every command sent, with timestamp and measured latency
- **Raw payload field**: send a custom JSON command straight to a device, to
  try a discovery before you formalize it in `devices/*.yaml`

## Engines

The desktop app shows this UI in the webview of the system: WebView2
(Chromium) on Windows, WebKit on macOS and WebKitGTK on Linux. Use only the
CSS and JavaScript that all three support. Test in Safari as well as in a
Chromium browser: Safari is the nearest engine to WebKitGTK.
