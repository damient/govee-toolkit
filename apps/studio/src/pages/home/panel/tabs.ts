import { icon } from "../../../components/icon/icon.ts";
import { element } from "../../../core/dom.ts";

/** One function of the light panel. A new function (music, scenes) adds one item to the list. */
export interface Tab {
  id: string;
  /** A path of `icon.ts`. */
  icon: string;
  label: string;
  node: HTMLElement;
}

let next = 0;

/** The tabs over their panels. One tab shows its node alone, with no tab bar. The arrow keys move between the tabs. */
export function tabs(list: Tab[], start: string, label: string): HTMLElement {
  const [first] = list;
  if (first === undefined) return element("div", "");
  if (list.length === 1) return first.node;
  const wrap = element("div", "panel-tabs");
  const bar = element("div", "panel-tabbar");
  bar.setAttribute("role", "tablist");
  bar.setAttribute("aria-label", label);
  next += 1;
  const buttons = list.map((tab) => {
    const button = element("button", "panel-tab");
    button.type = "button";
    button.id = `panel-tab-${String(next)}-${tab.id}`;
    button.setAttribute("role", "tab");
    button.append(icon(tab.icon), element("span", "", tab.label));
    tab.node.setAttribute("role", "tabpanel");
    tab.node.setAttribute("aria-labelledby", button.id);
    return button;
  });
  const select = (id: string, focus: boolean): void => {
    list.forEach((tab, index) => {
      const on = tab.id === id;
      const button = buttons[index];
      tab.node.hidden = !on;
      if (button === undefined) return;
      button.setAttribute("aria-selected", String(on));
      button.tabIndex = on ? 0 : -1;
      if (on && focus) button.focus();
    });
  };
  buttons.forEach((button, index) => {
    const id = list[index]?.id ?? first.id;
    button.addEventListener("click", () => {
      select(id, false);
    });
    button.addEventListener("keydown", (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      if (step === undefined) return;
      const target = list[(index + step + list.length) % list.length];
      if (target !== undefined) select(target.id, true);
    });
  });
  bar.append(...buttons);
  wrap.append(bar, ...list.map((tab) => tab.node));
  select(list.some((tab) => tab.id === start) ? start : first.id, false);
  return wrap;
}
