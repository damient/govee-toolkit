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

// An element of another type than the module expects is skipped.
const wire = <E extends Element>(selector: string, type: new () => E, setup: (element: E) => void): void => {
  for (const element of document.querySelectorAll(selector)) {
    if (element instanceof type) setup(element);
  }
};

wire("[data-strip]", HTMLElement, strip);
wire("[data-topbar]", HTMLElement, topbar);
wire("[data-copy]", HTMLButtonElement, copy);
wire("[data-filter]", HTMLInputElement, filter);
wire("[data-rows]", HTMLElement, rowLink);
wire("[data-doc-select]", HTMLDetailsElement, docSelect);
wire("[data-theme-toggle]", HTMLButtonElement, theme);
wire("[data-menu-toggle]", HTMLButtonElement, menu);
wire("[data-spy]", HTMLElement, spy);
wire("[data-dmx]", HTMLElement, personalities);
tabs();
