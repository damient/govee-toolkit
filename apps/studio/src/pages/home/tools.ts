import { SEARCH, SUN, icon } from "../../components/icon/icon.ts";
import { element } from "../../core/dom.ts";
import { t } from "../../core/i18n.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";

/** An icon button of the Home bar, with `label` as its name and its tooltip. */
export function toolButton(path: string, label: string): HTMLButtonElement {
  const button = element("button", "icon-button home-tool");
  button.type = "button";
  button.setAttribute("aria-label", label);
  button.append(icon(path));
  tooltip(button, label);
  return button;
}

/** Esc and a click outside `wrap` run `close`. The function that it returns stops the watch. */
function dismiss(wrap: HTMLElement, close: () => void): () => void {
  const outside = (event: Event): void => {
    if (event.target instanceof Node && !wrap.contains(event.target)) close();
  };
  const key = (event: KeyboardEvent): void => {
    if (event.key === "Escape") close();
  };
  document.addEventListener("pointerdown", outside, true);
  wrap.addEventListener("keydown", key);
  return () => {
    document.removeEventListener("pointerdown", outside, true);
    wrap.removeEventListener("keydown", key);
  };
}

/** A search button that opens the field. The field stays open while it holds a query. */
export function search(find: (query: string) => void): HTMLElement {
  const wrap = element("div", "home-search");
  const button = toolButton(SEARCH, t("home.search.open"));
  const input = element("input", "");
  input.type = "search";
  input.placeholder = t("home.search");
  input.setAttribute("aria-label", t("home.search"));
  input.hidden = true;
  let stop: (() => void) | null = null;
  const open = (on: boolean): void => {
    input.hidden = !on;
    button.hidden = on;
    wrap.classList.toggle("open", on);
    stop?.();
    stop = null;
    if (!on) return;
    input.focus();
    stop = dismiss(wrap, () => {
      if (input.value === "") open(false);
    });
  };
  button.addEventListener("click", () => {
    open(true);
  });
  input.addEventListener("input", () => {
    find(input.value.trim().toLowerCase());
  });
  input.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    input.value = "";
    find("");
    open(false);
    button.focus();
  });
  wrap.append(button, input);
  return wrap;
}

/** A brightness button that opens a slider. `level` gives the start value, in percent. */
export function brightness(level: () => number, dim: (percent: number) => void): HTMLElement {
  const wrap = element("div", "home-bright");
  const button = toolButton(SUN, t("home.level"));
  button.setAttribute("aria-expanded", "false");
  const pop = element("div", "home-bright-pop");
  pop.hidden = true;
  const slider = element("input", "home-bright-range");
  slider.type = "range";
  slider.min = "1";
  slider.max = "100";
  slider.setAttribute("aria-label", t("home.level"));
  const value = element("span", "home-bright-value");
  const draw = (): void => {
    value.textContent = `${slider.value}%`;
    slider.style.setProperty("--fill", `${slider.value}%`);
  };
  slider.addEventListener("input", () => {
    draw();
    dim(Number(slider.value));
  });
  let stop: (() => void) | null = null;
  const open = (on: boolean): void => {
    pop.hidden = !on;
    button.setAttribute("aria-expanded", String(on));
    stop?.();
    stop = null;
    if (!on) return;
    slider.value = String(level());
    draw();
    slider.focus();
    stop = dismiss(wrap, () => {
      open(false);
    });
  };
  button.addEventListener("click", () => {
    open(button.getAttribute("aria-expanded") !== "true");
  });
  pop.append(slider, value);
  wrap.append(button, pop);
  return wrap;
}
