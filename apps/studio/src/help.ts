import { t } from "./i18n.ts";

let count = 0;

/** Keep `tip` in the viewport, under `badge`, or above it near the bottom edge. */
function place(badge: HTMLElement, tip: HTMLElement): void {
  const margin = 8;
  const box = badge.getBoundingClientRect();
  const width = tip.offsetWidth;
  const left = Math.min(Math.max(margin, box.left + box.width / 2 - width / 2), innerWidth - width - margin);
  const below = box.bottom + margin;
  const top = below + tip.offsetHeight > innerHeight - margin ? box.top - margin - tip.offsetHeight : below;
  tip.style.left = `${left}px`;
  tip.style.top = `${top}px`;
}

function parts(text: string): { wrap: HTMLElement; badge: HTMLButtonElement; tip: HTMLElement } {
  count += 1;
  const wrap = document.createElement("span");
  wrap.className = "help";
  const badge = document.createElement("button");
  badge.type = "button";
  badge.className = "help-badge";
  badge.textContent = "?";
  badge.setAttribute("aria-label", t("help.label"));
  const tip = document.createElement("span");
  tip.className = "help-tip";
  tip.id = `help-${count}`;
  tip.setAttribute("role", "tooltip");
  tip.textContent = text;
  tip.hidden = true;
  badge.setAttribute("aria-describedby", tip.id);
  wrap.append(badge, tip);
  return { wrap, badge, tip };
}

/**
 * A "?" badge that shows `text` in a tooltip: on hover, on focus, and on a
 * tap for a touch screen. Esc and a tap elsewhere close it.
 */
export function help(text: string): HTMLElement {
  const { wrap, badge, tip } = parts(text);
  let pinned = false;
  const show = (open: boolean): void => {
    tip.hidden = !open;
    badge.setAttribute("aria-expanded", String(open));
    if (open) place(badge, tip);
  };
  const outside = (event: Event): void => {
    if (!wrap.isConnected) {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("scroll", outside, true);
      return;
    }
    if (event.target instanceof Node && wrap.contains(event.target)) return;
    pinned = false;
    show(false);
  };
  badge.addEventListener("mouseenter", () => {
    show(true);
  });
  badge.addEventListener("mouseleave", () => {
    if (!pinned) show(false);
  });
  badge.addEventListener("focus", () => {
    show(true);
  });
  badge.addEventListener("blur", () => {
    pinned = false;
    show(false);
  });
  badge.addEventListener("click", () => {
    pinned = !pinned;
    show(pinned);
  });
  badge.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    pinned = false;
    show(false);
  });
  document.addEventListener("pointerdown", outside);
  // The tooltip has a fixed position: a scroll would leave it behind.
  document.addEventListener("scroll", outside, true);
  return wrap;
}
