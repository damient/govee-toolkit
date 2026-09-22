// Progressive enhancement only. Every page reads and works with this file
// blocked: the line under the top bar and the strip are decoration, and the
// device table is in the HTML before the filter touches it.

import { copy } from "./copy.ts";
import { personalities } from "./dmx.ts";
import { filter, rowLink } from "./devices.ts";
import { docSelect, menu } from "./menu.ts";
import { spy } from "./spy.ts";
import { strip } from "./strip.ts";
import { tabs } from "./tabs.ts";
import { theme } from "./theme.ts";
import { topbar } from "./topbar.ts";

// The selector names the element and the module names its type. Nothing
// checks that the two agree.
const wire = <E extends Element>(selector: string, setup: (element: E) => void) =>
  document.querySelectorAll<E>(selector).forEach((element) => setup(element));

wire("[data-strip]", strip);
wire("[data-topbar]", topbar);
wire("[data-copy]", copy);
wire("[data-filter]", filter);
wire("[data-rows]", rowLink);
wire("[data-doc-select]", docSelect);
wire("[data-theme-toggle]", theme);
wire("[data-menu-toggle]", menu);
wire("[data-spy]", spy);
wire("[data-dmx]", personalities);
tabs();
