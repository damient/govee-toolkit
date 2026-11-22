import { read, write } from "./prefs.ts";

export const THEMES = ["dark", "light", "auto"] as const;
export type Theme = (typeof THEMES)[number];

// index.html reads the same key before the first paint.
export function theme(): Theme {
  return read("theme", THEMES, "dark");
}

/** Give the browser bars the color of the page. */
export function paint(): void {
  const meta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
  if (meta) meta.content = getComputedStyle(document.body).backgroundColor;
}

/** Store `value` and apply it: the tokens in style.css follow `data-theme`. */
export function setTheme(value: Theme): void {
  write("theme", value);
  document.documentElement.dataset["theme"] = value;
  paint();
}

// "Auto" follows a change of the system while the page is open.
matchMedia("(prefers-color-scheme: light)").addEventListener("change", paint);
