// What a machine reads: the JSON-LD blocks, `robots.txt` and the sitemap.

import { DESCRIPTION, SITE_URL, base, repoUrl } from "./config.mjs";
import { escapeHtml } from "./html.mjs";

/** The JSON-LD blocks of one page, as script tags. Empty when it has none. */
export function jsonLd(blocks) {
  if (!blocks?.length) return "";
  return blocks
    .map((block) => `<script type="application/ld+json">${JSON.stringify(block)}</script>`)
    .join("\n");
}

/** The two blocks the home page carries: the site, and the source it documents. */
export function homeData() {
  return [
    {
      "@context": "https://schema.org",
      "@type": "WebSite",
      name: "Govee Toolkit",
      url: `${SITE_URL}${base}`,
      description: DESCRIPTION,
      inLanguage: "en",
    },
    {
      "@context": "https://schema.org",
      "@type": "SoftwareSourceCode",
      name: "govee-toolkit",
      description: DESCRIPTION,
      codeRepository: repoUrl,
      programmingLanguage: ["Rust", "Python", "JavaScript"],
      license: "https://opensource.org/licenses/MIT",
      url: `${SITE_URL}${base}`,
    },
  ];
}

/** The trail, as `[name, url]` pairs. Home is added in front of it. */
export function breadcrumb(trail) {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [["Home", ""], ...trail].map(([name, url], index) => ({
      "@type": "ListItem",
      position: index + 1,
      name,
      item: `${SITE_URL}${base}${url}`,
    })),
  };
}

/** One model page. `dateModified` is left out when nobody verified the model. */
export function deviceData(device, page) {
  return {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: page.title,
    description: page.description,
    url: `${SITE_URL}${base}${page.url}`,
    inLanguage: "en",
    isPartOf: { "@type": "WebSite", name: "Govee Toolkit", url: `${SITE_URL}${base}` },
    ...(device.verified?.date ? { dateModified: device.verified.date } : {}),
  };
}

/** One documentation page that declares `faq` in its front matter. */
export function faqData(doc) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: doc.sections.map((section) => ({
      "@type": "Question",
      name: section.title,
      acceptedAnswer: { "@type": "Answer", text: section.answer },
    })),
  };
}

/** `robots.txt`, which points at the sitemap. */
export function robots() {
  return `User-agent: *\nAllow: /\n\nSitemap: ${SITE_URL}${base}sitemap.xml\n`;
}

// No `lastmod`: the build date is the date of the build and not the date the
// page changed, and a wrong one is worse than none.
/** The sitemap, from the canonical address of every page that is indexed. */
export function sitemapXml(sitemap) {
  const urls = sitemap
    .map((url) => `  <url><loc>${escapeHtml(url)}</loc></url>`)
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
}
