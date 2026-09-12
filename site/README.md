# Website

The site published at <https://damient.github.io/govee-toolkit/>.

Plain HTML, CSS and JavaScript. No framework. A build script assembles the
pages so that the header, the footer and the device tables exist once.

## Layout

| Path | What it holds |
| ---- | ------------- |
| `src/layout.html` | The shell of every page. |
| `src/pages/*.html` | The hand-written pages: home and devices. |
| `content/reference.json` | The reference page: one entry per command, with an example in each language. |
| `src/assets/` | CSS, JavaScript and the icon. Copied as they are. |
| `content/docs/*.md` | The documentation pages. One file, one page. |
| `public/` | Anything that must land at the root of the site, such as `CNAME`. |
| `build.mjs` | The build. |
| `dist/` | The output. It is not committed. |

## Build

The devices page reads `../dist/catalog.json`, which `xtask` generates from
`devices/*.yaml`. Generate it first:

```bash
cargo run -p xtask -- catalog
npm install
npm run build
```

Then `npm run dev` to work on it:

```bash
npm run dev     # http://localhost:8787
```

It builds with the base path at `/`, serves `dist/`, and rebuilds on every
change under `src/` and `content/`. Reload the page to see the change. `PORT`
selects another port. `npm run serve` serves without the watch.

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

The theme follows the system setting. The button switches away from it for as
long as the reader stays on the page, and nothing is stored: the next page
starts from the system setting again.

Under 820 px the links fold into a panel that opens from a button on the
right, and the theme control moves into that panel. Every rule of the folded
state is scoped to `[data-menu]`, an attribute that JavaScript sets: with
JavaScript blocked the links stay on the page.

## The base path

A project page lives under `/govee-toolkit/`. Every link passes through
`{{base}}` for that reason. A custom domain makes the base `/`: put the domain
in `public/CNAME` and set `BASE=/` in the workflow.

## Rules the site follows

- Nothing about a device is written here. The devices page is generated from
  the device files, and a fact the YAML does not carry does not reach the site.
- A mode that nobody probed shows `?`, never `none`.
- The site states what works today. Planned work is marked as planned.
- No measured number is invented. A latency or a zone count reaches the site
  from a device file or it does not reach it.
