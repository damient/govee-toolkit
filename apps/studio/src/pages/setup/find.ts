import { alert } from "../../components/alert/alert.ts";
import { type Found, type Keep, devices, saveDevices, scan } from "../../core/api.ts";
import { help } from "../../components/help/help.ts";
import { explain, t } from "../../core/i18n.ts";
import { type Draft, lightCard } from "./light.ts";
import { loading } from "../../components/loading/loading.ts";
import { ARROW, icon } from "../../shell/nav.ts";
import { notice } from "../../components/notice/notice.ts";
import { go, pathOf } from "../../core/router.ts";

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
function empty(result: Scan, run: () => void): HTMLElement {
  const fill = element("div", "fill");
  const again = button(t("setup.again"), REFRESH, run);
  fill.append(
    result.kind === "failed"
      ? notice({ icon: ALERT, title: t("setup.failed"), body: result.reason, actions: [again] })
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

function grid(list: Found[], drafts: Map<string, Draft>, checking = false): HTMLElement {
  const node = element("div", "light-grid");
  for (const found of list) node.append(lightCard(found, draftOf(drafts, found), checking));
  return node;
}

/**
 * The lights of the configuration with the result of the scan: `online`, and
 * the SKU and the family that the entry does not give.
 */
function merged(configured: Found[], result: Scan): Found[] {
  if (result.kind !== "found") return configured;
  const answered = new Map(result.list.map((found) => [found.id.toUpperCase(), found]));
  return configured.map((light) => {
    const seen = answered.get(light.id.toUpperCase());
    return { ...light, online: seen !== undefined, sku: light.sku ?? seen?.sku ?? null, family: light.family ?? seen?.family ?? null };
  });
}

/** The new lights while the scan runs, or when it failed. */
function waiting(result: Scan): HTMLElement {
  if (result.kind === "failed") return alert({ tone: "error", title: t("setup.failed"), body: result.reason });
  return loading({ text: t("setup.scanning") });
}

function newTitle(count: number): string {
  if (count === 0) return t("setup.new");
  return t(count === 1 ? "setup.new.one" : "setup.new.other", { count: String(count) });
}

/** The new lights: they answered the scan, and the configuration has no entry for them. */
function fresh(list: Found[], drafts: Map<string, Draft>, result: Scan): HTMLElement {
  const count = list.length;
  const h2 = element("h2", "section-title", newTitle(count));
  h2.append(help(t("setup.help")));
  const section = element("section", "light-group");
  if (result.kind !== "found") section.append(h2, waiting(result));
  else if (count > 0) section.append(h2, grid(list, drafts));
  else section.append(h2, alert({ tone: "success", title: t("setup.new.none"), body: t("setup.new.none.body") }));
  return section;
}

/** The lights of the configuration, that answered the scan or not. */
function known(list: Found[], drafts: Map<string, Draft>, checking: boolean): HTMLElement {
  const count = list.length;
  const section = element("section", "light-group");
  section.append(element("h2", "section-title", t(count === 1 ? "setup.known.one" : "setup.known.other", { count: String(count) })), grid(list, drafts, checking));
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
function actions(list: Found[], drafts: Map<string, Draft>, again: HTMLButtonElement): HTMLElement {
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
  row.append(error, again, next);
  return row;
}

/**
 * The lights of the configuration at once, and the scan in the background.
 * The scan gives the status of each light and the new lights. "Scan again"
 * runs the scan again and keeps the drafts.
 */
export function findStep(): HTMLElement {
  const section = element("section", "scan");
  section.setAttribute("aria-live", "polite");
  const drafts = new Map<string, Draft>();
  let configured: Found[] | null = null;
  let result: Scan = { kind: "scanning" };
  const draw = (): void => {
    if (configured === null) return;
    const mine = merged(configured, result);
    const added = result.kind === "found" ? result.list.filter((found) => !found.configured) : [];
    if (mine.length === 0 && added.length === 0) {
      section.replaceChildren(result.kind === "scanning" ? loading({ block: true, text: t("setup.scanning") }) : empty(result, rescan));
      return;
    }
    const again = button(t("setup.again"), REFRESH, rescan);
    again.disabled = result.kind === "scanning";
    const parts = [fresh(added, drafts, result)];
    if (mine.length > 0) parts.push(known(mine, drafts, result.kind === "scanning"));
    section.replaceChildren(...parts, actions([...mine, ...added], drafts, again));
  };
  function rescan(): void {
    result = { kind: "scanning" };
    draw();
    scan().then(
      (list) => {
        result = { kind: "found", list };
        draw();
      },
      (failure: unknown) => {
        result = { kind: "failed", reason: explain(failure) };
        draw();
      },
    );
  }
  function start(): void {
    section.replaceChildren(loading({ block: true, text: t("setup.loading") }));
    devices().then(
      (list) => {
        configured = list;
        draw();
      },
      (failure: unknown) => {
        section.replaceChildren(empty({ kind: "failed", reason: explain(failure) }, start));
      },
    );
  }
  start();
  rescan();
  return section;
}
