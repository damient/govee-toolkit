// What `scripts/copy-data.ts` copied into `data/`: the API join and the docs.

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { z } from "zod";

// `src/` and `dist/` both sit one level under the package root.
const data = fileURLToPath(new URL("../data/", import.meta.url));

const api = z.object({
  schema_version: z.number(),
  roles: z.record(z.string(), z.record(z.string(), z.array(z.string()))),
  methods: z.record(z.string(), z.array(z.string())),
});

/** The file `xtask api` writes. Each surface lists every method that serves the role. */
export type Api = z.infer<typeof api>;

/** Reads `data/api.json`. Throws when the file is missing or has another shape. */
export function readApi(): Api {
  return api.parse(JSON.parse(readFileSync(`${data}api.json`, "utf8")));
}

// `data/` does not change while the server runs.
const docs = new Map(
  readdirSync(`${data}docs`, { recursive: true, encoding: "utf8" })
    .filter((path) => path.endsWith(".md"))
    .map((path) => [path.replaceAll("\\", "/").slice(0, -".md".length), readFileSync(`${data}docs/${path}`, "utf8")]),
);

/** Every docs page copied, as its path under `docs/` without `.md`: `modes`, `protocol/lan`. */
export function docTopics(): string[] {
  return [...docs.keys()].toSorted();
}

/** One docs page. `topic` must be one that {@link docTopics} returns. */
export function readDoc(topic: string): string {
  return docs.get(topic) ?? "";
}
