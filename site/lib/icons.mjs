// One icon per capability, per mode and per device family. Each one is a file
// under `src/icons/`, so it opens in a vector editor, and the site inlines it:
// the site ships no icon font and no third-party sprite.
//
// The site keeps the `viewBox` and the shapes of a file and re-wraps them. The
// class, the color and the stroke of the root tag are the site's, so an icon
// follows the theme. A shape that carries a color of its own keeps it, and
// that is how the mode icons draw one filled dot.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./config.mjs";

const DIR = join(root, "src/icons");

function load(kind) {
  const dir = join(DIR, kind);
  const icons = new Map();
  for (const file of readdirSync(dir)) {
    if (!file.endsWith(".svg")) continue;
    const source = readFileSync(join(dir, file), "utf8");
    const open = source.match(/<svg\b[^>]*>/);
    if (!open) throw new Error(`${kind}/${file}: no <svg> element`);
    const viewBox = /viewBox="([^"]+)"/.exec(open[0])?.[1];
    if (!viewBox) throw new Error(`${kind}/${file}: no viewBox`);
    const inner = source.slice(open.index + open[0].length, source.lastIndexOf("</svg>"));
    icons.set(file.slice(0, -4), { viewBox, inner: inner.trim().replace(/\s*\n\s*/g, "") });
  }
  return icons;
}

const CAPS = load("cap");
const MODES = load("mode");
const FAMILIES = load("family");

// The two segment capabilities share one icon, and the color tells them apart:
// they address the same zones.
const CAP_FILES = new Map([["segment_brightness", "segments"]]);

const svg = (found, klass, attrs = "") =>
  found
    ? `<svg class="${klass}"${attrs} viewBox="${found.viewBox}" fill="none" stroke="currentColor"`
      + ` stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"`
      + ` aria-hidden="true">${found.inner}</svg>`
    : "";

/** The icon of one capability, or an empty string when no file draws it. */
export function icon(key) {
  const name = CAP_FILES.get(key) ?? key;
  return svg(CAPS.get(name), "cap-icon", ` data-cap="${key}"`);
}

/** The icon of one mode, or an empty string when no file draws it. */
export function modeIcon(mode) {
  return svg(MODES.get(mode), "mode-icon");
}

/** The icon of one device family, or an empty string when no file draws it. */
export function familyIcon(family) {
  return svg(FAMILIES.get(family), "family-icon");
}
