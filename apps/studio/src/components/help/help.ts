import "./help.css";
import { t } from "../../core/i18n.ts";
import { place, tipElement } from "../tooltip/tooltip.ts";

function parts(text: string): { wrap: HTMLElement; badge: HTMLButtonElement; tip: HTMLElement } {
  const wrap = document.createElement("span");
  wrap.className = "help";
  const badge = document.createElement("button");
  badge.type = "button";
  badge.className = "help-badge";
  badge.textContent = "?";
  badge.setAttribute("aria-label", t("help.label"));
  const tip = tipElement(text);
  badge.setAttribute("aria-describedby", tip.id);
  wrap.append(badge, tip);
  return { wrap, badge, tip };
}

/** A "?" badge that shows `text` in a tooltip, also on a tap. Esc and a tap elsewhere close it. */
export function help(text: string): HTMLElement {
  const { wrap, badge, tip } = parts(text);
  let pinned = false;
  const show = (open: boolean): void => {
    tip.hidden = !open;
    badge.setAttribute("aria-expanded", String(open));
    if (!open) {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("scroll", outside, true);
      return;
    }
    place(badge, tip);
    document.addEventListener("pointerdown", outside);
    document.addEventListener("scroll", outside, true);
  };
  const outside = (event: Event): void => {
    if (wrap.isConnected && event.target instanceof Node && wrap.contains(event.target)) return;
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
  return wrap;
}
