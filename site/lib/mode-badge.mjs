// The mode badge, used wherever a mode is named: the icon and the color carry
// the mode, and the optional second value follows the name behind a bar.
//
// The label stays `lan`, `ble` and `cloud`, because that is the word the
// reader types on the command line and writes in a configuration file.

import { modeIcon } from "./icons.mjs";

/**
 * One badge. `state` is optional and renders behind a bar: pass the support
 * of the mode, or nothing when the badge only names it.
 */
export function modeBadge(mode, state) {
  const extra = state
    ? `<span class="mbadge-extra"><span aria-hidden="true">|</span> ${state === "unknown" ? "?" : state}</span>`
    : "";
  return `<span class="mbadge mbadge-${mode}">${modeIcon(mode)}${mode}${extra}</span>`;
}
