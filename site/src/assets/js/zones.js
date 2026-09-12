// The zones the decoration draws, and the color it paints them.

// How many zones the decoration draws. This is a drawing count and not a
// measurement: a real zone count belongs to a device file.
export const ZONES = 42;

export const PALETTE = [
  [255, 61, 0],
  [255, 179, 92],
  [255, 94, 168],
  [141, 107, 255],
  [69, 227, 208],
];

export function build(el, tag) {
  const zones = [];
  for (let i = 0; i < ZONES; i += 1) {
    zones.push(el.appendChild(document.createElement(tag)));
  }
  return zones;
}

export function rgb([r, g, b], alpha = 1) {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function mix(a, b, t) {
  return a.map((value, i) => Math.round(value + (b[i] - value) * t));
}
