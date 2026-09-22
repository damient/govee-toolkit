// The FAQ page: one Markdown file, `content/faq.md`, where each `##` heading is
// a question. It carries the `FAQPage` block. It takes the frame of a
// documentation page, and its menu names the questions in place of the pages,
// since it speaks to a reader who writes no code.

import { readPage } from "./content.ts";
import { escapeAttr, escapeHtml } from "./html.ts";
import { breadcrumb, faqData } from "./seo.ts";
import type { Crumb } from "./types.ts";

const CHEVRON = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9.5 6 6 6-6"/></svg>`;

const TRAIL: Crumb[] = [["FAQ", "faq/"]];

/** The FAQ page, ready for the layout. */
export async function faqPage() {
  const doc = await readPage("faq.md");
  const links = doc.headings
    .map((h) => `<li><a href="#${escapeAttr(h.id)}">${escapeHtml(h.menu ?? h.text)}</a></li>`)
    .join("\n          ");
  return {
    url: "faq/",
    nav: "faq",
    title: doc.title,
    description: doc.description,
    klass: "is-faq",
    jsonld: [breadcrumb(TRAIL), faqData(doc)],
    body: `<div class="doc-grid">
      <details class="doc-select" data-doc-select>
        <summary>
          <span class="doc-select-here">Questions</span>
          ${CHEVRON}
        </summary>
        <ul>
          ${links}
        </ul>
      </details>
      <nav class="doc-nav" aria-label="Questions" data-spy>
        <p class="eyebrow">Questions</p>
        <ul>
          ${links}
        </ul>
      </nav>
      <article class="prose">
        <h1>${escapeHtml(doc.title)}</h1>
        <p class="lede">The short answers, for anyone who wants a light to work.</p>
        ${doc.html}
      </article>
    </div>`,
  };
}
