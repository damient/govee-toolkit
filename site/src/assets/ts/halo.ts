// The light follows the pointer: `halo.css` and the `.btn` rules read the
// position from `--mx` and `--my`. A touch screen has no hover, so the
// elements keep their neutral ring.
const hover = (): boolean => matchMedia("(hover: hover)").matches;

const place = (element: HTMLElement, event: PointerEvent): void => {
  const box = element.getBoundingClientRect();
  element.style.setProperty("--mx", `${event.clientX - box.left}px`);
  element.style.setProperty("--my", `${event.clientY - box.top}px`);
};

const clear = (element: HTMLElement): void => {
  element.style.removeProperty("--mx");
  element.style.removeProperty("--my");
};

/** Every card of a grid reads the pointer, so the light of one card spills
 * onto its neighbours. */
export function halo(grid: HTMLElement): void {
  if (!hover()) return;
  const cards = [...grid.children].filter((card): card is HTMLElement => card instanceof HTMLElement);
  grid.addEventListener("pointermove", (event) => {
    for (const card of cards) place(card, event);
  });
  grid.addEventListener("pointerleave", () => {
    for (const card of cards) clear(card);
  });
}

/** One button reads the pointer while the pointer is on it. */
export function glow(button: HTMLElement): void {
  if (!hover()) return;
  button.addEventListener("pointermove", (event) => {
    place(button, event);
  });
  button.addEventListener("pointerleave", () => {
    clear(button);
  });
}
