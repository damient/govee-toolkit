import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { SITE_URL, base, repoUrl } from "./config.ts";
import { fill } from "./html.ts";
import { ENGLISH, type Locale, type Sitemap, type T, alternates, homeRedirect, localizeLinks, prefix, strings, switcher } from "./locales.ts";
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

/** A page before its language is attached. */
export type Draft = Omit<Page, "locale">;

/** What every page of one language shares. */
export interface Frame {
  /** `src/layout.html`, with its strings in the language. */
  layout: string;
  css: string;
  docsHome: string;
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
  page: Page,
  frame: Frame,
  { routes, site }: { routes: Sitemap; site: Sitemap },
): Promise<void> {
  const t = strings(page.locale);
  const at = address(page);
  const filled = fill(frame.layout, {
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
    nav: topNav(page.nav, t),
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

// Docs pages light the Developers link: the guides and the reference serve the SDK.
function topNav(current: string, t: T): string {
  const here = current === "docs" ? "developers" : current;
  const link = (url: string, key: string, label: string) => {
    const on = key === here ? ' aria-current="page"' : "";
    return `<a href="${base}${url}"${on}>${label}</a>`;
  };
  return [
    link("studio/", "studio", "Studio"),
    link("dmx/", "dmx", "DMX"),
    link("sdk/", "developers", t("nav.developers")),
    link("devices/", "devices", t("nav.lights")),
    link("faq/", "faq", t("nav.help")),
  ].join("\n        ");
}
