import { PALETTE, ZONES, build, mix, rgb } from "./zones.ts";

// How far the light carries on each side of the pointer.
const REACH = 6;

function off(zone: HTMLElement): void {
  zone.style.removeProperty("--zone");
  delete zone.dataset.lit;
}

// The window that is lit now, so a move writes the zones that change and
// leaves the rest alone.
function lamp(zones: HTMLElement[]): { light: (index: number) => void; clear: () => void } {
  let from = 0;
  let to = -1;
  const light = (index: number) => {
    const first = Math.max(0, index - REACH);
    const last = Math.min(ZONES - 1, index + REACH);
    for (let i = from; i <= to; i += 1) {
      const zone = zones[i];
      if (zone && (i < first || i > last)) off(zone);
    }
    for (let i = first; i <= last; i += 1) {
      const distance = Math.abs(i - index);
      const fall = 1 - distance / (REACH + 1);
      const hue = mix(PALETTE[1], PALETTE[0], distance / (REACH + 1));
      const zone = zones[i];
      if (!zone) continue;
      zone.style.setProperty("--zone", rgb(hue, 0.25 + fall * 0.75));
      zone.dataset.lit = "";
    }
    from = first;
    to = last;
  };
  const clear = () => {
    for (const zone of zones.slice(from, to + 1)) off(zone);
    from = 0;
    to = -1;
  };
  return { light, clear };
}

export function strip(el: HTMLElement): void {
  const { light, clear } = lamp(build(el, "b"));

  // The box is read when the pointer arrives and when the layout can have
  // moved, never inside the move: reading it there forces a layout per event.
  let box: DOMRect | null = null;
  const measure = () => (box = el.getBoundingClientRect());
  const forget = () => {
    box = null;
  };
  // One frame in flight, holding the last position the pointer reached: a
  // burst of moves inside one frame paints once.
  let x: number | null = null;
  let frame = false;
  const paint = () => {
    frame = false;
    if (x === null) return;
    const { left, width } = box ?? measure();
    light(Math.max(0, Math.min(ZONES - 1, Math.floor(((x - left) / width) * ZONES))));
  };
  el.addEventListener("pointerenter", () => {
    measure();
  });
  el.addEventListener("pointermove", (event) => {
    x = event.clientX;
    if (frame) return;
    frame = true;
    requestAnimationFrame(paint);
  });
  el.addEventListener("pointerleave", () => {
    x = null;
    forget();
    clear();
  });
  addEventListener("resize", forget, { passive: true });
  addEventListener("scroll", forget, { passive: true });

  light(Math.floor(ZONES / 2));
}
