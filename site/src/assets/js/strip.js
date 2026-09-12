import { PALETTE, ZONES, build, mix, rgb } from "./zones.js";

export function strip(el) {
  const zones = build(el, "b");
  // How far the light carries on each side of the pointer.
  const REACH = 6;

  const off = (zone) => {
    zone.style.removeProperty("--zone");
    zone.removeAttribute("data-lit");
  };

  // The window that is lit now, so a move writes the zones that change and
  // leaves the rest alone.
  let from = 0;
  let to = -1;
  const light = (index) => {
    const first = Math.max(0, index - REACH);
    const last = Math.min(ZONES - 1, index + REACH);
    for (let i = from; i <= to; i += 1) {
      if (i < first || i > last) off(zones[i]);
    }
    for (let i = first; i <= last; i += 1) {
      const distance = Math.abs(i - index);
      const fall = 1 - distance / (REACH + 1);
      const hue = mix(PALETTE[1], PALETTE[0], distance / (REACH + 1));
      zones[i].style.setProperty("--zone", rgb(hue, 0.25 + fall * 0.75));
      zones[i].dataset.lit = "";
    }
    from = first;
    to = last;
  };

  const clear = () => {
    for (let i = from; i <= to; i += 1) off(zones[i]);
    from = 0;
    to = -1;
  };

  // The box is read when the pointer arrives and when the layout can have
  // moved, never inside the move: reading it there forces a layout per event.
  let box = null;
  const measure = () => (box = el.getBoundingClientRect());
  // One frame in flight, holding the last position the pointer reached: a
  // burst of moves inside one frame paints once.
  let x = null;
  let frame = false;
  const paint = () => {
    frame = false;
    if (x === null) return;
    if (!box) measure();
    light(Math.max(0, Math.min(ZONES - 1, Math.floor(((x - box.left) / box.width) * ZONES))));
  };
  el.addEventListener("pointerenter", measure);
  el.addEventListener("pointermove", (event) => {
    x = event.clientX;
    if (frame) return;
    frame = true;
    requestAnimationFrame(paint);
  });
  el.addEventListener("pointerleave", () => {
    x = null;
    box = null;
    clear();
  });
  addEventListener("resize", () => (box = null), { passive: true });
  addEventListener("scroll", () => (box = null), { passive: true });

  light(Math.floor(ZONES / 2));
}
