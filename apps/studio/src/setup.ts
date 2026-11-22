import { type Found, devices } from "./api.ts";
import { help } from "./help.ts";
import { ARROW } from "./home.ts";
import { type Key, explain, t } from "./i18n.ts";
import { lightCard } from "./light.ts";
import { loading } from "./loading.ts";
import { icon } from "./nav.ts";
import { notice } from "./notice.ts";
import { pathOf } from "./router.ts";
import { title } from "./title.ts";

const STEPS: Key[] = ["setup.step.find", "setup.step.lan", "setup.step.rooms"];
const WIFI = "M12 18h.01M9.2 15.2a4 4 0 0 1 5.6 0M6.3 12.3a8 8 0 0 1 11.4 0M3.5 9.4a12 12 0 0 1 17 0";
const ALERT = "M12 8v5M12 16h.01M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z";
const REFRESH = "M20 11A8.1 8.1 0 0 0 4.5 9M4 5v4h4M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4";
const CLOSE = "M18 6L6 18M6 6l12 12";

type Scan = { kind: "scanning" } | { kind: "found"; list: Found[] } | { kind: "failed"; reason: string };

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** The steps of the setup, and "1 / 3" for a phone. `at` is the index of the current one. */
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

function close(): HTMLElement {
  const link = element("a", "icon-button");
  link.href = pathOf("home");
  link.title = t("setup.close");
  link.setAttribute("aria-label", t("setup.close"));
  link.append(icon(CLOSE));
  return link;
}

/** A heading with the help on missing lights. */
function heading(text: string): HTMLElement {
  const h2 = element("h2", "section-title", text);
  h2.append(help(t("setup.help")));
  return h2;
}

function button(text: string, d: string, run: () => void): HTMLButtonElement {
  const node = element("button", "button");
  node.type = "button";
  node.append(icon(d), text);
  node.addEventListener("click", run);
  return node;
}

/** The scan found no light, or failed: a card in the free space, with "Scan again". */
function empty(scan: Scan, run: () => void): HTMLElement {
  const fill = element("div", "fill");
  const again = button(t("setup.again"), REFRESH, run);
  fill.append(
    scan.kind === "failed"
      ? notice({ icon: ALERT, title: t("setup.failed"), body: scan.reason, actions: [again] })
      : notice({ icon: WIFI, title: t("setup.none"), body: t("setup.none.body"), actions: [again] }),
  );
  return fill;
}

/** The lights found, selected at first. `selected` keeps the choice over a new scan. */
function result(list: Found[], selected: Map<string, boolean>, run: () => void): HTMLElement[] {
  const grid = element("div", "light-grid");
  for (const found of list) {
    if (!selected.has(found.id)) selected.set(found.id, true);
    grid.append(
      lightCard(found, selected.get(found.id) ?? true, (on) => {
        selected.set(found.id, on);
      }),
    );
  }
  const next = element("button", "button primary");
  next.type = "button";
  next.disabled = true;
  next.append(t("setup.continue"), icon(ARROW));
  const actions = element("div", "buttons end");
  actions.append(button(t("setup.again"), REFRESH, run), next);
  const count = list.length;
  return [heading(t(count === 1 ? "setup.found.one" : "setup.found.other", { count: String(count) })), grid, actions];
}

/** The scan and its result. It scans at once, and again on "Scan again". */
function scanSection(): HTMLElement {
  const section = element("section", "scan");
  section.setAttribute("aria-live", "polite");
  const selected = new Map<string, boolean>();
  const draw = (scan: Scan): void => {
    if (scan.kind === "scanning") section.replaceChildren(loading({ block: true, text: t("setup.scanning") }));
    else if (scan.kind === "found" && scan.list.length > 0) section.replaceChildren(...result(scan.list, selected, run));
    else section.replaceChildren(empty(scan, run));
  };
  function run(): void {
    draw({ kind: "scanning" });
    devices().then(
      (list) => {
        draw({ kind: "found", list });
      },
      (failure: unknown) => {
        draw({ kind: "failed", reason: explain(failure) });
      },
    );
  }
  run();
  return section;
}

/** Draw the first step of the setup: find the lights on the network. */
export function setup(view: HTMLElement): void {
  view.classList.add("column");
  view.append(title({ text: t("setup.title"), end: [...steps(0), close()] }), scanSection());
}
