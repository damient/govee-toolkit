import { icon } from "../../../components/icon/icon.ts";
import { element } from "../../../core/dom.ts";

/** One function of the light panel. A new function (music, scenes) adds one item to the list. */
export interface Tab {
  id: string;
  /** A path of `icon.ts`. */
  icon: string;
  label: string;
  node: HTMLElement;
  /** The tab starts hidden. `Tabs.show` shows it. */
  hidden?: boolean;
}

export interface Tabs {
  node: HTMLElement;
  /** Show or hide the tab `id`. A hidden tab that is selected gives the selection to the first tab shown. */
  show(id: string, shown: boolean): void;
  /** Select the tab `id`. */
  select(id: string): void;
}

let next = 0;

function tabButton(tab: Tab): HTMLButtonElement {
  const button = element("button", "panel-tab");
  button.type = "button";
  button.id = `panel-tab-${String(next)}-${tab.id}`;
  button.setAttribute("role", "tab");
  button.setAttribute("aria-label", tab.label);
  button.append(icon(tab.icon));
  tab.node.setAttribute("role", "tabpanel");
  tab.node.setAttribute("aria-labelledby", button.id);
  return button;
}

function tabBar(label: string): HTMLElement {
  const bar = element("div", "panel-tabbar");
  bar.setAttribute("role", "tablist");
  bar.setAttribute("aria-label", label);
  return bar;
}

function neighbour(open: Tab[], id: string, step: number): Tab | undefined {
  const at = open.findIndex((tab) => tab.id === id);
  return open[(at + step + open.length) % open.length];
}

/** The tabs over their panels. With one tab shown, the bar hides. The arrow keys move between the tabs shown. */
export function tabs(list: Tab[], start: string, label: string): Tabs {
  const wrap = element("div", "panel-tabs");
  const bar = tabBar(label);
  next += 1;
  const shown = new Set(list.filter((tab) => tab.hidden !== true).map((tab) => tab.id));
  const buttons = list.map((tab) => tabButton(tab));
  const visible = (): Tab[] => list.filter((tab) => shown.has(tab.id));
  const first = (): string => visible()[0]?.id ?? "";
  let selected = "";
  const select = (id: string, focus: boolean): void => {
    selected = id;
    list.forEach((tab, index) => {
      const on = tab.id === id;
      const button = buttons[index];
      tab.node.hidden = !on;
      if (button === undefined) return;
      button.hidden = !shown.has(tab.id);
      button.setAttribute("aria-selected", String(on));
      button.tabIndex = on ? 0 : -1;
      if (on && focus) button.focus();
    });
    bar.hidden = visible().length < 2;
  };
  buttons.forEach((button, index) => {
    const id = list[index]?.id ?? "";
    button.addEventListener("click", () => {
      select(id, false);
    });
    button.addEventListener("keydown", (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      const target = step === undefined ? undefined : neighbour(visible(), id, step);
      if (target !== undefined) select(target.id, true);
    });
  });
  bar.append(...buttons);
  wrap.append(bar, ...list.map((tab) => tab.node));
  select(shown.has(start) ? start : first(), false);
  const pick = (id: string): void => {
    select(id, false);
  };
  const show = (id: string, on: boolean): void => {
    if (on) shown.add(id);
    else shown.delete(id);
    pick(shown.has(selected) ? selected : first());
  };
  return { node: wrap, show, select: pick };
}
