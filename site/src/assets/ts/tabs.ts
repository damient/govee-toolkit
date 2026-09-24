import { paint, wire } from "./tablist.ts";

// One choice holds for the whole page.
export function tabs(): void {
  const elements = [...document.querySelectorAll<HTMLElement>("[data-langs]")];
  if (elements.length === 0) return;

  const groups = elements.map((el) => ({
    el,
    buttons: [...el.querySelectorAll<HTMLButtonElement>(".tabs button")],
    panes: [...el.querySelectorAll<HTMLElement>(".pane")],
    on: null as string | undefined | null,
  }));

  const show = (language: string | undefined) => {
    for (const group of groups) {
      const wanted = group.buttons.some((button) => button.dataset.lang === language);
      const chosen = wanted ? language : group.el.dataset.langs;
      if (chosen === group.on) continue;
      group.on = chosen;
      paint(group.buttons, group.panes, "lang", chosen);
    }
  };

  const choose = (language: string | undefined) => {
    if (language === undefined) return;
    show(language);
    try {
      localStorage.setItem("language", language);
    } catch {
      // Private mode: the choice lasts for this page only.
    }
  };

  for (const group of groups) {
    wire(group.el, "lang", group.buttons, choose);
  }

  let stored: string | null = null;
  try {
    stored = localStorage.getItem("language");
  } catch {
    // Private mode: nothing is stored.
  }
  if (stored !== null) show(stored);
}
