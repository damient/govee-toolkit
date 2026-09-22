// One icon per capability, per mode and per device family, as a file under
// `src/icons/`: the site ships no icon font and no third-party sprite.
//
// The site keeps a file's `viewBox` and shapes, and gives the root tag its own
// class, color and stroke, so an icon follows the theme. A shape that carries a
// color of its own keeps it, and that is how a mode icon draws one filled dot.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./config.ts";

const DIR = join(root, "src/icons");

interface Icon {
  viewBox: string;
  inner: string;
}

function load(kind: string): Map<string, Icon> {
  const dir = join(DIR, kind);
  const icons = new Map<string, Icon>();
  for (const file of readdirSync(dir)) {
    if (!file.endsWith(".svg")) continue;
    const source = readFileSync(join(dir, file), "utf8");
    const open = source.match(/<svg\b[^>]*>/);
    if (open?.index === undefined) throw new Error(`${kind}/${file}: no <svg> element`);
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
const SLOTS = load("slot");
const BADGES = load("badge");

// The two segment capabilities share one icon, and the color tells them apart:
// they address the same zones.
const CAP_FILES = new Map([["segment_brightness", "segments"]]);

const svg = (found: Icon | undefined, klass: string, attrs = ""): string =>
  found
    ? `<svg class="${klass}"${attrs} viewBox="${found.viewBox}" fill="none" stroke="currentColor"`
      + ` stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"`
      + ` aria-hidden="true">${found.inner}</svg>`
    : "";

/** Whether `key` draws the icon of another capability. A list that shows one
 * mark per icon leaves such a key out. */
export function sharesMark(key: string): boolean {
  return CAP_FILES.has(key);
}

/** The icon of one capability, or an empty string when no file draws it. */
export function icon(key: string): string {
  const name = CAP_FILES.get(key) ?? key;
  return svg(CAPS.get(name), "cap-icon", ` data-cap="${key}"`);
}

/** The icon of one mode, or an empty string when no file draws it. */
export function modeIcon(mode: string): string {
  return svg(MODES.get(mode), "mode-icon");
}

/** The icon of one DMX slot that no capability draws, or an empty string when
 * no file draws it either. */
export function slotIcon(name: string): string {
  return svg(SLOTS.get(name), "cap-icon", ` data-slot="${name}"`);
}

/** The icon of one badge that names no mode, such as the DMX one. */
export function badgeIcon(name: string): string {
  return svg(BADGES.get(name), "mode-icon");
}

/** The icon of one device family, or an empty string when no file draws it. */
export function familyIcon(family: string | undefined): string {
  return svg(family ? FAMILIES.get(family) : undefined, "family-icon");
}
