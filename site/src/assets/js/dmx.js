// The personality tabs of a model page: one channel table at a time. The
// choice stays inside the block, because a personality belongs to one model
// and not to the reader.
export function personalities(block) {
  const buttons = [...block.querySelectorAll("button[data-personality]")];
  const panes = [...block.querySelectorAll(".dmx-pane")];
  if (buttons.length < 2) return;

  const show = (name) => {
    for (const button of buttons) {
      const on = button.dataset.personality === name;
      button.setAttribute("aria-selected", String(on));
      // One stop in the tab order: the arrow keys move inside the list.
      button.tabIndex = on ? 0 : -1;
    }
    for (const pane of panes) {
      pane.hidden = pane.dataset.personality !== name;
    }
  };

  block.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-personality]");
    if (button) show(button.dataset.personality);
  });

  block.addEventListener("keydown", (event) => {
    const button = event.target.closest("button[data-personality]");
    if (!button) return;
    const step = { ArrowLeft: -1, ArrowRight: 1 }[event.key];
    let next = null;
    if (step) next = buttons[(buttons.indexOf(button) + step + buttons.length) % buttons.length];
    if (event.key === "Home") next = buttons[0];
    if (event.key === "End") next = buttons[buttons.length - 1];
    if (!next) return;
    event.preventDefault();
    show(next.dataset.personality);
    next.focus();
  });
}
