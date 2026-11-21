import { escapeAttr, escapeHtml, slugify } from "./html.ts";
import { type Locale, type T, prefix, strings } from "./locales.ts";
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

/** The options of the category select. Each value is the URL of its page. */
export function categoryOptions(devices: Device[], current: string | undefined, t: T): string {
  const all = `<option value="{{base}}devices/"${current === undefined ? " selected" : ""}>${escapeHtml(t("list.all_categories"))}</option>`;
  const each = tally(devices).map(([name, n]) => {
    const on = name === current ? " selected" : "";
    return `<option value="{{base}}${escapeAttr(categoryUrl(name))}"${on}>${escapeHtml(t.category(name))} (${n})</option>`;
  });
  return [all, ...each].join("\n          ");
}

/** The trail of a model page: the category sits between the list and the model. */
export function deviceTrail(d: Device, t: T): Crumb[] {
  const middle: Crumb[] = d.category === undefined ? [] : [[t.category(d.category), categoryUrl(d.category)]];
  return [[t("page.devices"), "devices/"], ...middle, [d.sku, `devices/${d.sku}/`]];
}

/** One page per category of a language, rendered from the template of the devices page. */
export function categoryPages(
  devices: Device[],
  render: (template: string, devices: Device[], current: string, t: T) => string,
  locale: Locale,
) {
  const t = strings(locale);
  return tally(devices).map(([name]) => {
    const url = categoryUrl(name);
    const members = devices.filter((d) => d.category === name);
    const shown = t.category(name);
    const trail: Crumb[] = [[t("page.devices"), "devices/"], [shown, url]];
    const title = t("list.category_title", { name: shown });
    return {
      src: "devices.html",
      url,
      nav: "devices",
      title,
      description: t("list.category_description", { count: members.length, name: shown }),
      render: (template: string, all: Device[]) => render(template, all, name, t),
      trail,
      jsonld: [collectionData(title, `${prefix(locale)}${url}`, members.map((d): Crumb => [`${d.sku} — ${d.name}`, `devices/${d.sku}/`]))],
    };
  });
}
