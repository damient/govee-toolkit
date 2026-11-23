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
- A change in `src/assets/locales/` loads the text again, without a reload.
- Any other change rebuilds `dist/` and reloads the page.

The dev server listens on `PORT` (4080). It starts `govee-server` on
`API_PORT` (4081) and passes every request to it.
The dev server stops when either port is in use, so only one dev server runs
at a time. It stops `govee-server` when it stops.

The fonts and the icon come from `site/src/assets` at build time.

## Layout

`src/` puts each file in a folder that gives its role:

| Folder | Content |
| ------ | ------- |
| `core/` | The modules that draw nothing: the API, the text, the preferences, the router, the theme. |
| `shell/` | The frame that `main.ts` mounts once: the rail, the navigation and the splash. |
| `components/` | The parts that two views or more use. Each part has a folder. |
| `pages/` | One folder per view. A part that only one view uses stays in the folder of that view. |
| `styles/` | The global stylesheets: the fonts, the tokens, the base rules, the logo and the controls that have no module. |
| `assets/` | The files that the page fetches: the text of the UI. |

A stylesheet sits beside the module that uses it, and the module imports it.
esbuild puts every imported stylesheet in `dist/app.css`, in the order of the
imports. `main.ts` imports `styles/` first. A page does not import another
page: move the shared code to `components/`, `shell/` or `core/`.

## UI

- A splash screen stays until the server answers `/api/settings`. If the
  server does not answer, the splash stays and gives the reason.
- The shell has a rail, with the logo above the navigation, and one view per
  space: Home, Library (presets and scenes), Live (pads), Stage (rooms in
  3D and the scene timeline) and Settings. Library, Live and Stage show a
  "Coming soon" card that says what the view will do (`soon.ts`).
- Each view has a path: `/`, `/settings`, `/setup`. `router.ts` routes the
  links with the history of the browser, so the back button and a reload
  keep the view. `govee-server` answers `index.html` for a path with no dot
  outside `/api`. The files of the build have absolute paths for this
  reason.
- Home shows the lights of `config.yaml` (`/api/devices`), one card per
  light (`card.ts`). Each card asks its light for its state
  (`/api/devices/{id}/status`). A dot on the icon is green when the light
  answers and gray when it does not. When the light answers, the card
  shows a power button and the color. When the device file gives a range,
  it also shows a brightness slider on the color bar, which sends the level
  while the knob moves. A light turned on is read again for its color. Every
  change goes through `/api/apply`. A click on a light that answers opens
  its panel (`panel/`), on the right of the page, or full screen on a
  phone (`components/sheet/`), with a back button. The head holds the
  power button. The body holds the colors (the current color when no
  swatch has it, the favorites, the base colors, and a chip that shows a
  color wheel), the white temperature, the brightness, the
  segments and the gradient. It shows each part that `features` of the
  status gives for the mode that answered. A change goes to the light at
  once, with one request in flight at most (`core/latest.ts`). A change to
  a light that is off turns it on in the same request. With
  segments selected, a color paints these segments. With none, it paints
  the whole light. The white swatch sends the white temperature where the
  mode has one, and shows its slider. The favorite button shows with the
  wheel, and for a favorite color. The favorites are a setting of the server, so every
  light and every client shows the same list. Studio keeps the colors of
  the segments and the gradient until a reload: the status of a light does
  not give them. "Save to library" waits for the Library view. The bar
  (`bar.ts`) holds the tabs of the rooms, a search on the name and the
  SKU, a power button, and a link to the setup. The power
  button turns off the lights shown that are on, then turns back on these
  lights only. Studio keeps that list until a reload, or until the user
  turns a light on from its card or its panel. With no light, Home shows a
  welcome card that opens the setup. Settings also opens the setup.
- `/setup` has no entry in the navigation. Its first step shows the
  lights of `config.yaml` at once (`/api/devices`), and runs a `lan` scan
  in the background (`/api/scan`). It shows one card per light
  (`light.ts`), in two groups: the new lights, and the lights of
  `config.yaml`. While the scan runs, a light of `config.yaml` shows a
  wait mark. Then a light that did not answer shows as offline. A card shows the
  icon of its family, and its name or its SKU. A card selects the light,
  renames it and identifies it (the walk of `govee identify`: green for
  some seconds, then off). A rename stays in the page until Continue
  (`find.ts`). Continue writes every entry in one `PUT /api/devices`:
  `config.yaml` keeps each selected light and loses each light of
  `config.yaml` that is not selected. Then it goes to the second step,
  `/setup/rooms` (`rooms.ts`). A scan that finds no light and
  a `config.yaml` with no device show how to make a light answer. On a
  phone, the steps show as "1 / 2". A done step links to its page.
- The second step reads the lights of `config.yaml` (`/api/devices`,
  with no scan) and the rooms (`/api/rooms`). It shows every light on the left and one card per room
  (`room.ts`). A room shows its number of lights in a badge, and a menu
  renames or deletes it. A light goes into a room with a drag (`drag.ts`), or with a
  click on the light and then on the room. Enter and Esc do the same from
  the keyboard. A drag from the light list adds the light, and a drag from
  a room moves it. A light can be in more than one room, and a room can be
  empty. The card at the end adds a room. Finish writes every room in one `PUT /api/rooms` and goes home.
  The drag uses the pointer events, so that a mouse and a touch use one
  path.
- The shared parts of a view:
  - `title.ts`: the head of a view. It holds an optional back link, the
    title, and content on the right.
  - `loading.ts`: the three dots of the logo in a wave, in three sizes,
    with an optional text on the right. `block` puts them in a card in the
    center of the free space, for a view that waits for its content.
  - `alert.ts`: a line in the flow of a view, with a tone: success,
    info, warning or error. The tone sets the color and the icon.
  - `notice.ts`: a centered card with an icon, a title, a text and
    actions. The welcome card, the "Coming soon" card and an empty result
    use it.
  - `help.ts`: a "?" badge with a tooltip, on hover, on focus and on a
    tap. A view puts its hints there, not in a paragraph.
  - `menu.ts`: a button with three dots that opens a list of actions.
    The arrow keys move in the list. Esc, a click elsewhere and a scroll
    close it.
  - `tooltip.ts`: a tooltip on hover and on focus of a control. An icon
    button names its action there, not in a `title`.
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
  below its rows. The server gives its two with the settings
  (`/api/settings`). `build.ts` reads the version of Studio from
  `package.json`.
- Settings sets the theme and the language. The theme is dark by default.
  "Auto" follows the system. The server keeps both choices (`/api/settings`),
  so every browser and the desktop app show the same ones. `localStorage`
  keeps a copy of the theme, which the script in `index.html` applies
  before the first paint, and the state of the rail of this screen.
- Settings > Data exports one JSON file with the devices, the settings and
  the API key of the cloud mode when one is set, and imports it again. The
  file is `govee-toolkit-studio-<date>-<time>.json`, in local time. A
  modal asks before an import replaces the files. While the server keeps the
  copies from before an import, Restore shows on the same row and puts back
  that configuration.
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
- The text of the UI is in `src/assets/locales/<code>.json`. `en.json` is the
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
