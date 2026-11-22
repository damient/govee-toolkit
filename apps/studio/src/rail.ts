import { t } from "./i18n.ts";
import { icon } from "./nav.ts";
import { read, write } from "./prefs.ts";

const STATES = ["full", "min"] as const;
type State = (typeof STATES)[number];

const FRAME = "M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2zM9 4v16";
const COLLAPSE = `${FRAME}M15 10l-2 2 2 2`;
const EXPAND = `${FRAME}M14 10l2 2-2 2`;

/**
 * Set the rail of a wide screen to "full" (icons and labels) or "min" (icons
 * only) with `button`. The tablet and the phone layouts ignore the choice.
 * Call the function that it returns after a change of language.
 */
export function rail(app: HTMLElement, button: HTMLButtonElement): () => void {
  let state: State = read("rail", STATES, "full");
  const draw = (): void => {
    const full = state === "full";
    app.dataset["rail"] = state;
    button.replaceChildren(icon(full ? COLLAPSE : EXPAND));
    button.setAttribute("aria-label", t(full ? "rail.collapse" : "rail.expand"));
    button.setAttribute("aria-expanded", String(full));
    button.title = t(full ? "rail.collapse" : "rail.expand");
  };
  button.addEventListener("click", () => {
    state = state === "full" ? "min" : "full";
    write("rail", state);
    draw();
  });
  draw();
  return draw;
}
