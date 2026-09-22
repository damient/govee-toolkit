# Website

The site published at <https://gvetk.com>.

Plain HTML, CSS and TypeScript. No framework. A build script assembles the
pages so that the header, the footer and the device tables exist once.

## Layout

| Path | What it holds |
| ---- | ------------- |
| `src/layout.html` | The shell of every page. |
| `src/pages/*.html` | The hand-written pages: home, devices, add a device and the LAN Control list. |
| `content/reference.json` | One entry per command, with an example in each language. It feeds the reference page and the "What you can send" section of every model page. |
| `src/assets/css/*.css` | The stylesheet. `site.css` imports the rest. |
| `src/assets/ts/*.ts` | The browser script. `site.ts` wires the rest to the page. |
| `src/assets/` | The fonts and the images. |
| `lib/*.ts` | The renderers the build calls. `config.ts` holds the address of the site, and `types.ts` the shape of the catalog and of the reference. |
| `content/docs/*.md` | The documentation pages. One file, one page. |
| `content/faq.md` | The FAQ page. Each `##` heading is a question, and `<!-- menu: Label -->` at its end names it in the menu on the left. The page carries the `FAQPage` JSON-LD block. |
| `public/` | Anything that must land at the root of the site, such as `CNAME`. |
| `build.ts` | The build. |
| `tools/og.py` | Draws the social preview image and the touch icon. It writes `tools/og.svg` and both PNGs. |
| `tools/banner.py` | Draws the README banner. It writes `tools/banner.svg` and `docs/assets/banner.png`. |
| `dist/` | The output. It is not committed. |

## Build

The build needs Node 26 or later. The devices page reads
`../dist/catalog.json`, which `xtask` generates from `devices/*.yaml`. The LAN
Control page reads `../docs/lan-supported-devices.json`, which is committed.
Generate it first:

```bash
npm run catalog
npm install
npm run build
```

Then `npm run dev` to work on it:

```bash
npm run dev     # http://localhost:8787
```

It serves `dist/` and rebuilds on every change under `src/` and `content/`,
and on a new `../dist/catalog.json`. An open page follows the rebuild: a
change to a stylesheet replaces the styles in place, and any other change
reloads the page. A change to `build.ts` or `lib/` restarts the process,
because Node keeps an imported module in memory. `PORT` selects another port.
`npm run serve` serves without the watch.

The reload script is added by the server when it sends a page. `dist/` stays
the site that ships.

## Write a documentation page

Add a Markdown file under `content/docs/`. The front matter carries the title,
the address and the position in the menu:

```markdown
---
title: Modes
slug: modes
order: 4
description: One sentence. It becomes the page description.
---
```

The menu, the "on this page" list and the anchors are generated. `{{base}}`
inside a page becomes the base path of the site, and `{{repo}}` the address of
the repository.

## A state badge in a heading

A heading can end with a badge, which the install page uses:

```markdown
## Command line <span class="state ok">Available</span>
```

The anchor and the "on this page" list take what stands before the HTML, so
the heading above is reachable at `#command-line` and reads `Command line` in
the menu.

## Add a reference entry

`content/reference.json` holds the groups, and each group holds the entries.
One entry carries a title, a summary, an optional detail and one example per
language:

```json
{
  "id": "power",
  "title": "power",
  "summary": "Turn the device on or off.",
  "examples": {
    "cli": "govee on DEVICE",
    "rust": "device.power(true).await?;",
    "python": "await device.power(True)",
    "node": "await device.power(true)"
  }
}
```

The menu, the anchors and the language tabs are generated. A language the
build marks as planned carries a dot and a note, so a reader never takes a
shape for a release. Leave a language out and its tab does not appear.

## One entry on a model page

A model page names the same entries under "What you can send". The entry
reaches a device file through `roles:`, and `action:` says how the model page
renders it:

```json
{
  "id": "brightness",
  "title": "brightness",
  "roles": ["brightness"],
  "args": { "level": "brightness" },
  "values": { "level": 40 },
  "action": { "order": 20, "title": "Set the brightness" },
  "examples": { "cli": "govee brightness DEVICE {level}" }
}
```

| Field | What it does |
| ----- | ------------ |
| `roles` | The device-file roles that serve the entry. A model page shows the entry where it serves one of them. An entry without `roles` stays on the reference page. |
| `action.order` | Where the block sits on a model page. |
| `action.title` | The heading a model page uses, which is a sentence and not a command name. |
| `action.summary` | One line under that heading. Optional. |
| `action.segments` | Adds the zone count and the native resolution of the unit. |
| `args` | A placeholder, and the argument role that bounds it. A model page lists the range and puts a value inside it in the example. |
| `values` | The value each placeholder takes where nothing bounds it. The reference page always takes these. |

An example fills every `{name}` from `values`, and a name that neither
`values` nor the model supplies is left alone: a Rust format string survives.

An `adds` block joins the example of the entry, and carries the languages it
applies to. Its `needs` names what the model must satisfy: a role it serves,
or `native_pixels`, which says the unit renders more LEDs than it has zones.
The reference page satisfies every name, so it shows every block.

```json
"adds": [
  { "needs": ["segment_color_masked"], "examples": { "cli": "govee segment DEVICE --zones 0,1,2 \"#ff3d00\"" } }
]
```

## The top bar

The links are `Home`, `Docs`, `Devices` and `FAQ`. The install page and the reference
page both sit inside the documentation, so the top bar marks `Docs` while a
reader is on either one.

The theme follows the system setting. The button switches away from it, and
the choice goes into `sessionStorage`: it follows the reader from page to page
and ends with the tab. The next visit starts from the system setting again.

Under 820 px the links fold into a panel that opens from a button on the
right, and the theme control moves into that panel. Every rule of the folded
state is scoped to `[data-menu]`, an attribute that JavaScript sets: with
JavaScript blocked the links stay on the page.

## What a machine reads

`build.ts` writes `robots.txt` and `sitemap.xml`, and every page carries a
canonical address, an Open Graph block and a JSON-LD block. The absolute form
comes from `SITE_URL` in `lib/config.ts`, and the domain is also in
`public/CNAME`. Change both together.

The 404 page carries `noindex` and stays out of the sitemap.

## The fonts and the images

The two faces are served from this origin, under `src/assets/fonts/`. They
carry the latin subset, which is the range the site writes in. IBM Plex is
under the SIL Open Font License and the text sits beside the files.

`tools/og.py` draws the social preview image and the touch icon. It writes
`tools/og.svg` from the fonts under `src/assets/fonts/`, as outlines, so the
card needs no font installed. It then writes both PNGs:

```bash
python3 tools/og.py
```

Edit `tools/og.py`, not `tools/og.svg`: the next run overwrites the SVG.

`tools/banner.py` draws the banner at the top of the repository README. It uses
the same helpers and writes `tools/banner.svg` and `docs/assets/banner.png`:

```bash
python3 tools/banner.py
```

## The stylesheet and the script

Both are written as several files and bundled into one.
`src/assets/css/site.css` imports the parts, in cascade order, and
`src/assets/ts/site.ts` imports the modules it wires to the page. `npm run
build` resolves each entry with esbuild and minifies the result.

The stylesheet is then inlined into every page: the whole site is one small
file, and an external one costs a round trip before the first paint. The
script stays a file — it is deferred, and a second page reads it from the
cache. Neither source directory is copied into `dist/`.

## Linters

```bash
npm run qa         # the catalog, the build, the four linters, the file length
```

`npm run qa` is `tools/qa-site.sh`, which mirrors
`.github/workflows/pages.yml` and prints a pass/fail summary. To run the
linters alone:

```bash
npm run build      # the HTML linter reads dist/
npm run lint       # all four
```

| Command | Tool | What it reads |
| ------- | ---- | ------------- |
| `npm run lint:types` | tsc | `build.ts` and `lib/`, then `src/assets/ts/` |
| `npm run lint:js` | oxlint, with the rules that read the types | `build.ts`, `lib/` and `src/assets/ts/` |
| `npm run lint:css` | stylelint | `src/assets/css/` |
| `npm run lint:html` | html-validate | `dist/`, after a build |

The HTML linter reads the output and not the sources: a page under `src/`
holds `{{title}}` and the rest, which is not valid HTML. It reads the
generated pages too, which is where a broken table or a missing label shows
up first.

Each configuration file carries the rules the repository turns off:
`.oxlintrc.json`, `.stylelintrc.json` and `.htmlvalidate.json`.

## TypeScript

Node runs `build.ts` and `lib/` as they are: it removes the types and checks
nothing. esbuild does the same for `src/assets/ts/`. `npm run lint:types` is
the one step that checks the types, and CI runs it through `npm run lint`.

Two `tsconfig.json` files hold the two targets. The one at the root reads the
Node types, and the one under `src/assets/ts/` reads the DOM. A browser module
cannot import `node:fs`, and the build cannot touch `document`.

Because Node removes the types without a compiler, a source file must follow
two rules. `erasableSyntaxOnly` enforces both:

- An import names the file with its `.ts` extension.
- Use no syntax that emits code: no `enum`, no `namespace`, and no parameter
  property in a constructor. A site source
file stays under 300 lines, which `tools/check-file-length.sh` enforces.

## Rules the site follows

- Nothing about a device is written here. The devices page is generated from
  the device files, and a fact the YAML does not carry does not reach the site.
  The LAN Control page is generated from `docs/lan-supported-devices.json`.
- A mode that nobody probed shows `?`, never `none`.
- The site states what works today. Planned work is marked as planned.
- The copy writes for a reader, in the positive form. A limit of a mode
  stays on the documentation page that owns it. See `AGENTS.md`.
- No measured number is invented. A latency or a zone count reaches the site
  from a device file or it does not reach it.
