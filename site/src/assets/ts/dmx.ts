import { paint, wire } from "./tablist.ts";

// The personality tabs of a model page: one channel table at a time. The
// choice stays inside the block, because a personality belongs to one model
// and not to the reader.
export function personalities(block: HTMLElement): void {
  const buttons = [...block.querySelectorAll<HTMLButtonElement>("button[data-personality]")];
  const panes = [...block.querySelectorAll<HTMLElement>(".dmx-pane")];
  if (buttons.length < 2) return;
  wire(block, "personality", buttons, (name) => paint(buttons, panes, "personality", name));
}
