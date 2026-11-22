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
  (rooms in 3D and the scene timeline) and Settings. Lights, Library, Live
  and Stage show a "Coming soon" card that says what the view will do
  (`soon.ts`).
- Each view has a path: `/`, `/lights`, `/setup`. `router.ts` routes the
  links with the history of the browser, so the back button and a reload
  keep the view. `govee-server` answers `index.html` for a path with no dot
  outside `/api`. The files of the build have absolute paths for this
  reason.
- Home shows a welcome card, centered in the view, that opens the setup.
  `/setup` has no entry in the navigation. Its first step runs a `lan`
  scan (`/api/devices`) and shows one card per light that answered
  (`light.ts`): the icon of its family, and its name or its SKU. A card
  selects the light, renames it in `config.yaml` and identifies it (the
  walk of `govee identify`: green for some seconds, then off). A scan
  that finds no light shows how to make a light answer. On a phone, the
  steps show as "1 / 3". The other steps are not built yet.
- The shared parts of a view:
  - `title.ts`: the head of a view. It holds an optional back link, the
    title, and content on the right.
  - `loading.ts`: the three dots of the logo in a wave, in three sizes,
    with an optional text on the right. `block` puts them in a card in the
    center of the free space, for a view that waits for its content.
  - `notice.ts`: a centered card with an icon, a title, a text and
    actions. The welcome card, the "Coming soon" card and an empty result
    use it.
  - `help.ts`: a "?" badge with a tooltip, on hover, on focus and on a
    tap. A view puts its hints there, not in a paragraph.
  - `family.ts`: the icon of a device family. `build.ts` reads the icons
    from `site/src/icons/family/`, so the site and Studio share one copy.
- A view is full width. A view with `width: "narrow"` in `nav.ts`
  (Settings) centers its content at `--page-narrow` (720 px).
- On a wide screen, the button next to the logo sets the rail to icons
  only, and back. On a tablet, the rail always shows icons only. On a
  phone, the logo goes and the navigation is a tab bar. Settings goes to
  "More".
- A view can need a larger screen (`min` in `nav.ts`): Stage needs a
  tablet, Mobile needs a computer. A smaller screen leaves the view out of
  the navigation, and the view shows a note in place of its content.
- Mobile, on a wide screen only, shares Studio on the network and shows its
  address as a QR code for a phone or a tablet. Only the computer of the
  server can switch sharing, and the server keeps the choice.
- On a phone, "Add to Home Screen" opens Studio without the browser bars
  (`manifest.webmanifest`).
- Settings shows the versions of Studio, of `govee-server` and of the SDK
  below its rows. `build.ts` reads the version of Studio from `package.json`.
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
- Settings > Reset opens a modal with a checkbox for the preferences, one
  for the device list, and All, which checks both. A second modal asks for
  a confirmation. The page reloads after the reset. Only the computer of
  the server can reset.
- `modal.ts` shows a modal dialog with a title, an optional text, an
  optional form and buttons, and answers the button that the user picks.
  The caller reads the form after the answer. `ready` keeps the primary
  button disabled until the form holds a valid choice.
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
