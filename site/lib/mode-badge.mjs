// The mode badge, used wherever a mode is named. The label stays `lan`, `ble`
// and `cloud`: that is the word the reader types on the command line and
// writes in a configuration file.

import { MODES } from "./config.mjs";
import { modeIcon } from "./icons.mjs";

/** One badge, naming the mode. */
export function modeBadge(mode) {
  return `<span class="mbadge mbadge-${mode}">${modeIcon(mode)}${mode}</span>`;
}

/** One `{{badge_<mode>}}` variable per mode, so a static page names a mode
 * with the same component the model pages use. For `fill()`. */
export function modeBadges() {
  return Object.fromEntries(MODES.map((m) => [`badge_${m}`, modeBadge(m)]));
}
