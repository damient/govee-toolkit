import { escapeAttr, escapeHtml } from "./html.ts";
import { EN, type T } from "./locales.ts";
import type { Catalog, Device } from "./types.ts";


// A rule and a category name a shape of product, never a model.
const SHAPE_WORDS: [RegExp[], string][] = [
  [[/\bstrip\b/iu, /\brgbic/iu], "rgbic-neon-rope"],
  [[/\bcone\b/iu], "tree"],
  [[/\bcurtain\b/iu], "curtain"],
  [[/\bchristmas\b/iu], "snowflake"],
];
const SHAPE_CATEGORIES = new Map([
  ["RGBIC Strip Lights", "rgbic-neon-rope"],
  ["LED Bulbs", "rgbww-bulb"],
  ["Ceiling Light", "ceiling-light"],
  ["Ceiling Fan Light", "ceiling-light"],
  ["Wall Lights", "wall-light"],
  ["Projector Light", "projector-light"],
  ["Landscape Lighting", "sprout"],
  ["TV Backlights", "tv"],
  ["Pixel Light", "pixel-light"],
]);

const FALLBACK_WORDS: [RegExp, string][] = [[/\bstrip\b/iu, "rgb-car-strip"]];

function shapeOf(name: string, category: string | undefined): string | undefined {
  return SHAPE_WORDS.find(([words]) => words.every((w) => w.test(name)))?.[1]
    ?? (category === undefined ? undefined : SHAPE_CATEGORIES.get(category))
    ?? FALLBACK_WORDS.find(([word]) => word.test(name))?.[1];
}

/**
 * Every device file and every model of a listing, sorted by SKU, so that two
 * builds of one catalog give one page. A model that a device file names as a
 * candidate alias takes the family of that file as its shape. `categories`
 * gives each SKU on Govee's LAN list its category.
 */
export function everyModel(catalog: Catalog, categories: Map<string, string>): Device[] {
  const shapes = new Map<string, string>();
  for (const d of catalog.devices) {
    if (d.family === undefined) continue;
    for (const sku of d.candidate_aliases ?? []) shapes.set(sku, d.family);
  }
  const listed = (catalog.listings ?? []).flatMap((l) =>
    l.models.map((m): Device => {
      const shape = shapes.get(m.sku) ?? shapeOf(m.name, categories.get(m.sku));
      return { ...l.device, ...m, listing: l.listing, ...(shape === undefined ? {} : { shape }) };
    }),
  );
  return [...catalog.devices, ...listed]
    .map((d): Device => {
      const category = categories.get(d.sku);
      return category === undefined ? d : Object.assign({}, d, { category });
    })
    .toSorted((a, b) => a.sku.localeCompare(b.sku));
}

/** The mark beside the name of a listed model. */
export function infoBadge(t: T = EN): string {
  const tip = t("list.untested");
  return `<span class="info-tip" tabindex="0" role="img" aria-label="${escapeAttr(tip)}" data-tip="${escapeAttr(tip)}">i</span>`;
}

export function listedVerification(t: T): string {
  return `<h2 id="verification">${escapeHtml(t("model.verification"))}</h2>
      <p>${t("listed.verification")}</p>`;
}

export function listedFile(d: Device, t: T): string {
  const path = `devices/listed/${d.listing ?? ""}.yaml`;
  const link = `<a href="{{repo}}/blob/main/${escapeAttr(path)}"><code>${path}</code></a>`;
  return `<h2 id="the-device-file">${escapeHtml(t("model.file_title"))}</h2>
          <p>${t("listed.file", { file: link })}</p>`;
}
