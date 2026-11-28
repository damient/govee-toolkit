import { element } from "../../../core/dom.ts";

export interface Switch {
  row: HTMLElement;
  set(on: boolean): void;
}

export function switchRow(text: string, on: boolean, click: (on: boolean) => void): Switch {
  const row = element("label", "panel-row");
  const button = element("button", "switch");
  button.type = "button";
  button.setAttribute("role", "switch");
  const set = (next: boolean): void => {
    button.setAttribute("aria-checked", String(next));
  };
  button.addEventListener("click", () => {
    click(button.getAttribute("aria-checked") !== "true");
  });
  set(on);
  row.append(element("span", "panel-label", text), button);
  return { row, set };
}

export interface Slider {
  row: HTMLElement;
  /** The bar under the knob, as the color bar of a card. */
  track: HTMLElement;
  /** Move the knob to `level`, and send nothing. */
  set: (level: number) => void;
}

/** A value out of the bounds shows at the nearest bound. A `null` text names the slider for a screen reader only. */
export function slider(
  text: string | null,
  name: string,
  [low, high]: [number, number],
  level: number | null,
  change: (level: number) => void,
): Slider {
  const row = element("label", "panel-field");
  const track = element("span", "home-swatch panel-track");
  const input = element("input", "home-level");
  input.setAttribute("aria-label", name);
  input.type = "range";
  input.min = String(low);
  input.max = String(high);
  input.value = String(Math.min(high, Math.max(low, level ?? high)));
  input.addEventListener("input", () => {
    change(Number(input.value));
  });
  track.append(input);
  if (text !== null) row.append(element("span", "panel-label", text));
  row.append(track);
  const set = (next: number): void => {
    input.value = String(Math.min(high, Math.max(low, next)));
  };
  return { row, track, set };
}
