// One page per category of Govee's LAN list, at `devices/<slug>/`. The device
// list renders it from the template of the devices page, with the rows of that
// category alone.

import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { SITE_URL, base, root } from "./config.ts";
import { crumbs } from "./crumbs.ts";
import { escapeAttr, escapeHtml, fill, slugify } from "./html.ts";
import { shapeIcon } from "./icons.ts";
import { breadcrumb } from "./seo.ts";
import type { Crumb, Device } from "./types.ts";

// Paths under `devices/` that a category slug must not take.
const TAKEN = new Set(["add", "lan"]);

/** The URL of one category page, relative to the site root. */
export const categoryUrl = (category: string): string => `devices/${slugify(category)}/`;

/** Every category with its model count, sorted by name. */
export function tally(devices: Device[]): [string, number][] {
  const count = new Map<string, number>();
  for (const d of devices) if (d.category !== undefined) count.set(d.category, (count.get(d.category) ?? 0) + 1);
  return [...count].toSorted(([a], [b]) => a.localeCompare(b));
}

/** The options of the category select. Each value is the URL of its page. */
export function categoryOptions(devices: Device[], current: string | undefined): string {
  const all = `<option value="{{base}}devices/"${current === undefined ? " selected" : ""}>All categories</option>`;
  const each = tally(devices).map(([name, n]) => {
    const on = name === current ? " selected" : "";
    return `<option value="{{base}}${escapeAttr(categoryUrl(name))}"${on}>${escapeHtml(name)} (${n})</option>`;
  });
  return [all, ...each].join("\n          ");
}

/** The badge that names the category of a model and links to its page. Empty
 * where the model has no category. */
export function categoryBadge(d: Device): string {
  if (d.category === undefined) return "";
  return `<a class="mbadge mbadge-category" href="{{base}}${escapeAttr(categoryUrl(d.category))}">`
    + `${shapeIcon(d.shape ?? d.family)}${escapeHtml(d.category)}</a>`;
}

/** The trail of a model page: the category sits between the list and the model. */
export function deviceTrail(d: Device): Crumb[] {
  const middle: Crumb[] = d.category === undefined ? [] : [[d.category, categoryUrl(d.category)]];
  return [["Devices", "devices/"], ...middle, [d.sku, `devices/${d.sku}/`]];
}

/** One page per category, ready for the layout. Throws where a slug takes the
 * path of another page. */
export async function categoryPages(
  devices: Device[],
  render: (template: string, devices: Device[], current: string) => string,
) {
  const template = await readFile(join(root, "src/pages/devices.html"), "utf8");
  const skus = new Set(devices.map((d) => d.sku.toLowerCase()));
  return tally(devices).map(([name, n]) => {
    const slug = slugify(name);
    if (TAKEN.has(slug) || skus.has(slug)) throw new Error(`category ${name}: devices/${slug}/ is taken`);
    const url = categoryUrl(name);
    const members = devices.filter((d) => d.category === name);
    const trail: Crumb[] = [["Devices", "devices/"], [name, url]];
    const body = fill(render(template, devices, name), { crumbs: crumbs(trail) });
    return {
      url,
      nav: "devices",
      title: `Govee ${name}`,
      description: `The ${n} Govee ${name} models in the Govee Toolkit catalog, and what each one answers over Wi-Fi, over Bluetooth, through the cloud and from a DMX desk.`,
      jsonld: [breadcrumb(trail), itemList(name, url, members)],
      body,
    };
  });
}

function itemList(name: string, url: string, members: Device[]) {
  return {
    "@context": "https://schema.org",
    "@type": "CollectionPage",
    name: `Govee ${name}`,
    url: `${SITE_URL}${base}${url}`,
    mainEntity: {
      "@type": "ItemList",
      numberOfItems: members.length,
      itemListElement: members.map((d, at) => ({
        "@type": "ListItem",
        position: at + 1,
        name: `${d.sku} — ${d.name}`,
        url: `${SITE_URL}${base}devices/${d.sku}/`,
      })),
    },
  };
}
