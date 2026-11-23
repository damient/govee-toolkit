import "./setup.css";
import { findStep } from "./find.ts";
import { roomsStep } from "./rooms.ts";
import { type Key, t } from "../../core/i18n.ts";
import { pathOf } from "../../core/router.ts";
import { title } from "../../components/title/title.ts";
import { element } from "../../core/dom.ts";

const STEPS: { key: Key; page: string }[] = [
  { key: "setup.step.find", page: "setup" },
  { key: "setup.step.rooms", page: "setup/rooms" },
];

function steps(at: number): HTMLElement[] {
  const list = element("ol", "steps");
  list.setAttribute("aria-label", t("setup.steps"));
  for (const [index, step] of STEPS.entries()) {
    const item = element("li", index < at ? "done" : "");
    if (index < at) {
      const link = element("a", "", t(step.key));
      link.href = pathOf(step.page);
      item.append(link);
    } else {
      item.textContent = t(step.key);
    }
    if (index === at) item.setAttribute("aria-current", "step");
    list.append(item);
  }
  const position = { at: String(at + 1), count: String(STEPS.length) };
  const short = element("span", "steps-short", t("setup.step.short", position));
  short.setAttribute("aria-label", t("setup.step.at", position));
  return [list, short];
}

function head(view: HTMLElement, at: number, back: string): void {
  view.classList.add("column");
  view.append(title({ text: t("setup.title"), back, end: steps(at) }));
}

export function setup(view: HTMLElement): void {
  head(view, 0, pathOf("home"));
  view.append(findStep());
}

export function rooms(view: HTMLElement): void {
  head(view, 1, pathOf("setup"));
  view.append(roomsStep());
}
