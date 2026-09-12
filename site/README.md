# Website

The site published at <https://gvetk.com>.

Plain HTML, CSS and JavaScript. No framework. A build script assembles the
pages so that the header, the footer and the device tables exist once.

## Layout

| Path | What it holds |
| ---- | ------------- |
| `src/layout.html` | The shell of every page. |
| `src/pages/*.html` | The hand-written pages: home and devices. |
| `content/reference.json` | The reference page: one entry per command, with an example in each language. |
| `src/assets/css/*.css` | The stylesheet. `site.css` imports the rest. |
| `src/assets/js/*.js` | The browser script. `site.js` wires the rest to the page. |
| `src/assets/` | The fonts and the images. |
| `lib/*.mjs` | The renderers the build calls, and `config.mjs`, which holds the address of the site. |
| `content/docs/*.md` | The documentation pages. One file, one page. |
| `public/` | Anything that must land at the root of the site, such as `CNAME`. |
| `build.mjs` | The build. |
| `tools/og.py` | Draws the social preview image and the touch icon. It writes `tools/og.svg` and both PNGs. |
| `dist/` | The output. It is not committed. |

## Build

The devices page reads `../dist/catalog.json`, which `xtask` generates from
`devices/*.yaml`. Generate it first:

```bash
npm run catalog
npm install
npm run build
```

Then `npm run dev` to work on it:

```bash
npm run dev     # http://localhost:8787
```

It serves `dist/` and rebuilds on every change under `src/`, `content/` and
`lib/`. Reload the page to see the change. `PORT` selects another port.
`npm run serve` serves without the watch.

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
    "cli": "govee on living-room",
    "rust": "device.power(true).await?;",
    "python": "await device.power(True)",
    "node": "await device.power(true)"
  }
}
```

The menu, the anchors and the language tabs are generated. A language the
build marks as planned carries a dot and a note, so a reader never takes a
shape for a release. Leave a language out and its tab does not appear.

## The top bar

The links are `Home`, `Docs` and `Devices`. The install page and the reference
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

`build.mjs` writes `robots.txt` and `sitemap.xml`, and every page carries a
canonical address, an Open Graph block and a JSON-LD block. The absolute form
comes from `SITE_URL` in `lib/config.mjs`, and the domain is also in
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

## The stylesheet and the script

Both are written as several files and bundled into one.
`src/assets/css/site.css` imports the parts, in cascade order, and
`src/assets/js/site.js` imports the modules it wires to the page. `npm run
build` resolves each entry with esbuild and minifies the result.

The stylesheet is then inlined into every page: the whole site is one small
file, and an external one costs a round trip before the first paint. The
script stays a file — it is deferred, and a second page reads it from the
cache. Neither source directory is copied into `dist/`.

## Linters

```bash
npm run qa         # the catalog, the build, the three linters, the file length
```

`npm run qa` is `tools/qa-site.sh`, which mirrors
`.github/workflows/pages.yml` and prints a pass/fail summary. To run the
linters alone:

```bash
npm run build      # the HTML linter reads dist/
npm run lint       # all three
```

| Command | Tool | What it reads |
| ------- | ---- | ------------- |
| `npm run lint:js` | oxlint | `build.mjs`, `lib/` and `src/assets/js/` |
| `npm run lint:css` | stylelint | `src/assets/css/` |
| `npm run lint:html` | html-validate | `dist/`, after a build |

The HTML linter reads the output and not the sources: a page under `src/`
holds `{{title}}` and the rest, which is not valid HTML. It reads the
generated pages too, which is where a broken table or a missing label shows
up first.

Each configuration file carries the rules the repository turns off:
`.oxlintrc.json`, `.stylelintrc.json` and `.htmlvalidate.json`. A site source
file stays under 300 lines, which `tools/check-file-length.sh` enforces.

## Rules the site follows

- Nothing about a device is written here. The devices page is generated from
  the device files, and a fact the YAML does not carry does not reach the site.
- A mode that nobody probed shows `?`, never `none`.
- The site states what works today. Planned work is marked as planned.
- No measured number is invented. A latency or a zone count reaches the site
  from a device file or it does not reach it.
