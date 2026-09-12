import { PALETTE, ZONES, build, mix, rgb } from "./zones.js";

// The rope repaints on a timer, so a reader who asks for less motion gets the
// first paint and nothing after it.
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

export function rope(el) {
  const zones = build(el, "i");

  const paint = (offset) => {
    zones.forEach((zone, i) => {
      const position = (i / ZONES) * (PALETTE.length - 1) + offset;
      const low = Math.floor(position) % PALETTE.length;
      const high = (low + 1) % PALETTE.length;
      const color = mix(PALETTE[low], PALETTE[high], position % 1);
      zone.style.setProperty("--zone", rgb(color, 0.85));
    });
  };

  paint(0);
  if (reduced) return;

  // A timer in a background tab costs battery and paints nothing anybody
  // sees, so the rope repaints only while it is on screen and the tab is.
  let offset = 0;
  let timer = null;
  let visible = true;
  const start = () => {
    if (timer || !visible || document.hidden) return;
    timer = setInterval(() => {
      offset += 0.12;
      paint(offset);
    }, 900);
  };
  const stop = () => {
    clearInterval(timer);
    timer = null;
  };
  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    if (visible) start(); else stop();
  }).observe(el);
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));
  start();
}
