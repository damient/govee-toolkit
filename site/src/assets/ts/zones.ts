
// How many zones the decoration draws. This is a drawing count and not a
// measurement: a real zone count belongs to a device file.
export const ZONES = 42;

/** One color as `[r, g, b]`, each 0–255. */
export type Rgb = [number, number, number];

export const PALETTE: [Rgb, Rgb, ...Rgb[]] = [
  [255, 61, 0],
  [255, 179, 92],
  [255, 94, 168],
  [141, 107, 255],
  [69, 227, 208],
];

export function build(el: HTMLElement, tag: string): HTMLElement[] {
  const zones: HTMLElement[] = [];
  for (let i = 0; i < ZONES; i += 1) {
    zones.push(el.appendChild(document.createElement(tag)));
  }
  return zones;
}

export function rgb([r, g, b]: Rgb, alpha = 1): string {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function mix(a: Rgb, b: Rgb, t: number): Rgb {
  const at = (from: number, to: number) => Math.round(from + (to - from) * t);
  return [at(a[0], b[0]), at(a[1], b[1]), at(a[2], b[2])];
}
