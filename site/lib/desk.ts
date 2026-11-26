// The six faders of a `full` fixture, drawn as a lighting desk.
// `src/assets/ts/desk.ts` draws what the faders send in the lamp above them.
// Without JavaScript the faders still move.

import type { T } from "./locales.ts";

// [channel, string key, data key, start value]
const FADERS: [number, string, string, number][] = [
  [1, "desk.dimmer", "dim", 210],
  [2, "desk.mode", "mode", 0],
  [3, "desk.white", "white", 0],
  [4, "desk.red", "r", 255],
  [5, "desk.green", "g", 61],
  [6, "desk.blue", "b", 0],
];

/** The desk markup. The faders are `<input type="range">`, 0 to 255 as on the wire. */
export function deskHtml(t: T): string {
  const faders = FADERS.map(([channel, label, key, value]) => {
    const name = t(label);
    return `<div class="fader ${key}">`
    + `<b aria-hidden="true">${channel}</b>`
    + `<input type="range" min="0" max="255" value="${value}" data-channel="${key}" aria-label="${t("desk.channel", { n: channel, name })}">`
    + `<span aria-hidden="true">${name}</span><span class="value" aria-hidden="true">${value}</span></div>`;
  }).join("\n      ");
  return `<div class="desk" data-desk>
      <div class="desk-lamp" aria-hidden="true"></div>
      <div class="desk-faders">
      ${faders}
      </div>
    </div>`;
}
