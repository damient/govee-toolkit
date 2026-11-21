// What a machine reads: the JSON-LD blocks, `robots.txt` and the sitemap.

import { SITE_URL, base, repoUrl } from "./config.ts";
import { escapeAttr, escapeHtml, filled } from "./html.ts";
import { ENGLISH, LOCALES, type Locale, type Sitemap, absolute, carriers, strings } from "./locales.ts";
import type { Crumb, Device } from "./types.ts";
import type { Section } from "./content.ts";

/** One JSON-LD object. */
type LinkedData = Record<string, unknown>;

/** The JSON-LD blocks of one page, as script tags. Empty when it has none. */
export function jsonLd(blocks: LinkedData[] | undefined): string {
  if (!blocks || blocks.length === 0) return "";
  return blocks
    .map((block) => `<script type="application/ld+json">${JSON.stringify(block)}</script>`)
    .join("\n");
}

/** The two blocks the home page carries: the site, and the source it documents. */
export function homeData(locale: Locale): LinkedData[] {
  const t = strings(locale);
  return [
    {
      "@context": "https://schema.org",
      "@type": "WebSite",
      name: "Govee Toolkit",
      url: absolute(locale, ""),
      description: t("meta.description"),
      inLanguage: locale.id,
    },
    {
      "@context": "https://schema.org",
      "@type": "SoftwareSourceCode",
      name: "govee-toolkit",
      description: t("meta.description"),
      codeRepository: repoUrl,
      programmingLanguage: ["Rust", "Python", "JavaScript"],
      license: "https://opensource.org/licenses/MIT",
      url: `${SITE_URL}${base}`,
    },
  ];
}

/** The trail, as `[name, url]` pairs inside `locale`. Home is added in front of it. */
export function breadcrumb(trail: Crumb[], locale: Locale): LinkedData {
  const t = strings(locale);
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [[t("nav.home"), ""] as Crumb, ...trail].map(([name, url], index) => ({
      "@type": "ListItem",
      position: index + 1,
      name,
      item: absolute(locale, url),
    })),
  };
}

/**
 * A page of `locale` that lists `items`, as `[name, url]` pairs. The items
 * point at the English pages: a model page in another language is not indexed.
 */
export function collectionData(name: string, locale: Locale, url: string, items: Crumb[]): LinkedData {
  return {
    "@context": "https://schema.org",
    "@type": "CollectionPage",
    name,
    url: absolute(locale, url),
    mainEntity: {
      "@type": "ItemList",
      numberOfItems: items.length,
      itemListElement: items.map(([item, at], index) => ({
        "@type": "ListItem",
        position: index + 1,
        name: item,
        url: absolute(ENGLISH, at),
      })),
    },
  };
}

/** One model page. `dateModified` is left out when nobody verified the model. */
export function deviceData(
  device: Device,
  page: { title: string; description: string; url: string },
  locale: Locale,
): LinkedData {
  return {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: page.title,
    description: page.description,
    url: absolute(locale, page.url),
    inLanguage: locale.id,
    isPartOf: { "@type": "WebSite", name: "Govee Toolkit", url: `${SITE_URL}${base}` },
    ...(device.category === undefined ? {} : { articleSection: device.category }),
    ...(filled(device.verified?.date) ? { dateModified: device.verified.date } : {}),
  };
}

/** The FAQ page: one question per `##` heading. */
export function faqData(sections: Section[]): LinkedData {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: sections.map((section) => ({
      "@type": "Question",
      name: section.title,
      acceptedAnswer: { "@type": "Answer", text: section.answer },
    })),
  };
}

export function robots(): string {
  return `User-agent: *\nAllow: /\n\nSitemap: ${SITE_URL}${base}sitemap.xml\n`;
}

// No `lastmod`: the build date is the date of the build and not the date the
// page changed, and a wrong one is worse than none.
/** A page that sends the reader on to `to`, and that no index keeps. */
export function redirect(to: string): string {
  const url = `${SITE_URL}${base}${to}`;
  return `<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><title>Moved</title>`
    + `<meta name="robots" content="noindex"><link rel="canonical" href="${url}">`
    + `<meta http-equiv="refresh" content="0; url=${base}${to}"></head>`
    + `<body><p><a href="${base}${to}">${url}</a></p></body></html>\n`;
}

/**
 * The sitemap, from the path of every page that is indexed in each language.
 * A page that more than one language carries lists each of them.
 */
export function sitemapXml(site: Sitemap): string {
  const urls = LOCALES.flatMap((locale) => [...(site.get(locale.id) ?? [])].map((path) => {
    const links = carriers(path, site)
      .map((other) => `\n    <xhtml:link rel="alternate" hreflang="${other.id}" href="${escapeAttr(absolute(other, path))}"/>`)
      .join("");
    return `  <url><loc>${escapeHtml(absolute(locale, path))}</loc>${links}\n  </url>`;
  })).join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${urls}
</urlset>
`;
}
