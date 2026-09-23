// The FAQ page: one Markdown file, `content/faq.md`, where each `##` heading is
// a question. It carries the `FAQPage` block. It takes the frame of a
// documentation page, and its menu names the questions in place of the pages,
// since it speaks to a reader who writes no code.

import { readPage, sections } from "./content.ts";
import { menus } from "./docs.ts";
import { escapeAttr, escapeHtml } from "./html.ts";
import { breadcrumb, faqData } from "./seo.ts";
import type { Crumb } from "./types.ts";

const TRAIL: Crumb[] = [["FAQ", "faq/"]];

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
    jsonld: [breadcrumb(TRAIL), faqData(sections(doc))],
    body: `<div class="doc-grid">
      ${menus({ here: "Questions", label: "Questions", select: links, side: links, spy: true })}
      <article class="prose">
        <h1>${escapeHtml(doc.title)}</h1>
        <p class="lede">The short answers, for anyone who wants a light to work.</p>
        ${doc.html}
      </article>
    </div>`,
  };
}
