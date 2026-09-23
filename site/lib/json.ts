// The guards check the top level only. The renderers trust the fields below it.

import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { relative } from "node:path";
import { CATALOG_SCHEMA, catalogPath, repo } from "./config.ts";
import type { Catalog, LanList, Reference } from "./types.ts";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Parses one file. Throws where the guard refuses the value. */
export async function readJson<T>(path: string, is: (value: unknown) => value is T): Promise<T> {
  const value: unknown = JSON.parse(await readFile(path, "utf8"));
  if (!is(value)) throw new Error(`${relative(repo, path)}: unexpected shape`);
  return value;
}

export function isCatalog(value: unknown): value is Catalog {
  return isRecord(value) && typeof value.schema_version === "number" && Array.isArray(value.devices);
}

export function isReference(value: unknown): value is Reference {
  return isRecord(value) && typeof value.intro === "string" && Array.isArray(value.groups);
}

export function isLanList(value: unknown): value is LanList {
  return isRecord(value) && typeof value.source === "string" && Array.isArray(value.models);
}

/** The `version` field of a JSON manifest, or undefined when it has none. */
export function versionOf(text: string): unknown {
  const value: unknown = JSON.parse(text);
  return isRecord(value) ? value.version : undefined;
}

/** Exits where the catalog is missing, or where its schema is not the one the
 * renderers read. */
export async function readCatalog(): Promise<Catalog> {
  if (!existsSync(catalogPath)) {
    console.error(
      `missing ${relative(repo, catalogPath)}.\n` +
      `Generate it first: cargo run -p xtask -- catalog`,
    );
    process.exit(1);
  }
  const catalog = await readJson(catalogPath, isCatalog);
  if (catalog.schema_version !== CATALOG_SCHEMA) {
    console.error(
      `${relative(repo, catalogPath)} is schema ${catalog.schema_version}, ` +
      `and the renderers read schema ${CATALOG_SCHEMA}.\n` +
      `A field that moved renders as "?" and as "not verified", which states ` +
      `something nobody established. Update lib/types.ts and lib/devices.ts first.`,
    );
    process.exit(1);
  }
  return catalog;
}
