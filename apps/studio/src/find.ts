import { alert } from "./alert.ts";
import { type Found, type Keep, devices, saveDevices } from "./api.ts";
import { help } from "./help.ts";
import { ARROW } from "./home.ts";
import { explain, t } from "./i18n.ts";
import { type Draft, lightCard } from "./light.ts";
import { loading } from "./loading.ts";
import { icon } from "./nav.ts";
import { notice } from "./notice.ts";
import { go, pathOf } from "./router.ts";

const WIFI = "M12 18h.01M9.2 15.2a4 4 0 0 1 5.6 0M6.3 12.3a8 8 0 0 1 11.4 0M3.5 9.4a12 12 0 0 1 17 0";
const ALERT = "M12 8v5M12 16h.01M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z";
const REFRESH = "M20 11A8.1 8.1 0 0 0 4.5 9M4 5v4h4M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4";

type Scan = { kind: "scanning" } | { kind: "found"; list: Found[] } | { kind: "failed"; reason: string };

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function button(text: string, d: string, run: () => void): HTMLButtonElement {
  const node = element("button", "button");
  node.type = "button";
  node.append(icon(d), text);
  node.addEventListener("click", run);
  return node;
}

/** The scan found no light and the configuration has none, or the scan failed: a card with "Scan again". */
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

/** The draft of each light, selected at first. A new scan keeps the drafts. */
function draftOf(drafts: Map<string, Draft>, found: Found): Draft {
  const draft = drafts.get(found.id) ?? { selected: true, name: found.name };
  drafts.set(found.id, draft);
  return draft;
}

function grid(list: Found[], drafts: Map<string, Draft>): HTMLElement {
  const node = element("div", "light-grid");
  for (const found of list) node.append(lightCard(found, draftOf(drafts, found)));
  return node;
}

function newTitle(count: number): string {
  if (count === 0) return t("setup.new");
  return t(count === 1 ? "setup.new.one" : "setup.new.other", { count: String(count) });
}

/** The new lights: they answered the scan, and the configuration has no entry for them. */
function fresh(list: Found[], drafts: Map<string, Draft>): HTMLElement {
  const count = list.length;
  const h2 = element("h2", "section-title", newTitle(count));
  h2.append(help(t("setup.help")));
  const section = element("section", "light-group");
  section.append(h2, count > 0 ? grid(list, drafts) : alert({ tone: "success", title: t("setup.new.none"), body: t("setup.new.none.body") }));
  return section;
}

/** The lights of the configuration, that answered the scan or not. */
function known(list: Found[], drafts: Map<string, Draft>): HTMLElement {
  const count = list.length;
  const section = element("section", "light-group");
  section.append(element("h2", "section-title", t(count === 1 ? "setup.known.one" : "setup.known.other", { count: String(count) })), grid(list, drafts));
  return section;
}

/**
 * Write the drafts: the configuration keeps each selected light with its
 * name, and loses each light of the configuration that is not selected.
 */
function save(list: Found[], drafts: Map<string, Draft>): Promise<void> {
  const keep: Keep[] = [];
  const remove: string[] = [];
  for (const found of list) {
    const draft = draftOf(drafts, found);
    if (draft.selected) keep.push({ id: found.id, sku: found.sku, name: draft.name ?? "" });
    else if (found.configured) remove.push(found.id);
  }
  return saveDevices(keep, remove);
}

/** Continue saves the drafts and goes to the next step. An error shows on its left. */
function actions(list: Found[], drafts: Map<string, Draft>, run: () => void): HTMLElement {
  const error = element("p", "error");
  error.hidden = true;
  const next = element("button", "button primary");
  next.type = "button";
  next.append(t("setup.continue"), icon(ARROW));
  next.addEventListener("click", () => {
    next.disabled = true;
    error.hidden = true;
    save(list, drafts).then(
      () => {
        go(pathOf("setup/rooms"));
      },
      (failure: unknown) => {
        next.disabled = false;
        error.hidden = false;
        error.textContent = explain(failure);
      },
    );
  });
  const row = element("div", "buttons end");
  row.append(error, button(t("setup.again"), REFRESH, run), next);
  return row;
}

/** The scan and its result. It scans at once, and again on "Scan again". */
export function findStep(): HTMLElement {
  const section = element("section", "scan");
  section.setAttribute("aria-live", "polite");
  const drafts = new Map<string, Draft>();
  const draw = (scan: Scan): void => {
    if (scan.kind === "scanning") {
      section.replaceChildren(loading({ block: true, text: t("setup.scanning") }));
      return;
    }
    if (scan.kind === "failed" || scan.list.length === 0) {
      section.replaceChildren(empty(scan, run));
      return;
    }
    const configured = scan.list.filter((found) => found.configured);
    const parts = [fresh(scan.list.filter((found) => !found.configured), drafts)];
    if (configured.length > 0) parts.push(known(configured, drafts));
    section.replaceChildren(...parts, actions(scan.list, drafts, run));
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
