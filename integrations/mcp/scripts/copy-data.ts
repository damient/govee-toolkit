// Copies `dist/api.json` and `docs/**/*.md` into `data/`, which the package
// ships. The server reads the docs topics from that directory.

import { existsSync } from "node:fs";
import { copyFile, mkdir, readdir, rm } from "node:fs/promises";
import { dirname, join, relative } from "node:path";

const pkg = dirname(import.meta.dirname);
const repo = join(pkg, "..", "..");
const data = join(pkg, "data");
const api = join(repo, "dist", "api.json");
const docs = join(repo, "docs");

if (!existsSync(api)) {
  console.error("dist/api.json is missing: run `cargo run -p xtask -- api` in packages/rust");
  process.exit(1);
}

await rm(data, { recursive: true, force: true });
await mkdir(join(data, "docs"), { recursive: true });
await copyFile(api, join(data, "api.json"));

const pages = (await readdir(docs, { recursive: true })).filter((path) => path.endsWith(".md"));
await Promise.all(
  pages.map(async (page) => {
    const target = join(data, "docs", page);
    await mkdir(dirname(target), { recursive: true });
    await copyFile(join(docs, page), target);
  }),
);
console.error(`copy-data: api.json and ${pages.length} docs pages into ${relative(repo, data)}`);
