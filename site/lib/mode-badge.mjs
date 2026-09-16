// The mode badge, used wherever a mode is named. The label stays `lan`, `ble`
// and `cloud`: that is the word the reader types on the command line and
// writes in a configuration file.

import { MODES } from "./config.mjs";
import { escapeHtml } from "./html.mjs";
import { familyIcon, modeIcon } from "./icons.mjs";

const badge = (kind, iconHtml, text) =>
  `<span class="mbadge mbadge-${kind}">${iconHtml}${text}</span>`;

/** One badge, naming the mode. */
export function modeBadge(mode) {
  return badge(mode, modeIcon(mode), mode);
}

/** The same badge with the icon alone, for a row that already carries a word.
 * The name stays in the markup, for a screen reader and for a pointer. */
export function modeMark(mode) {
  return `<span class="mbadge mbadge-${mode} mbadge-mini" title="${mode}">${modeIcon(mode)}`
    + `<span class="visually-hidden">${mode}</span></span>`;
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
