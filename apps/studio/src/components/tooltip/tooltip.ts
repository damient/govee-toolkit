import "./tooltip.css";
let count = 0;

/** A tooltip element, hidden, with an id for `aria-describedby`. */
export function tipElement(text: string): HTMLElement {
  count += 1;
  const tip = document.createElement("span");
  tip.className = "tip";
  tip.id = `tip-${count}`;
  tip.setAttribute("role", "tooltip");
  tip.textContent = text;
  tip.hidden = true;
  return tip;
}

/** Keep `tip` in the viewport, under `anchor`, or above it near the bottom edge. */
export function place(anchor: HTMLElement, tip: HTMLElement): void {
  const margin = 8;
  const box = anchor.getBoundingClientRect();
  const width = tip.offsetWidth;
  const left = Math.min(Math.max(margin, box.left + box.width / 2 - width / 2), innerWidth - width - margin);
  const below = box.bottom + margin;
  const top = below + tip.offsetHeight > innerHeight - margin ? box.top - margin - tip.offsetHeight : below;
  tip.style.left = `${left}px`;
  tip.style.top = `${top}px`;
}

/**
 * Show `text` in a tooltip on hover and on focus of `target`. Esc, a click
 * and a scroll close it. The tooltip is in `document.body` only while it shows.
 */
export function tooltip(target: HTMLElement, text: string): void {
  const tip = tipElement(text);
  target.setAttribute("aria-describedby", tip.id);
  const show = (open: boolean): void => {
    tip.hidden = !open;
    if (!open) {
      tip.remove();
      document.removeEventListener("scroll", close, true);
      return;
    }
    document.body.append(tip);
    place(target, tip);
    // The tooltip has a fixed position: a scroll would leave it behind.
    document.addEventListener("scroll", close, true);
  };
  const close = (): void => {
    show(false);
  };
  target.addEventListener("mouseenter", () => {
    show(true);
  });
  target.addEventListener("focus", () => {
    if (target.matches(":focus-visible")) show(true);
  });
  target.addEventListener("mouseleave", close);
  target.addEventListener("blur", close);
  target.addEventListener("pointerdown", close);
  target.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close();
  });
}
