// The trail over a page title. One trail feeds both the crumbs a reader sees
// and the JSON-LD `seo.breadcrumb()` writes, so the two cannot disagree.

import { escapeAttr, escapeHtml } from "./html.mjs";

/**
 * The crumbs of one page, as `[label, url]` pairs. The last pair is the page
 * itself, so it is drawn as text. Home is left out: the top bar carries it.
 */
export function crumbs(trail) {
  const items = trail.map(([label, url], at) =>
    at === trail.length - 1
      ? `<span>${escapeHtml(label)}</span>`
      : `<a href="{{base}}${escapeAttr(url)}">${escapeHtml(label)}</a>`,
  );
  return `<nav class="crumbs" aria-label="Breadcrumb">
      ${items.join(' <span aria-hidden="true">/</span> ')}
    </nav>`;
}
