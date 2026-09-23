// The frame every documentation page shares. The Markdown pages and the
// reference page differ in what they put in the middle, and in nothing else.

import { escapeAttr, escapeHtml } from "./html.ts";
import type { TocEntry } from "./types.ts";

type NavLink = { url: string; title: string };

interface Shell {
  base: string;
  nav: NavLink[];
  current: string;
  toc: TocEntry[];
  body: string;
  klass?: string;
  collapse?: boolean;
}

/**
 * One documentation page. `nav` holds `{ url, title }` for the menu and
 * `current` names the url the reader is on. `toc` holds the entries of the
 * "On this page" menu, `body` the article, and `klass` a class the article
 * carries beside `prose`. `collapse` folds each group of that menu, for a
 * page whose menu is longer than a screen.
 */
export function docShell({ base, nav, current, toc, body, klass = "", collapse = false }: Shell): string {
  return `<div class="doc-grid">
      ${menus({
        here: nav.find((item) => item.url === current)?.title ?? "Contents",
        label: "Documentation",
        select: selectItems(base, nav, current, toc, collapse),
        side: nav.map((item) => navItem(base, item, current)).join("\n          "),
      })}
      <article class="prose${klass ? ` ${klass}` : ""}">
        ${body}
        ${pager(base, nav, current)}
      </article>
      ${pageToc(toc, collapse)}
    </div>`;
}

/**
 * The two links that close a page: the page before it and the page after it,
 * in menu order. The first page carries no `Prev` and the last no `Next`.
 */
function pager(base: string, nav: NavLink[], current: string): string {
  const at = nav.findIndex((item) => item.url === current);
  const links = [
    pagerLink(base, nav[at - 1], "prev"),
    pagerLink(base, nav[at + 1], "next"),
  ].filter(Boolean);
  if (links.length === 0) return "";
  return `<nav class="pager" aria-label="Documentation pages">
          ${links.join("\n          ")}
        </nav>`;
}

function pagerLink(base: string, item: NavLink | undefined, klass: "prev" | "next"): string {
  if (!item) return "";
  const prev = klass === "prev";
  const icon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${prev ? "m14 6-6 6 6 6" : "m10 6 6 6-6 6"}"/></svg>`;
  const text = `<span>${escapeHtml(item.title)}</span>`;
  return `<a class="${prev ? "btn ghost" : "btn"} pager-link ${klass}" rel="${klass}" href="${base}${item.url}">${prev ? icon + text : text + icon}</a>`;
}

// The headings of the current page nest under its entry of the narrow menu,
// so the "On this page" menu stays where a reader looks for it.
function selectItems(base: string, nav: NavLink[], current: string, toc: TocEntry[], collapse: boolean): string {
  return nav
    .map((item) => {
      if (item.url !== current) return navItem(base, item, current);
      const inner = toc.length > 0
        ? `\n            ${tocList(toc, "doc-select-toc", collapse ? "toc-select" : "")}`
        : "";
      return navItem(base, item, current, inner, "on");
    })
    .join("\n          ");
}

interface Menus {
  here: string;
  label: string;
  select: string;
  side: string;
  spy?: boolean;
}

/**
 * The side menu, and the same entries as one control for a screen too narrow
 * to carry a column on each side. `here` names the narrow control, `select`
 * and `side` hold the `<li>` entries of each. `spy` marks the side entry the
 * reader is on, for a menu of in-page anchors.
 */
export function menus({ here, label, select, side, spy = false }: Menus): string {
  return `<details class="doc-select" data-doc-select>
        <summary>
          <span class="doc-select-here">${escapeHtml(here)}</span>
          ${CHEVRON}
        </summary>
        <ul>
          ${select}
        </ul>
      </details>
      <nav class="doc-nav" aria-label="${escapeAttr(label)}"${spy ? " data-spy" : ""}>
        <p class="eyebrow">${escapeHtml(label)}</p>
        <ul>
          ${side}
        </ul>
      </nav>`;
}

/**
 * One entry of a documentation menu. `inner` nests under the link and `klass`
 * goes on the item, so the two menus differ in those two and in nothing else.
 */
export function navItem(base: string, item: NavLink, current: string | null, inner = "", klass = ""): string {
  const on = item.url === current ? ' aria-current="page"' : "";
  const cls = klass ? ` class="${klass}"` : "";
  return `<li${cls}><a href="${base}${item.url}"${on}>${escapeHtml(item.title)}</a>${inner}</li>`;
}

/**
 * The "On this page" menu. An entry carries `id` and `text`; `code` sets the
 * text in a monospace face, and `children` opens a second level under it.
 * `data-spy` is what marks the entry the reader is on — see `spy()` in
 * `src/assets/ts/spy.ts`.
 */
function pageToc(entries: TocEntry[], collapse: boolean): string {
  return `<aside class="doc-toc" aria-label="On this page" data-spy>
        <p class="eyebrow">On this page</p>
        ${tocList(entries, "sub", collapse ? "toc-page" : "")}
      </aside>`;
}

/** The chevron a folded control carries. `details[open]` turns it. */
const CHEVRON = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9.5 6 6 6-6"/></svg>`;

/**
 * `fold` folds each group behind its title, closed, and names the accordion:
 * the groups of one menu share the name, so one of them is open at a time.
 * Each menu takes a name of its own, because a name is unique in the page and
 * the two menus carry the same entries.
 *
 * The title toggles the group, and `data-to` names the section it goes to:
 * `spy()` reads both, and opens the group the reader scrolls into.
 */
function tocList(entries: TocEntry[], klass: string, fold = ""): string {
  const items = entries.map((entry) => {
    if (!entry.children || entry.children.length === 0) return `<li>${tocLink(entry)}</li>`;
    const nested = tocList(entry.children, "");
    const body = fold
      ? `<details name="${escapeAttr(fold)}"><summary data-to="${escapeAttr(entry.id)}">${escapeHtml(entry.text)}${CHEVRON}</summary>${nested}</details>`
      : `${tocLink(entry)}${nested}`;
    return `<li class="sub-group">${body}</li>`;
  });
  return `<ul${klass ? ` class="${klass}"` : ""}>${items.join("")}</ul>`;
}

function tocLink(entry: TocEntry): string {
  const label = escapeHtml(entry.text);
  return `<a href="#${escapeAttr(entry.id)}">${entry.code === true ? `<code>${label}</code>` : label}</a>`;
}
