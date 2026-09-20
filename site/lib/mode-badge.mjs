// The mode badge, used wherever a mode is named. The label is the mode in
// capitals: it reads as a label beside the DMX badge, and the word itself is
// the one the reader types on the command line and writes in a configuration
// file.

import { MODES } from "./config.mjs";
import { escapeHtml } from "./html.mjs";
import { badgeIcon, familyIcon, modeIcon } from "./icons.mjs";

const badge = (kind, iconHtml, text) =>
  `<span class="mbadge mbadge-${kind}">${iconHtml}${text}</span>`;

/** One badge, naming the mode. */
export function modeBadge(mode) {
  return badge(mode, modeIcon(mode), mode.toUpperCase());
}

/** The same badge with the icon alone, for a row that already carries a word.
 * The name stays in the markup, for a screen reader and for a pointer. */
export function modeMark(mode) {
  return `<span class="mbadge mbadge-${mode} mbadge-mini" title="${mode}">${modeIcon(mode)}`
    + `<span class="visually-hidden">${mode}</span></span>`;
}

/** One badge, saying the model answers a DMX channel table. It names an
 * input and not a mode, so it takes the neutral tint. */
export function dmxBadge() {
  return badge("dmx", badgeIcon("dmx"), "DMX");
}

/** One badge, naming the device family. Empty where the catalog names none.
 * It takes the same box as a mode badge: the two sit on one line. */
export function familyBadge(family) {
  return family ? badge("family", familyIcon(family), escapeHtml(family)) : "";
}

/** One `{{badge_<mode>}}` variable per mode, so a static page names a mode
 * with the same component the model pages use. For `fill()`. */
export function modeBadges() {
  return Object.fromEntries(MODES.map((m) => [`badge_${m}`, modeBadge(m)]));
}
