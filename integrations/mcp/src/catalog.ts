// The embedded catalog, as the read tools see it. Every value a tool accepts
// is read from here, so the source names no SKU, mode or capability.

import { Catalog, MODES, PERSONALITIES, SUPPORT } from "govee-toolkit";
import { z } from "zod";

const described = z.looseObject({
  sku: z.string(),
  name: z.string(),
  family: z.string(),
  capabilities: z.array(z.string()),
  modes: z.record(z.string(), z.looseObject({ support: z.string(), capabilities: z.array(z.string()) })),
});

const dmxEntry = z.object({ personalities: z.array(z.looseObject({ personality: z.string() })) });

/** The part of the `describe` record that the tools read. The rest passes through. */
export type Described = z.infer<typeof described>;

/** The DMX entry of one device, as `Catalog.dmx()` returns it. */
export type DmxEntry = z.infer<typeof dmxEntry>;

/** One short row of `list_devices`. */
export interface Row {
  sku: string;
  /** The SKU of the device file when `sku` is a verified alias, else null. */
  alias_of: string | null;
  name: string;
  family: string;
  support: Record<string, string>;
}

const catalog = Catalog.embedded();
const skus: readonly string[] = catalog.skus();

const records = new Map(skus.map((sku) => [sku, described.parse(catalog.describe(sku))]));
const tables = new Map(skus.map((sku) => [sku, dmxEntry.parse(catalog.dmx(sku))]));

// The maps hold every SKU. A miss calls the binding only for the `unknown_sku` error it throws.

/** Throws the binding error, with its `code`, when nothing declares the SKU. */
export function describe(sku: string): Described {
  return records.get(sku) ?? described.parse(catalog.describe(sku));
}

/** Throws the binding error, with its `code`, when nothing declares the SKU. */
export function dmx(sku: string): DmxEntry {
  return tables.get(sku) ?? dmxEntry.parse(catalog.dmx(sku));
}

/** The record that `describe_device` and the device resource return. */
export function record(sku: string): Described & { dmx: DmxEntry } {
  return { ...describe(sku), dmx: dmx(sku) };
}

export function allSkus(): readonly string[] {
  return skus;
}

export function row(sku: string): Row {
  const found = describe(sku);
  return {
    sku,
    alias_of: found.sku === sku ? null : found.sku,
    name: found.name,
    family: found.family,
    support: Object.fromEntries(Object.entries(found.modes).map(([mode, m]) => [mode, m.support])),
  };
}

/** The values a filter or an argument accepts, as the core lists them. */
export const vocabulary = {
  modes: [...MODES],
  capabilities: catalog.capabilities(),
  support: [...SUPPORT],
  personalities: [...PERSONALITIES],
};
