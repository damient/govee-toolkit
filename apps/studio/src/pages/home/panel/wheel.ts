import { t } from "../../../core/i18n.ts";

/** A hue in degrees and a saturation from 0 to 1, at full value. */
interface HueSat {
  hue: number;
  sat: number;
}

const HUE_STEP = 5;
const SAT_STEP = 0.05;

const channel = (value: number): string =>
  Math.round(value * 255)
    .toString(16)
    .padStart(2, "0");

function toHex({ hue, sat }: HueSat): string {
  const f = (n: number): number => {
    const k = (n + hue / 60) % 6;
    return 1 - sat * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return `#${channel(f(5))}${channel(f(3))}${channel(f(1))}`;
}

/** The hue and the saturation of `hex`. The value is dropped: the brightness slider sets it. */
function fromHex(hex: string): HueSat {
  const [r = 0, g = 0, b = 0] = [1, 3, 5].map((at) => Number.parseInt(hex.slice(at, at + 2), 16) / 255);
  const max = Math.max(r, g, b);
  const span = max - Math.min(r, g, b);
  if (span === 0) return { hue: 0, sat: 0 };
  let hue = max === r ? (g - b) / span : max === g ? 2 + (b - r) / span : 4 + (r - g) / span;
  hue = (hue * 60 + 360) % 360;
  return { hue, sat: span / max };
}

export interface Wheel {
  node: HTMLElement;
  /** Move the knob to `hex`, with no pick. */
  set(hex: string): void;
}

function place(node: HTMLElement, knob: HTMLElement, at: HueSat): void {
  const angle = (at.hue * Math.PI) / 180;
  // The distance is the square of the saturation: panel.css draws the white with the same curve.
  const distance = at.sat * at.sat;
  knob.style.left = `${50 + 50 * distance * Math.sin(angle)}%`;
  knob.style.top = `${50 - 50 * distance * Math.cos(angle)}%`;
  const hex = toHex(at);
  knob.style.background = hex;
  node.setAttribute("aria-valuenow", String(Math.round(at.hue)));
  node.setAttribute("aria-valuetext", hex);
}

/** The point of `event` on the wheel `node`. Out of the circle, the saturation stays at 1. The saturation is the square root of the distance. */
function pointed(node: HTMLElement, event: PointerEvent): HueSat {
  const box = node.getBoundingClientRect();
  const radius = box.width / 2;
  const dx = event.clientX - box.left - radius;
  const dy = event.clientY - box.top - radius;
  const hue = ((Math.atan2(dx, -dy) * 180) / Math.PI + 360) % 360;
  return { hue, sat: Math.sqrt(Math.min(1, Math.hypot(dx, dy) / radius)) };
}

/** The point after the arrow `key`, or `null` for another key. */
function stepped(key: string, at: HueSat): HueSat | null {
  const turn = { ArrowLeft: -HUE_STEP, ArrowRight: HUE_STEP }[key] ?? 0;
  const grow = { ArrowDown: -SAT_STEP, ArrowUp: SAT_STEP }[key] ?? 0;
  if (turn === 0 && grow === 0) return null;
  return { hue: (at.hue + turn + 360) % 360, sat: Math.min(1, Math.max(0, at.sat + grow)) };
}

function shell(): { node: HTMLElement; knob: HTMLElement } {
  const node = document.createElement("div");
  node.className = "panel-wheel";
  node.tabIndex = 0;
  node.setAttribute("role", "slider");
  node.setAttribute("aria-label", t("panel.wheel"));
  node.setAttribute("aria-valuemin", "0");
  node.setAttribute("aria-valuemax", "359");
  const knob = document.createElement("span");
  knob.className = "panel-knob";
  node.append(knob);
  return { node, knob };
}

/**
 * A color wheel: the hue around, the saturation from the center out. A drag
 * runs `pick` on each move. The arrows change the hue (left, right) and the
 * saturation (up, down).
 */
export function wheel(pick: (hex: string) => void): Wheel {
  const { node, knob } = shell();
  let at: HueSat = { hue: 0, sat: 0 };
  const choose = (next: HueSat): void => {
    at = next;
    place(node, knob, at);
    pick(toHex(at));
  };
  node.addEventListener("pointerdown", (event) => {
    node.setPointerCapture(event.pointerId);
    choose(pointed(node, event));
  });
  node.addEventListener("pointermove", (event) => {
    if (node.hasPointerCapture(event.pointerId)) choose(pointed(node, event));
  });
  node.addEventListener("keydown", (event) => {
    const next = stepped(event.key, at);
    if (next === null) return;
    event.preventDefault();
    choose(next);
  });
  place(node, knob, at);
  return {
    node,
    set(hex) {
      at = fromHex(hex);
      place(node, knob, at);
    },
  };
}
