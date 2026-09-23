// A drawing count, not a measurement.
const ZONES = 42;

/** One color as `[r, g, b]`, each 0–255. */
type Rgb = [number, number, number];

const PALETTE: [Rgb, Rgb] = [
  [255, 61, 0],
  [255, 179, 92],
];

function build(el: HTMLElement, tag: string): HTMLElement[] {
  const zones: HTMLElement[] = [];
  for (let i = 0; i < ZONES; i += 1) {
    const zone = document.createElement(tag);
    el.append(zone);
    zones.push(zone);
  }
  return zones;
}

function rgb([r, g, b]: Rgb, alpha = 1): string {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function mix(a: Rgb, b: Rgb, t: number): Rgb {
  const at = (from: number, to: number) => Math.round(from + (to - from) * t);
  return [at(a[0], b[0]), at(a[1], b[1]), at(a[2], b[2])];
}

const REACH = 6;

function off(zone: HTMLElement): void {
  zone.style.removeProperty("--zone");
  delete zone.dataset.lit;
}

// A move writes only the zones that change.
function lamp(zones: HTMLElement[]): (index: number) => void {
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
  return light;
}

export function strip(el: HTMLElement): void {
  const light = lamp(build(el, "b"));
  const rest = Math.floor(ZONES / 2);

  // Never read the box inside the move: that forces a layout per event.
  let box: DOMRect | null = null;
  const measure = () => (box = el.getBoundingClientRect());
  const forget = () => {
    box = null;
  };
  // A burst of moves inside one frame paints once.
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
    light(rest);
  });
  addEventListener("resize", forget, { passive: true });
  addEventListener("scroll", forget, { passive: true });

  light(rest);
}
