import { paint, wire } from "./tablist.js";

// The personality tabs of a model page: one channel table at a time. The
// choice stays inside the block, because a personality belongs to one model
// and not to the reader.
export function personalities(block) {
  const buttons = [...block.querySelectorAll("button[data-personality]")];
  const panes = [...block.querySelectorAll(".dmx-pane")];
  if (buttons.length < 2) return;
  wire(block, "personality", buttons, (name) => paint(buttons, panes, "personality", name));
}
