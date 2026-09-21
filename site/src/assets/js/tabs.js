import { paint, wire } from "./tablist.js";

// One choice for the whole page: a reader who works in Python reads Python
// everywhere.
export function tabs() {
  const elements = [...document.querySelectorAll("[data-langs]")];
  if (!elements.length) return;

  // Queried once: a language change writes these and queries nothing.
  const groups = elements.map((el) => ({
    el,
    buttons: [...el.querySelectorAll(".tabs button")],
    panes: [...el.querySelectorAll(".pane")],
    on: null,
  }));

  const show = (language) => {
    for (const group of groups) {
      const wanted = group.buttons.some((button) => button.dataset.lang === language);
      const chosen = wanted ? language : group.buttons[0].dataset.lang;
      if (chosen === group.on) continue;
      group.on = chosen;
      paint(group.buttons, group.panes, "lang", chosen);
    }
  };

  const choose = (language) => {
    show(language);
    try {
      localStorage.setItem("language", language);
    } catch { /* private mode: the choice lasts for this page only. */ }
  };

  for (const group of groups) {
    wire(group.el, "lang", group.buttons, choose);
  }

  let stored = null;
  try {
    stored = localStorage.getItem("language");
  } catch { /* nothing stored. */ }
  if (stored) show(stored);
}
