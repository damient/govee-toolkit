// What every tab list on the site shares: the roving tab order and the keys
// that move inside it. The data attribute that names a tab is the one
// difference between two lists.

// Select one tab, and show the pane that answers to it.
export function paint(buttons, panes, key, value) {
  for (const button of buttons) {
    const on = button.dataset[key] === value;
    button.setAttribute("aria-selected", String(on));
    // One stop in the tab order: the arrow keys move inside the list.
    button.tabIndex = on ? 0 : -1;
  }
  for (const pane of panes) {
    pane.hidden = pane.dataset[key] !== value;
  }
}

// Wire the click and the keys of one list. `choose` takes the value of the
// tab the reader asked for.
export function wire(element, key, buttons, choose) {
  const target = (event) => event.target.closest(`button[data-${key}]`);
  element.addEventListener("click", (event) => {
    const button = target(event);
    if (button) choose(button.dataset[key]);
  });
  element.addEventListener("keydown", (event) => {
    const button = target(event);
    if (!button) return;
    const step = { ArrowLeft: -1, ArrowRight: 1 }[event.key];
    let next = null;
    if (step) next = buttons[(buttons.indexOf(button) + step + buttons.length) % buttons.length];
    if (event.key === "Home") next = buttons[0];
    if (event.key === "End") next = buttons[buttons.length - 1];
    if (!next) return;
    event.preventDefault();
    choose(next.dataset[key]);
    next.focus();
  });
}
