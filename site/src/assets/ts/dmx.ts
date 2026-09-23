import { paint, wire } from "./tablist.ts";

// The choice stays inside the block: a personality belongs to one model.
export function personalities(block: HTMLElement): void {
  const buttons = [...block.querySelectorAll<HTMLButtonElement>("button[data-personality]")];
  const panes = [...block.querySelectorAll<HTMLElement>(".dmx-pane")];
  if (buttons.length < 2) return;
  wire(block, "personality", buttons, (name) => { paint(buttons, panes, "personality", name); });
}
