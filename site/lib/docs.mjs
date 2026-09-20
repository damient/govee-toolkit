// The frame every documentation page shares. The Markdown pages and the
// reference page differ in what they put in the middle, and in nothing else.

import { escapeAttr, escapeHtml } from "./html.mjs";

/**
 * One documentation page. `nav` holds `{ url, title }` for the menu and
 * `current` names the url the reader is on. `toc` holds the entries of the
 * "On this page" menu, `body` the article, and `klass` a class the article
 * carries beside `prose`. `collapse` folds each group of that menu, for a
 * page whose menu is longer than a screen.
 */
export function docShell({ base, nav, current, toc, body, klass = "", collapse = false }) {
  return `<div class="doc-grid">
      ${docSelect(base, nav, current, toc, collapse)}
      ${sideNav(base, nav, current)}
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
function pager(base, nav, current) {
  const at = nav.findIndex((item) => item.url === current);
  const links = [
    pagerLink(base, nav[at - 1], "prev"),
    pagerLink(base, nav[at + 1], "next"),
  ].filter(Boolean);
  if (!links.length) return "";
  return `<nav class="pager" aria-label="Documentation pages">
          ${links.join("\n          ")}
        </nav>`;
}

function pagerLink(base, item, klass) {
  if (!item) return "";
  const prev = klass === "prev";
  const icon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${prev ? "m14 6-6 6 6 6" : "m10 6 6 6-6 6"}"/></svg>`;
  const text = `<span>${escapeHtml(item.title)}</span>`;
  return `<a class="${prev ? "btn ghost" : "btn"} pager-link ${klass}" rel="${klass}" href="${base}${item.url}">${prev ? icon + text : text + icon}</a>`;
}

// The two menus as one control, for a screen too narrow to carry a column on
// each side. The headings of the current page nest under it, so the "On this
// page" menu stays where a reader looks for it.
function docSelect(base, nav, current, toc, collapse) {
  const here = nav.find((item) => item.url === current);
  const items = nav
    .map((item) => {
      if (item.url !== current) return navItem(base, item, current);
      const inner = toc.length
        ? `\n            ${tocList(toc, "doc-select-toc", collapse && "toc-select")}`
        : "";
      return navItem(base, item, current, inner, "on");
    })
    .join("\n          ");
  return `<details class="doc-select" data-doc-select>
        <summary>
          <span class="doc-select-here">${escapeHtml(here?.title ?? "Contents")}</span>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9.5 6 6 6-6"/></svg>
        </summary>
        <ul>
          ${items}
        </ul>
      </details>`;
}

function sideNav(base, nav, current) {
  const links = nav.map((item) => navItem(base, item, current)).join("\n          ");
  return `<nav class="doc-nav" aria-label="Documentation">
        <p class="eyebrow">Documentation</p>
        <ul>
          ${links}
        </ul>
      </nav>`;
}

/**
 * One entry of a documentation menu. `inner` nests under the link and `klass`
 * goes on the item, so the two menus differ in those two and in nothing else.
 */
export function navItem(base, item, current, inner = "", klass = "") {
  const on = item.url === current ? ' aria-current="page"' : "";
  const cls = klass ? ` class="${klass}"` : "";
  return `<li${cls}><a href="${base}${item.url}"${on}>${escapeHtml(item.title)}</a>${inner}</li>`;
}

/**
 * The "On this page" menu. An entry carries `id` and `text`; `code` sets the
 * text in a monospace face, and `children` opens a second level under it.
 * `data-spy` is what marks the entry the reader is on — see `spy()` in
 * `site.js`.
 */
function pageToc(entries, collapse) {
  return `<aside class="doc-toc" aria-label="On this page" data-spy>
        <p class="eyebrow">On this page</p>
        ${tocList(entries, "sub", collapse && "toc-page")}
      </aside>`;
}

/** The chevron a folded group carries. `details[open]` turns it. */
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
function tocList(entries, klass, fold = "") {
  const items = entries.map((entry) => {
    const group = entry.children?.length;
    if (!group) return `<li>${tocLink(entry)}</li>`;
    const nested = tocList(entry.children, "");
    const body = fold
      ? `<details name="${escapeAttr(fold)}"><summary data-to="${escapeAttr(entry.id)}">${escapeHtml(entry.text)}${CHEVRON}</summary>${nested}</details>`
      : `${tocLink(entry)}${nested}`;
    return `<li class="sub-group">${body}</li>`;
  });
  return `<ul${klass ? ` class="${klass}"` : ""}>${items.join("")}</ul>`;
}

function tocLink(entry) {
  const label = escapeHtml(entry.text);
  return `<a href="#${escapeAttr(entry.id)}">${entry.code ? `<code>${label}</code>` : label}</a>`;
}
