import { paint, wire } from "./tablist.ts";

// The tab lists that `lib/tabs.ts` writes into a page. A choice holds for
// every list of the same group, on this page and on the next one.
export function choices(): void {
  const groups = [...document.querySelectorAll<HTMLElement>("[data-tabs]")].map((el) => ({
    name: el.dataset.tabs ?? "",
    buttons: [...el.querySelectorAll<HTMLButtonElement>(":scope > .tabs button")],
    panes: [...el.querySelectorAll<HTMLElement>(":scope > .tab-pane")],
    el,
  }));
  if (groups.length === 0) return;

  // A list that does not hold the value keeps its own tab.
  const show = (name: string, value: string) => {
    for (const group of groups) {
      if (group.name !== name || !group.buttons.some((b) => b.dataset.tab === value)) continue;
      paint(group.buttons, group.panes, "tab", value);
    }
  };

  for (const group of groups) {
    wire(group.el, "tab", group.buttons, (value) => {
      if (value === undefined) return;
      show(group.name, value);
      try {
        localStorage.setItem(`tab-${group.name}`, value);
      } catch {
        // Private mode: the choice lasts for this page only.
      }
    });
  }

  for (const name of new Set(groups.map((g) => g.name))) {
    try {
      const stored = localStorage.getItem(`tab-${name}`);
      if (stored !== null) show(name, stored);
    } catch {
      // Private mode: nothing is stored.
    }
  }
}
