// A model of a listing has no device file: it shares the definition of
// `devices/listed/<listing>.yaml`.

import { escapeAttr } from "./html.ts";
import type { Catalog, Device } from "./types.ts";

const TIP = "Untested for this model";

// The icon of a listed model with no candidate shape: the first rule whose
// words all match the name, else the one of its category. The words and the
// categories name a shape of product, never a model.
const SHAPE_WORDS: [RegExp[], string][] = [
  [[/\bstrip\b/iu, /\brgbic/iu], "rgbic-neon-rope"],
  [[/\bcone\b/iu], "tree"],
  [[/\bchristmas\b/iu], "snowflake"],
];
const SHAPE_CATEGORIES = new Map([
  ["RGBIC Strip Lights", "rgbic-neon-rope"],
  ["LED Bulbs", "rgbww-bulb"],
  ["Ceiling Light", "ceiling-light"],
  ["Ceiling Fan Light", "ceiling-light"],
  ["Wall Lights", "wall-light"],
  ["Projector Light", "projector-light"],
]);

// The icon of a listed model that no rule and no category draws.
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
export function infoBadge(): string {
  return `<span class="info-tip" tabindex="0" role="img" aria-label="${escapeAttr(TIP)}" data-tip="${escapeAttr(TIP)}">i</span>`;
}

export function listedVerification(): string {
  return `<h2 id="verification">Verification</h2>
      <p>This model is on Govee's list of LAN Control models. The toolkit sends
      it the commands that Govee documents for every model on that list, and
      nobody has tested them on this one yet.
      <a href="{{repo}}/issues">Report what you observe</a>, and the model gets
      a device file of its own.</p>`;
}

export function listedFile(d: Device): string {
  const path = `devices/listed/${d.listing ?? ""}.yaml`;
  return `<h2 id="the-device-file">The device file</h2>
          <p>This model shares the definition in
          <a href="{{repo}}/blob/main/${escapeAttr(path)}"><code>${path}</code></a>
          with every model of Govee's LAN list that has no file of its own.</p>`;
}
