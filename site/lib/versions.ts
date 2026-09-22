// The version of each package, read from its manifest when the site is built,
// and the registry that serves it. The site carries no version of its own, so
// a release that bumps a manifest moves the page with it.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { repo } from "./config.ts";
import { escapeHtml } from "./html.ts";
import { badgeIcon } from "./icons.ts";

// One package, one manifest and one registry. The key is the name a page
// writes as `{{version_<key>}}` and `{{registry_<key>}}`.
const PACKAGES = {
  rust: {
    manifest: "packages/rust/Cargo.toml",
    registry: "crates.io",
    url: "https://crates.io/crates/govee-toolkit",
  },
  cli: {
    manifest: "packages/rust/crates/cli/Cargo.toml",
    registry: "crates.io",
    url: "https://crates.io/crates/govee-toolkit-cli",
  },
  dmx: {
    manifest: "packages/rust/crates/dmx/Cargo.toml",
    registry: "crates.io",
    url: "https://crates.io/crates/govee-toolkit-dmx",
  },
  python: {
    manifest: "packages/python/pyproject.toml",
    registry: "PyPI",
    url: "https://pypi.org/project/govee-toolkit/",
  },
  node: {
    manifest: "packages/node/package.json",
    registry: "npm",
    url: "https://www.npmjs.com/package/govee-toolkit",
  },
};

// The first `version` at the head of a line, which is the one the `[package]`
// or the `[project]` table holds. A dependency states its version indented or
// inside a table of its own.
function fromToml(text: string): string | undefined {
  return /^version\s*=\s*"([^"]+)"/m.exec(text)?.[1];
}

function read(path: string): string {
  const text = readFileSync(join(repo, path), "utf8");
  const version: unknown = path.endsWith(".json") ? JSON.parse(text).version : fromToml(text);
  if (typeof version !== "string" || !version) throw new Error(`${path}: no version`);
  return version;
}

let chips: Record<string, string> | null = null;

/** One `{{version_<package>}}` and one `{{registry_<package>}}` per package,
 * as the chips a heading closes with. For `fill()`.
 *
 * Read once: every page of one build carries the same versions. */
export function versionChips(): Record<string, string> {
  if (chips) return chips;
  const vars: Record<string, string> = {};
  for (const [name, pkg] of Object.entries(PACKAGES)) {
    vars[`version_${name}`] = `<span class="version">v${escapeHtml(read(pkg.manifest))}</span>`;
    vars[`registry_${name}`] = `<a class="version version-link" href="${pkg.url}"`
      + ` target="_blank" rel="noreferrer">${badgeIcon("link")}`
      + `${escapeHtml(pkg.registry)}</a>`;
  }
  chips = vars;
  return vars;
}
