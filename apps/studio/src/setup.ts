import { findStep } from "./find.ts";
import { type Key, t } from "./i18n.ts";
import { pathOf } from "./router.ts";
import { title } from "./title.ts";

const STEPS: Key[] = ["setup.step.find", "setup.step.rooms"];

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** The steps of the setup, and "1 / 2" for a phone. `at` is the index of the current one. */
function steps(at: number): HTMLElement[] {
  const list = element("ol", "steps");
  list.setAttribute("aria-label", t("setup.steps"));
  for (const [index, key] of STEPS.entries()) {
    const item = element("li", index < at ? "done" : "", t(key));
    if (index === at) item.setAttribute("aria-current", "step");
    list.append(item);
  }
  const position = { at: String(at + 1), count: String(STEPS.length) };
  const short = element("span", "steps-short", t("setup.step.short", position));
  short.setAttribute("aria-label", t("setup.step.at", position));
  return [list, short];
}

/** The head of a step. The back link goes to the step before, or home from the first one. */
function head(view: HTMLElement, at: number, back: string): void {
  view.classList.add("column");
  view.append(title({ text: t("setup.title"), back, end: steps(at) }));
}

/** Draw the first step of the setup: find the lights on the network. */
export function setup(view: HTMLElement): void {
  head(view, 0, pathOf("home"));
  view.append(findStep());
}

/** Draw the second step of the setup: put the lights in rooms. */
export function rooms(view: HTMLElement): void {
  head(view, 1, pathOf("setup"));
}
