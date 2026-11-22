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

`npm run dev` is the same page with a reload on every change in `src/`:

- A stylesheet change replaces the stylesheet in the page, without a reload.
- A change in `src/locales/` loads the text again, without a reload.
- Any other change rebuilds `dist/` and reloads the page.

The dev server listens on `PORT` (4080). It starts `govee-server` on
`API_PORT` (4081) and passes every request to it.

The fonts and the icon come from `site/src/assets` at build time.

## UI

- A splash screen stays until the server answers `/api/hello`. If the server
  does not answer, the splash stays and gives the reason.
- The shell has a rail, with the logo above the navigation, and one view per
  space: Home, Lights, Library (presets and scenes), Live (pads), Stage
  (rooms in 3D and the scene timeline) and Settings. Each view is empty.
- On a wide screen, the button next to the logo sets the rail to icons
  only, and back. On a tablet, the rail always shows icons only. On a
  phone, the logo goes and the navigation is a tab bar; Stage and Settings
  go to "More".
- Settings sets the theme and the language. The theme is dark by default;
  "Auto" follows the system. This browser keeps both choices
  (`localStorage`).
- The text of the UI is in `src/locales/<code>.json`. `en.json` is the
  reference: it gives the keys, and a key that another file does not have
  shows in English. A change of language shows the splash while the new
  file loads. English is the only language for now.

A view gets its content when the content works against the server, `lan`
first.

## Engines

The desktop app shows this UI in the webview of the system: WebView2
(Chromium) on Windows, WebKit on macOS and WebKitGTK on Linux. Use only the
CSS and JavaScript that all three support. Test in Safari as well as in a
Chromium browser: Safari is the nearest engine to WebKitGTK.
