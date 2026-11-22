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
- Mobile, on a wide screen only, shares Studio on the network and shows its
  address as a QR code for a phone or a tablet. Only the computer of the
  server can switch sharing, and the server keeps the choice.
- On a phone, "Add to Home Screen" opens Studio without the browser bars
  (`manifest.webmanifest`).
- Settings sets the theme and the language. The theme is dark by default;
  "Auto" follows the system. The server keeps both choices (`/api/settings`),
  so every browser and the desktop app show the same ones. `localStorage`
  keeps a copy of the theme, which the script in `index.html` applies
  before the first paint, and the state of the rail of this screen.
- Settings > Data exports one JSON file with the devices, the settings and
  the API key of the cloud mode when one is set, and imports it again. The
  file is `govee-toolkit-studio-<date>-<time>.json`, in local time. A
  modal asks before an import replaces the files. After an import, Restore
  shows on the same row and puts back the configuration from before it. The
  button goes when the user leaves Settings or reloads the page; the server
  keeps its copies.
  Only the computer of the server can export, import or restore.
- `modal.ts` shows a modal dialog with a title, a text and buttons, and
  answers the button that the user picks.
- An error shows as a sentence: the server does not answer, the server is
  older than Studio, or the reason that the server gives.
- The text of the UI is in `src/locales/<code>.json`. `en.json` is the
  reference: it gives the keys, and a key that another file does not have
  shows in English. A change of language shows the splash while the new
  file loads. English is the only language for now.

A view gets its content when the content works against the server, `lan`
first.

## Engines

The desktop app shows this UI in the webview of the system: WebView2
(Chromium) on Windows and WebKit on macOS. On Linux, and on a phone or a
tablet, the UI runs in a browser. Use only the CSS and JavaScript that
Chromium, WebKit and Firefox all support. Test in Safari as well as in a
Chromium browser: WebKit is the engine of the macOS app and of every browser
on iOS.
