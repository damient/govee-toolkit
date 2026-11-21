// The languages of the site. English sits at the root, and every other language
// sits under a prefix of its own. `content/i18n/<id>.json` holds the strings of
// a language, and `content/<id>/` holds its pages. `translations.ts`
// reports a string or a page that is missing or older than the English one.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { SITE_URL, base, root } from "./config.ts";
import { escapeAttr, escapeHtml } from "./html.ts";
import { isRecord } from "./json.ts";

export interface Locale {
  id: string;
  /** The name of the language, in that language. */
  name: string;
  /** The value of `og:locale`. */
  og: string;
}

export const ENGLISH: Locale = { id: "en", name: "English", og: "en_US" };

export const LOCALES: readonly Locale[] = [
  ENGLISH,
  { id: "fr", name: "Français", og: "fr_FR" },
  { id: "de", name: "Deutsch", og: "de_DE" },
  { id: "es", name: "Español", og: "es_ES" },
  { id: "it", name: "Italiano", og: "it_IT" },
];

/** Every language but English. */
export const TRANSLATIONS: readonly Locale[] = LOCALES.slice(1);

/** The path of a language under the site root: empty for English. */
export const prefix = (locale: Locale): string => (locale.id === ENGLISH.id ? "" : `${locale.id}/`);

/** The absolute URL of `path` in a language. */
export const absolute = (locale: Locale, path: string): string => `${SITE_URL}${base}${prefix(locale)}${path}`;

/** The directory that holds the Markdown pages and `reference.json` of a language. */
export const contentDir = (locale: Locale): string =>
  locale.id === ENGLISH.id ? join(root, "content") : join(root, "content", locale.id);

/**
 * The text of a key, with each `{name}` replaced from `vars`. A key that the
 * language lacks takes the English text. A key that English lacks throws.
 */
export interface T {
  (key: string, vars?: Record<string, string | number>): string;
  has(key: string): boolean;
  /** The text of `key`, or `fallback` where no language has the key. */
  or(key: string, fallback: string): string;
  /** The name of a category of Govee's LAN list. An untranslated name stays as it is. */
  category(name: string): string;
  /** The strings the browser script reads, as JSON for a `<script>` element. */
  client(): string;
}

type Table = Record<string, unknown>;

/** The file that holds the strings of a language. */
export const tablePath = (id: string): string => join(root, "content/i18n", `${id}.json`);

export function readTable(id: string): Table {
  const path = tablePath(id);
  if (!existsSync(path)) throw new Error(`content/i18n/${id}.json is missing`);
  const table: unknown = JSON.parse(readFileSync(path, "utf8"));
  if (!isRecord(table)) throw new Error(`content/i18n/${id}.json: not an object`);
  return table;
}

const cache = new Map<string, T>();

/** The strings of one language. */
export function strings(locale: Locale): T {
  const known = cache.get(locale.id);
  if (known) return known;
  const own = readTable(locale.id);
  const english = locale.id === ENGLISH.id ? own : readTable(ENGLISH.id);
  const text = (key: string): string | undefined => {
    const value = own[key] ?? english[key];
    return typeof value === "string" ? value : undefined;
  };
  const categories = own["categories"];
  let client: string | undefined;
  const lookup = (key: string, vars: Record<string, string | number> = {}): string => {
    const value = text(key);
    if (value === undefined) throw new Error(`content/i18n/en.json has no string \`${key}\``);
    return value.replaceAll(/\{(\w+)\}/gu, (all, name: string) => (name in vars ? String(vars[name]) : all));
  };
  const t: T = Object.assign(lookup, {
    has: (key: string) => text(key) !== undefined,
    or: (key: string, fallback: string) => (text(key) === undefined ? fallback : lookup(key)),
    category: (name: string) => {
      const value = isRecord(categories) ? categories[name] : undefined;
      return typeof value === "string" ? value : name;
    },
    client: () => {
      if (client !== undefined) return client;
      const keys = Object.keys(english).filter((key) => key.startsWith("client."));
      const pairs = Object.fromEntries(keys.map((key) => [key.slice("client.".length), lookup(key)]));
      // `<` cannot close the element from inside a string.
      client = JSON.stringify(pairs).replaceAll("<", "\\u003c");
      return client;
    },
  });
  cache.set(locale.id, t);
  return t;
}

/** Drops the strings read so far: `npm run dev` rebuilds after a change to them. */
export function forgetStrings(): void {
  cache.clear();
}

/** Replaces every `{{t:key}}` with its text. */
export function translate(template: string, t: T): string {
  return template.replaceAll(/\{\{t:([\w.]+)\}\}/gu, (_all, key: string) => t(key));
}

/**
 * Points every root-relative link of a page at the page of the same language,
 * where that language carries it. `pages` holds the URLs of the language,
 * without the prefix. A link to a page that exists in English alone stays.
 */
export function localizeLinks(html: string, locale: Locale, pages: ReadonlySet<string>): string {
  if (locale.id === ENGLISH.id) return html;
  const link = new RegExp(`(href|value)="${base}([^"#?]*)`, "gu");
  return html.replaceAll(link, (all, attr: string, path: string) =>
    pages.has(path) ? `${attr}="${base}${prefix(locale)}${path}` : all);
}

/** The URLs of each language, without the prefix. */
export type Sitemap = ReadonlyMap<string, ReadonlySet<string>>;

/** The languages that carry `path`, in the order of `LOCALES`. Empty where one language alone carries it. */
export function carriers(path: string, site: Sitemap): Locale[] {
  const langs = LOCALES.filter((locale) => site.get(locale.id)?.has(path) === true);
  return langs.length < 2 ? [] : langs;
}

/** The `hreflang` links of a page. Empty where one language alone carries it. */
export function alternates(path: string, site: Sitemap): string {
  const langs = carriers(path, site);
  if (langs.length === 0) return "";
  const link = (lang: string, locale: Locale) =>
    `<link rel="alternate" hreflang="${lang}" href="${absolute(locale, path)}">`;
  return [...langs.map((locale) => link(locale.id, locale)), link("x-default", ENGLISH)].join("\n");
}

// A language that lacks the page takes the closest page above it: a model page
// leads to the device list of that language.
function nearest(path: string, pages: ReadonlySet<string> | undefined): string {
  let at = path;
  while (at !== "" && pages?.has(at) !== true) at = at.replace(/[^/]*\/?$/u, "");
  return at;
}

/** The menu that moves the reader to the same page in another language. */
export function switcher(current: Locale, path: string, site: Sitemap, t: T): string {
  const items = LOCALES.map((locale) => {
    const href = `${base}${prefix(locale)}${nearest(path, site.get(locale.id))}`;
    const on = locale.id === current.id ? ' aria-current="true"' : "";
    return `<li><a href="${escapeAttr(href)}" hreflang="${locale.id}" lang="${locale.id}"${on}>${escapeHtml(locale.name)}</a></li>`;
  });
  return `<details class="lang-menu" data-lang-menu>
          <summary aria-label="${escapeAttr(`${t("nav.language")}: ${current.name}`)}">${GLOBE}<span>${current.id.toUpperCase()}</span></summary>
          <ul>${items.join("")}</ul>
        </details>`;
}

const GLOBE = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3Z"/></svg>';

/**
 * The script that moves a visitor from the English home page to the home page
 * of their language. It acts only on arrival from another site, so a link
 * inside the site always lands where it points. The choice that the language
 * menu stores in `lang` wins over the browser languages.
 * Search engines read the English page: their browser asks for English.
 */
export function homeRedirect(): string {
  const others = JSON.stringify(TRANSLATIONS.map((l) => l.id));
  return `<script>try{(function(){var o=${others},r=document.referrer;` +
    `if(r&&new URL(r).host===location.host)return;var w=localStorage.getItem("lang");` +
    `if(!w){var n=navigator.languages||[navigator.language];` +
    `for(var i=0;i<n.length&&!w;i++){var c=String(n[i]).toLowerCase().split("-")[0];if(c==="en"||o.indexOf(c)>=0)w=c}}` +
    `if(w&&o.indexOf(w)>=0)location.replace("${base}"+w+"/"+location.search+location.hash)})()}catch(e){}</script>`;
}
