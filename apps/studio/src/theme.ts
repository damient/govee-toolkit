import { read, write } from "./prefs.ts";

export const THEMES = ["dark", "light", "auto"] as const;
export type Theme = (typeof THEMES)[number];

// index.html reads the same key before the first paint.
export function theme(): Theme {
  return read("theme", THEMES, "dark");
}

/** Store `value` and apply it: the tokens in style.css follow `data-theme`. */
export function setTheme(value: Theme): void {
  write("theme", value);
  document.documentElement.dataset["theme"] = value;
}
