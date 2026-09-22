// What the whole build agrees on: where the site lives, and where its inputs
// are.

import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Mode } from "./types.ts";

/** The site directory: every input path starts here. */
export const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const repo = resolve(root, "..");
export const dist = join(root, "dist");

// The absolute form is for the canonical, the sitemap and the preview image,
// which a machine reads outside of a page. Every link in a page is
// root-relative. `public/CNAME` carries the same domain.
export const SITE_URL = "https://gvetk.com";
export const base = "/";
export const repoUrl = "https://github.com/damient/govee-toolkit";

export const DESCRIPTION = "An unofficial toolkit that controls Govee lights over your own network, from Rust, Python, Node.js or the command line.";

/** The catalog `xtask` writes, and the schema the renderers read. */
export const catalogPath = join(repo, "dist/catalog.json");
export const CATALOG_SCHEMA = 1;

/** The modes a device file can declare, in the order a reader looks for. */
export const MODES: readonly Mode[] = ["lan", "ble", "cloud"];
