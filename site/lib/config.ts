// What the whole build agrees on: where the site lives, and where its inputs
// are.

import { join, resolve } from "node:path";
import type { Mode } from "./types.ts";

/** The site directory: every input path starts here. */
export const root = resolve(import.meta.dirname, "..");
export const repo = resolve(root, "..");
export const dist = join(root, "dist");

// The absolute form is for the canonical, the sitemap and the preview image,
// which a machine reads outside of a page. Every link in a page is
// root-relative. `public/CNAME` carries the same domain.
export const SITE_URL = "https://gvetk.com";
export const base = "/";
export const repoUrl = "https://github.com/damient/govee-toolkit";

export const DESCRIPTION = "An unofficial toolkit that controls Govee lights from your own computer, over Wi-Fi, Bluetooth, DMX or the cloud. From the command line, Rust, Python or Node.js.";

/** The catalog `xtask` writes, and the schema the renderers read. */
export const catalogPath = join(repo, "dist/catalog.json");
export const CATALOG_SCHEMA = 1;

/** The models that carry the LAN switch. `xtask lan` reads the same file. */
export const lanListPath = join(repo, "docs/lan-supported-devices.json");

/** The modes a device file can declare, in the order a reader looks for. */
export const MODES: readonly Mode[] = ["lan", "ble", "cloud"];
