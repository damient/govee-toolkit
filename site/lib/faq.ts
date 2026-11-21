// Each `##` heading of `content/faq.md` is a question, and the menu names them.

import { readPage, sections } from "./content.ts";
import { menus } from "./docs.ts";
import { escapeAttr, escapeHtml } from "./html.ts";
import { type Locale, prefix, strings } from "./locales.ts";
import { breadcrumb, faqData } from "./seo.ts";

/** The FAQ of a language, or `null` where the language does not carry it. */
export async function faqPage(locale: Locale) {
  const doc = await readPage("faq.md", locale);
  if (doc === null) return null;
  const t = strings(locale);
  const links = doc.headings
    .map((h) => `<li><a href="#${escapeAttr(h.id)}">${escapeHtml(h.menu ?? h.text)}</a></li>`)
    .join("\n          ");
  return {
    url: "faq/",
    nav: "faq",
    title: doc.title,
    description: doc.description,
    klass: "is-faq",
    jsonld: [breadcrumb([[t("nav.faq"), "faq/"]], t, prefix(locale)), faqData(sections(doc))],
    body: `<div class="doc-grid">
      ${menus({ here: t("faq.questions"), label: t("faq.questions"), select: links, side: links, spy: true })}
      <article class="prose">
        <h1>${escapeHtml(doc.title)}</h1>
        <p class="lede">${escapeHtml(t("faq.lede"))}</p>
        ${doc.html}
      </article>
    </div>`,
  };
}
