import { escapeAttr, escapeHtml, slugify } from "./html.ts";
import { type Locale, type T, strings } from "./locales.ts";
import { collectionData } from "./seo.ts";
import type { Crumb, Device } from "./types.ts";

/** The URL of one category page, relative to the site root. */
export const categoryUrl = (category: string): string => `devices/${slugify(category)}/`;

/** Every category with its model count, sorted by name. */
export function tally(devices: Device[]): [string, number][] {
  const count = new Map<string, number>();
  for (const d of devices) if (d.category !== undefined) count.set(d.category, (count.get(d.category) ?? 0) + 1);
  return [...count].toSorted(([a], [b]) => a.localeCompare(b));
}

/** What a list page shows: every model, the models tested on hardware, or one category. */
export type ListView = { kind: "all" } | { kind: "tested" } | { kind: "category"; name: string };

export const TESTED_URL = "devices/tested/";

/** A model that a device file describes and that somebody verified on a unit. */
export const isTested = (d: Device): boolean => d.listing === undefined && d.verified?.date !== undefined;

/** The models one view lists. */
export function members(devices: Device[], view: ListView): Device[] {
  if (view.kind === "tested") return devices.filter((d) => isTested(d));
  if (view.kind === "category") return devices.filter((d) => d.category === view.name);
  return devices;
}

const option = (url: string, text: string, on: boolean): string =>
  `<option value="{{base}}${escapeAttr(url)}"${on ? " selected" : ""}>${escapeHtml(text)}</option>`;

/** The options of the category select. Each value is the URL of its page.
 * The categories sort by their name in the language of the page. */
export function categoryOptions(devices: Device[], view: ListView, t: T): string {
  const all = option("devices/", t("list.all_categories"), view.kind === "all");
  const tested = option(TESTED_URL, `${t("list.tested")} (${members(devices, { kind: "tested" }).length})`, view.kind === "tested");
  const each = tally(devices)
    .map(([name, n]) => ({ name, n, shown: t.category(name) }))
    .toSorted((a, b) => a.shown.localeCompare(b.shown, t.lang))
    .map(({ name, n, shown }) => option(categoryUrl(name), `${shown} (${n})`, view.kind === "category" && view.name === name));
  return [all, tested, ...each].join("\n          ");
}

/** The trail of a model page: the category sits between the list and the model. */
export function deviceTrail(d: Device, t: T): Crumb[] {
  const middle: Crumb[] = d.category === undefined ? [] : [[t.category(d.category), categoryUrl(d.category)]];
  return [[t("page.devices"), "devices/"], ...middle, [d.sku, `devices/${d.sku}/`]];
}

/** The page of the tested models, then one page per category, in one language,
 * rendered from the template of the devices page. */
export function categoryPages(
  devices: Device[],
  render: (template: string, devices: Device[], view: ListView, t: T) => string,
  locale: Locale,
) {
  const t = strings(locale);
  const page = (view: ListView, url: string, shown: string, title: string, description: string) => ({
    src: "devices.html",
    url,
    nav: "devices",
    title,
    description,
    render: (template: string, all: Device[]) => render(template, all, view, t),
    trail: [[t("page.devices"), "devices/"], [shown, url]] satisfies Crumb[],
    jsonld: [collectionData(title, locale, url, members(devices, view).map((d): Crumb => [`${d.sku} - ${d.name}`, `devices/${d.sku}/`]))],
  });
  const tested = members(devices, { kind: "tested" });
  return [
    page({ kind: "tested" }, TESTED_URL, t("list.tested"), t("list.tested_title"), t("list.tested_description", { count: tested.length })),
    ...tally(devices).map(([name]) => {
      const view: ListView = { kind: "category", name };
      const shown = t.category(name);
      return page(view, categoryUrl(name), shown, t("list.category_title", { name: shown }),
        t("list.category_description", { count: members(devices, view).length, name: shown }));
    }),
  ];
}
