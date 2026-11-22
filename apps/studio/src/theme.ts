import { prefs, save } from "./prefs.ts";

export const THEMES = ["dark", "light", "auto"] as const;
export type Theme = (typeof THEMES)[number];

export function theme(): Theme {
  return THEMES.find((value) => value === prefs().theme) ?? "dark";
}

/** Give the browser bars the color of the page. */
export function paint(): void {
  const meta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
  if (meta) meta.content = getComputedStyle(document.body).backgroundColor;
}

/** Apply the stored theme: the tokens in style.css follow `data-theme`. */
export function applyTheme(): void {
  document.documentElement.dataset["theme"] = theme();
  paint();
}

/** Apply `value` and store it on the server. */
export async function setTheme(value: Theme): Promise<void> {
  const saved = save({ theme: value });
  applyTheme();
  await saved;
}

// "Auto" follows a change of the system while the page is open.
matchMedia("(prefers-color-scheme: light)").addEventListener("change", paint);
