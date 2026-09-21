import { PALETTE, mix, rgb } from "./zones.js";

// The hero rig: the two kinds of light the toolkit drives, one at a time. A
// strip carries one color per zone and runs a wave; a bulb carries one color
// for the whole device. The buttons are drawn here too, so a page with this
// file blocked shows an empty box and no dead control.

const SVG = "http://www.w3.org/2000/svg";

// The strip is drawn as one line of zones. The count is a drawing count and
// not a measurement: a real zone count belongs to a device file. It is high
// enough that a straight segment still follows the half turns.
const COUNT = 60;
// The gap between two zones, and the height of one, in viewBox units.
const GAP = 2;
const HEIGHT = 15;
// A light that is off. The shapes are opaque, so a dark zone reads as off
// over the beam in the hero and not as a hole in it.
const OFF = [26, 31, 43];

const STRIP_BOX = [420, 350];
// Three runs joined by two half turns. The turn radius is half the gap
// between the runs, so the arcs meet the runs without a corner. A wide turn
// keeps the angle between two zones small, and the zones touch through it.
const TRACK = "M 40 45 H 330 A 65 65 0 0 1 330 175 H 90 A 65 65 0 0 0 90 305 H 403";

const BULB_BOX = [150, 180];

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

// The hero ground is warm, so the lights answer with the cool half of the
// palette: magenta, violet, cyan.
const COOL = [PALETTE[2], PALETTE[3], PALETTE[4]];

// The wave the strip runs: a crest that travels along the zones. `u` is the
// place of the zone on the strip, from 0 to 1, and `t` is seconds.
function wave(u, t) {
  const crest = (Math.sin(u * 19 - t * 2.2) + 1) / 2;
  return [mix(COOL[1], COOL[2], crest), 0.16 + crest * 0.84];
}

// The bulb takes one color for the whole device, so it walks the ramp
// instead of holding a pattern.
function whole(t) {
  const point = (((t * 0.09) % 1) + 1) % 1 * COOL.length;
  const low = Math.floor(point) % COOL.length;
  const color = mix(COOL[low], COOL[(low + 1) % COOL.length], point % 1);
  return [color, 0.72 + ((Math.sin(t * 1.1) + 1) / 2) * 0.28];
}

function surface(box, id, classes) {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `0 0 ${box[0]} ${box[1]}`);
  svg.setAttribute("class", classes);
  svg.setAttribute("aria-hidden", "true");
  const lights = document.createElementNS(SVG, "g");
  lights.id = id;
  return [svg, lights];
}

// The light a shape throws: the same group again, blurred and added to what
// is under it, so one fill per shape paints the light and its glow.
function halo(svg, id) {
  const copy = document.createElementNS(SVG, "use");
  copy.setAttribute("href", `#${id}`);
  copy.setAttribute("class", "halo");
  svg.append(copy);
}

function strip(box) {
  const [svg, lights] = surface(STRIP_BOX, "rig-strip", "light strip");
  const track = document.createElementNS(SVG, "path");
  track.setAttribute("d", TRACK);
  track.setAttribute("class", "track");
  svg.append(track);

  const length = track.getTotalLength();
  // One zone covers its share of the strip, less the gap to the next one.
  const step = length / COUNT;
  const width = step - GAP;
  const zones = [];
  for (let i = 0; i < COUNT; i += 1) {
    const at = (i + 0.5) * step;
    const point = track.getPointAtLength(at);
    // The zone lies along the strip, so it turns with it. The tangent comes
    // from the two points around it, which holds through the half turns.
    const back = track.getPointAtLength(Math.max(0, at - step / 2));
    const ahead = track.getPointAtLength(Math.min(length, at + step / 2));
    const angle = (Math.atan2(ahead.y - back.y, ahead.x - back.x) * 180) / Math.PI;
    const zone = document.createElementNS(SVG, "rect");
    zone.setAttribute("x", (point.x - width / 2).toFixed(2));
    zone.setAttribute("y", (point.y - HEIGHT / 2).toFixed(2));
    zone.setAttribute("width", width.toFixed(2));
    zone.setAttribute("height", HEIGHT);
    zone.setAttribute("rx", "3");
    zone.setAttribute("transform", `rotate(${angle.toFixed(1)} ${point.x.toFixed(2)} ${point.y.toFixed(2)})`);
    lights.append(zone);
    zones.push(zone);
  }
  svg.append(lights);
  halo(svg, "rig-strip");
  box.append(svg);
  return zones;
}

function bulb(box) {
  const [svg, lights] = surface(BULB_BOX, "rig-bulb", "light bulb");

  // The glass, and the neck that carries it down to the cap.
  const glass = document.createElementNS(SVG, "path");
  glass.setAttribute(
    "d",
    "M 75 26 A 49 49 0 0 1 106 113 L 100 140 H 50 L 44 113 A 49 49 0 0 1 75 26 Z",
  );
  lights.append(glass);
  svg.append(lights);
  halo(svg, "rig-bulb");

  // The cap. It carries no light, so it is drawn over the glow.
  const cap = document.createElementNS(SVG, "g");
  cap.setAttribute("class", "cap");
  [145, 155].forEach((y) => {
    const ring = document.createElementNS(SVG, "rect");
    ring.setAttribute("x", "52");
    ring.setAttribute("y", String(y));
    ring.setAttribute("width", "46");
    ring.setAttribute("height", "8");
    ring.setAttribute("rx", "3");
    cap.append(ring);
  });
  const foot = document.createElementNS(SVG, "rect");
  foot.setAttribute("x", "60");
  foot.setAttribute("y", "165");
  foot.setAttribute("width", "30");
  foot.setAttribute("height", "7");
  foot.setAttribute("rx", "3");
  cap.append(foot);
  svg.append(cap);

  box.append(svg);
  return glass;
}

function stage(el, kind) {
  const box = document.createElement("div");
  box.className = `light-box ${kind}-box`;
  el.append(box);
  return box;
}

function controls(el, pick) {
  const bar = document.createElement("div");
  bar.className = "devices";
  bar.setAttribute("role", "group");
  bar.setAttribute("aria-label", "Device");
  const buttons = ["LED strip", "Lightbulb"].map((name, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = name;
    button.setAttribute("aria-pressed", String(index === 0));
    button.addEventListener("click", () => {
      buttons.forEach((other, i) => other.setAttribute("aria-pressed", String(i === index)));
      pick(index);
    });
    bar.append(button);
    return button;
  });
  el.append(bar);
}

export function rig(el) {
  const boxes = [stage(el, "strip"), stage(el, "bulb")];
  const zones = strip(boxes[0]);
  const glass = bulb(boxes[1]);
  // The device that is off screen is not painted, so the rig writes one
  // device per frame whatever the reader picked.
  let shown = 0;
  boxes[1].hidden = true;

  const paint = (t) => {
    if (shown === 0) {
      zones.forEach((zone, i) => {
        const [color, level] = wave(i / (zones.length - 1), t);
        zone.setAttribute("fill", rgb(mix(OFF, color, level)));
      });
      return;
    }
    const [color, level] = whole(t);
    glass.setAttribute("fill", rgb(mix(OFF, color, level)));
  };

  controls(el, (index) => {
    shown = index;
    boxes.forEach((box, i) => (box.hidden = i !== index));
    // The frames stop in a hidden tab and under a reduced-motion setting, so
    // the device that arrives is painted here and not by the next frame.
    paint(performance.now() / 1000);
  });

  paint(0);
  if (reduced) return;

  // A frame in a background tab costs battery and paints nothing anybody
  // sees, so the rig runs only while it is on screen and the tab is.
  let frame = null;
  let visible = true;
  const step = (now) => {
    paint(now / 1000);
    frame = requestAnimationFrame(step);
  };
  const start = () => {
    if (frame || !visible || document.hidden) return;
    frame = requestAnimationFrame(step);
  };
  const stop = () => {
    cancelAnimationFrame(frame);
    frame = null;
  };
  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    if (visible) start(); else stop();
  }).observe(el);
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));
  start();
}
