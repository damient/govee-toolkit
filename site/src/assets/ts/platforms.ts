import { paint, wire } from "./tablist.ts";

// One system at a time in `#get`: the first card is open on load.
// With JavaScript blocked, every pane shows.
export function platforms(element: HTMLElement): void {
  const buttons = [...element.querySelectorAll<HTMLButtonElement>("button[data-os]")];
  const panes = [...element.querySelectorAll<HTMLElement>(".get-pane")];
  const show = (os: string | undefined): void => {
    paint(buttons, panes, "os", os);
  };
  wire(element, "os", buttons, show);
  show(buttons[0]?.dataset.os);
}
