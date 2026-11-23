import "./tooltip.css";
let count = 0;

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
 * Show `text` on hover with a mouse or a pen, and on focus from the keyboard.
 * Esc, a click and a scroll close it. The function that it returns changes the text.
 */
export function tooltip(target: HTMLElement, text: string): (text: string) => void {
  const tip = tipElement(text);
  target.setAttribute("aria-describedby", tip.id);
  const show = (open: boolean): void => {
    tip.hidden = !open;
    if (!open) {
      tip.remove();
      document.removeEventListener("scroll", close, true);
      document.removeEventListener("pointerover", outside, true);
      return;
    }
    // An open dialog is in the top layer: a tooltip outside it shows under it.
    (target.closest("dialog") ?? document.body).append(tip);
    place(target, tip);
    // The tooltip has a fixed position: a scroll would leave it behind.
    document.addEventListener("scroll", close, true);
    // A target removed from the page under the pointer gets no `pointerleave`.
    document.addEventListener("pointerover", outside, true);
  };
  const close = (): void => {
    show(false);
  };
  const outside = (event: PointerEvent): void => {
    if (!target.isConnected || !(event.target instanceof Node && target.contains(event.target))) close();
  };
  // A touch shows no tooltip: the finger hides it, and the tap acts at once.
  target.addEventListener("pointerenter", (event) => {
    if (event.pointerType !== "touch") show(true);
  });
  target.addEventListener("focus", () => {
    if (target.matches(":focus-visible")) show(true);
  });
  target.addEventListener("pointerleave", close);
  target.addEventListener("blur", close);
  target.addEventListener("pointerdown", close);
  target.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close();
  });
  return (next) => {
    tip.textContent = next;
  };
}
