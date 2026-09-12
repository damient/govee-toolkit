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
      for (const button of group.buttons) {
        const on = button.dataset.lang === chosen;
        button.setAttribute("aria-selected", String(on));
        // One stop in the tab order per group: the arrow keys move inside it.
        button.tabIndex = on ? 0 : -1;
      }
      for (const pane of group.panes) {
        pane.hidden = pane.dataset.lang !== chosen;
      }
    }
  };

  const choose = (language) => {
    show(language);
    try {
      localStorage.setItem("language", language);
    } catch { /* private mode: the choice lasts for this page only. */ }
  };

  for (const group of groups) {
    group.el.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-lang]");
      if (button) choose(button.dataset.lang);
    });
    group.el.addEventListener("keydown", (event) => {
      const button = event.target.closest("button[data-lang]");
      if (!button) return;
      const { buttons } = group;
      const step = { ArrowLeft: -1, ArrowRight: 1 }[event.key];
      let next = null;
      if (step) next = buttons[(buttons.indexOf(button) + step + buttons.length) % buttons.length];
      if (event.key === "Home") next = buttons[0];
      if (event.key === "End") next = buttons[buttons.length - 1];
      if (!next) return;
      event.preventDefault();
      choose(next.dataset.lang);
      next.focus();
    });
  }

  let stored = null;
  try {
    stored = localStorage.getItem("language");
  } catch { /* nothing stored. */ }
  if (stored) show(stored);
}
