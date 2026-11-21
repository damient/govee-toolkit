// Writes one page into the layout, in the language of the page.

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { SITE_URL, base, repoUrl } from "./config.ts";
import { fill } from "./html.ts";
import { ENGLISH, type Locale, type Sitemap, alternates, homeRedirect, localizeLinks, prefix, strings, switcher, translate } from "./locales.ts";
import { jsonLd } from "./seo.ts";

/** One page, ready for the layout. `url` is the path inside its language. */
export interface Page {
  locale: Locale;
  url: string;
  nav: string;
  title: string | null;
  description?: string;
  klass?: string;
  noindex?: boolean;
  jsonld?: Record<string, unknown>[];
  body: string;
}

/** What every page of one language shares. */
export interface Frame {
  css: string;
  docsHome: string;
  docNav: string;
  vars: Record<string, string>;
}

/** The path of a page from the site root. */
export const address = (page: Page): string => `${prefix(page.locale)}${page.url}`;

/**
 * Writes one page under `out`. `routes` holds every page of each language and
 * decides where a link goes. `site` holds the indexed pages, and decides which
 * `hreflang` links the page carries.
 */
export async function emit(
  out: string,
  layout: string,
  page: Page,
  frame: Frame,
  { routes, site }: { routes: Sitemap; site: Sitemap },
): Promise<void> {
  const t = strings(page.locale);
  const at = address(page);
  const filled = fill(translate(layout, t), {
    base,
    site: SITE_URL,
    repo: repoUrl,
    lang: page.locale.id,
    og_locale: page.locale.og,
    title: page.title !== null && page.title !== "" ? t("meta.title_page", { title: page.title }) : t("meta.title"),
    description: page.description ?? t("meta.description"),
    bodyclass: page.klass ?? "",
    canonical: `${SITE_URL}${base}${at}`,
    og_type: page.url === "" ? "website" : "article",
    robots: page.noindex === true
      ? '<meta name="robots" content="noindex, follow">'
      : '<meta name="robots" content="index, follow, max-image-preview:large">',
    css: frame.css,
    redirect: page.locale.id === ENGLISH.id && page.url === "" ? homeRedirect() : "",
    jsonld: jsonLd(page.jsonld),
    nav: topNav(page.nav, frame.docsHome, page.locale),
    docnav: frame.docNav,
    content: fill(page.body, frame.vars),
    client_strings: t.client(),
    year: String(new Date().getUTCFullYear()),
  });
  // The two menus come after the links are localized: they name each language.
  const html = localizeLinks(filled, page.locale, routes.get(page.locale.id) ?? new Set())
    .replace("{{alternates}}", page.noindex === true ? "" : alternates(page.url, site))
    .replace("{{langs}}", switcher(page.locale, page.url, routes, t));
  const file = page.url.endsWith(".html") ? join(out, at) : join(out, at, "index.html");
  await mkdir(dirname(file), { recursive: true });
  await writeFile(file, html);
}

function topNav(current: string, docsHome: string, locale: Locale): string {
  const t = strings(locale);
  const items = [
    { url: "", key: "home" },
    { url: docsHome, key: "docs" },
    { url: "devices/", key: "devices" },
    { url: "faq/", key: "faq" },
  ];
  return items
    .map(({ url, key }) => {
      const on = key === current ? ' aria-current="page"' : "";
      return `<a href="${base}${url}"${on}>${t(`nav.${key}`)}</a>`;
    })
    .join("\n        ");
}
