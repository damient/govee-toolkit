// The frame every documentation page shares: the menu on the left, the page
// in the middle, the "On this page" menu on the right. The Markdown pages and
// the reference page differ in what they put in the middle, and in nothing
// else.

import { escapeAttr, escapeHtml } from "./html.mjs";

/**
 * One documentation page. `nav` holds `{ url, title }` for the menu and
 * `current` names the url the reader is on. `toc` holds the entries of the
 * "On this page" menu, `body` the article, and `klass` a class the article
 * carries beside `prose`.
 */
export function docShell({ base, nav, current, toc, body, klass = "" }) {
  return `<div class="doc-grid">
      ${docSelect(base, nav, current, toc)}
      ${sideNav(base, nav, current)}
      <article class="prose${klass ? ` ${klass}` : ""}">
        ${body}
      </article>
      ${pageToc(toc)}
    </div>`;
}

// The two menus, as one control, for a screen too narrow to carry a column on
// each side. It reads as a select: the page you are on, and the pages you can
// go to. The headings of the page you are on nest under it, so the "On this
// page" menu stays where a reader looks for it instead of landing under the
// article.
function docSelect(base, nav, current, toc) {
  const here = nav.find((item) => item.url === current);
  const items = nav
    .map((item) => {
      if (item.url !== current) {
        return `<li><a href="${base}${item.url}">${escapeHtml(item.title)}</a></li>`;
      }
      const inner = toc.length ? `\n            ${tocList(toc, "doc-select-toc", 12)}` : "";
      return `<li class="on"><a href="${base}${item.url}" aria-current="page">${escapeHtml(item.title)}</a>${inner}</li>`;
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
  const links = nav
    .map((item) => {
      const on = item.url === current ? ' aria-current="page"' : "";
      return `<li><a href="${base}${item.url}"${on}>${escapeHtml(item.title)}</a></li>`;
    })
    .join("\n          ");
  return `<nav class="doc-nav" aria-label="Documentation">
        <p class="eyebrow">Documentation</p>
        <ul>
          ${links}
        </ul>
      </nav>`;
}

/**
 * The "On this page" menu. An entry carries `id` and `text`; `code` sets the
 * text in a monospace face, and `children` opens a second level under it.
 * `data-spy` is what marks the entry the reader is on — see `spy()` in
 * `site.js`.
 */
function pageToc(entries) {
  return `<aside class="doc-toc" aria-label="On this page" data-spy>
        <p class="eyebrow">On this page</p>
        ${tocList(entries, "sub", 8)}
      </aside>`;
}

function tocList(entries, klass, depth) {
  const pad = " ".repeat(depth + 2);
  const items = entries.map((entry) => {
    const group = entry.children?.length;
    const nested = group ? `\n${pad}${tocList(entry.children, "", depth + 2)}\n${pad}` : "";
    return `${pad}<li${group ? ' class="sub-group"' : ""}>${tocLink(entry)}${nested}</li>`;
  });
  return `<ul${klass ? ` class="${klass}"` : ""}>
${items.join("\n")}
${" ".repeat(depth)}</ul>`;
}

function tocLink(entry) {
  const label = escapeHtml(entry.text);
  return `<a href="#${escapeAttr(entry.id)}">${entry.code ? `<code>${label}</code>` : label}</a>`;
}
