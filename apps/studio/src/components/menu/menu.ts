import "./menu.css";
import { icon } from "../icon/icon.ts";
import { element } from "../../core/dom.ts";

const DOTS = "M12 5h.01M12 12h.01M12 19h.01";

export interface MenuItem {
  text: string;
  /** The `d` attribute of the icon. */
  icon: string;
  run: () => void;
  /** `danger` marks an action that deletes. */
  tone?: "plain" | "danger";
}

export interface Menu {
  /** The name of the button, for a screen reader. */
  label: string;
  items: MenuItem[];
}

/** Put `list` under `button`, with their right edges aligned, or above it near the bottom edge. */
function place(button: HTMLElement, list: HTMLElement): void {
  const margin = 6;
  const box = button.getBoundingClientRect();
  const left = Math.max(margin, Math.min(box.right - list.offsetWidth, innerWidth - list.offsetWidth - margin));
  const below = box.bottom + margin;
  const top = below + list.offsetHeight > innerHeight - margin ? box.top - margin - list.offsetHeight : below;
  list.style.left = `${left}px`;
  list.style.top = `${top}px`;
}

function entry(item: MenuItem, close: () => void): HTMLButtonElement {
  const node = element("button", `menu-item ${item.tone ?? "plain"}`);
  node.type = "button";
  node.setAttribute("role", "menuitem");
  node.tabIndex = -1;
  node.append(icon(item.icon), item.text);
  node.addEventListener("click", () => {
    close();
    item.run();
  });
  return node;
}

/**
 * A button with three dots that opens a list of actions. Arrow keys move in
 * the list, Esc, a click elsewhere and a scroll close it.
 */
export function menu(spec: Menu): HTMLElement {
  const wrap = element("span", "menu");
  const button = element("button", "icon-button menu-button");
  button.type = "button";
  button.setAttribute("aria-label", spec.label);
  button.setAttribute("aria-haspopup", "menu");
  button.setAttribute("aria-expanded", "false");
  button.append(icon(DOTS));
  const list = element("div", "menu-list");
  list.setAttribute("role", "menu");
  list.hidden = true;
  const items = (): HTMLElement[] => [...list.querySelectorAll<HTMLElement>(".menu-item")];
  const outside = (event: Event): void => {
    if (event.target instanceof Node && wrap.contains(event.target)) return;
    show(false);
  };
  let opened = false;
  function show(open: boolean): void {
    opened = open;
    list.hidden = !open;
    button.setAttribute("aria-expanded", String(open));
    const listen = open ? document.addEventListener.bind(document) : document.removeEventListener.bind(document);
    listen("pointerdown", outside, true);
    listen("scroll", outside, true);
    if (open) place(button, list);
  }
  list.append(...spec.items.map((item) => entry(item, () => {
    show(false);
  })));
  button.addEventListener("click", () => {
    show(!opened);
    if (opened) items()[0]?.focus();
  });
  wrap.addEventListener("keydown", (event) => {
    if (!opened) return;
    const all = items();
    const at = all.findIndex((node) => node === document.activeElement);
    if (event.key === "Escape") {
      show(false);
      button.focus();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : all.length - 1;
      all[(at + step) % all.length]?.focus();
    }
  });
  wrap.append(button, list);
  return wrap;
}
