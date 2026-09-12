// Progressive enhancement only. Every page reads and works with this file
// blocked: the rope and the strip are decoration, and the device table is in
// the HTML before the filter touches it.

import { copy } from "./copy.js";
import { filter, rowLink } from "./devices.js";
import { docSelect, menu } from "./menu.js";
import { rope } from "./rope.js";
import { spy } from "./spy.js";
import { strip } from "./strip.js";
import { tabs } from "./tabs.js";
import { theme } from "./theme.js";

const WIRING = [
  ["[data-rope]", rope],
  ["[data-strip]", strip],
  ["[data-copy]", copy],
  ["[data-filter]", filter],
  ["[data-rows]", rowLink],
  ["[data-doc-select]", docSelect],
  ["[data-theme-toggle]", theme],
  ["[data-menu-toggle]", menu],
  ["[data-spy]", spy],
];

for (const [selector, wire] of WIRING) {
  document.querySelectorAll(selector).forEach(wire);
}
tabs();
