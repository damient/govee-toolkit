// The "DMX" section of a model page: the channel table an operator patches a
// desk with. `dist/catalog.json` carries the table, derived by
// `govee-toolkit-dmx` from the device file, so this page and the Art-Net node
// cannot disagree. Nothing below derives a channel of its own.

import { escapeAttr, escapeHtml } from "./html.mjs";
import { icon, slotIcon } from "./icons.mjs";

// A slot kind as a reader reads it. An unknown kind falls back to its own
// name, so a new slot reaches the page without a change here.
const SLOTS = new Map([
  ["dimmer", "Dimmer"],
  ["white_temp", "White temperature"],
  ["control", "Control"],
]);

const DASH = '<span class="muted">—</span>';

// A slot that drives a capability carries that capability's mark, so a reader
// finds the same icon and the same color as in the lists above. `control`
// drives none and carries a mark of its own.
const SLOT_CAPS = new Map([
  ["dimmer", "brightness"],
  ["white_temp", "colortemp"],
  ["zone", "segments"],
]);

// A color channel carries the component itself rather than a drawing of one.
const swatch = (component) =>
  `<svg class="dmx-swatch" data-component="${escapeAttr(component ?? "")}" viewBox="0 0 24 24"
            aria-hidden="true"><rect x="2" y="2" width="20" height="20" rx="5" fill="currentColor"/></svg>`;

function mark(channel) {
  if (channel.slot === "color") return swatch(channel.component);
  const cap = SLOT_CAPS.get(channel.slot);
  return cap ? icon(cap) : slotIcon(channel.slot);
}

/** What one channel drives. A color names its component. */
function slotLabel(channel) {
  if (channel.slot === "color") return capitalize(channel.component ?? "color");
  return SLOTS.get(channel.slot) ?? channel.slot.replace(/_/g, " ");
}

const capitalize = (word) => word.charAt(0).toUpperCase() + word.slice(1);

const range = (channel) =>
  channel.range ? `${channel.range[0]} – ${channel.range[1]}` : DASH;

const steps = (channel) => (channel.steps ? String(channel.steps) : DASH);

function row(head, channel, slot, scaled) {
  return `<tr><th scope="row">${head}</th><td class="dmx-mark">${mark(channel)}</td><td>${slot}</td>
            <td>${range(scaled ?? {})}</td><td>${steps(scaled ?? {})}</td></tr>`;
}

// A zone triple repeats once per zone, and a 132-zone strip holds 396 of them.
// The run reads as one row: the operator needs where it starts, how wide it is
// and the order inside it.
function zoneRow(run) {
  const first = run[0];
  const last = run[run.length - 1];
  const zones = new Set(run.map((c) => c.zone)).size;
  const components = [...new Set(run.map((c) => c.component))].join(", ");
  const head = `${first.offset} – ${last.offset}`;
  const body = `${zones} zones, three channels each
            <span class="dmx-order">${escapeHtml(components)}, in zone order</span>`;
  return row(head, first, body, null);
}

/** Every channel as a row, with the zone channels folded into one. */
function rows(channels) {
  const out = [];
  for (let at = 0; at < channels.length; at += 1) {
    const channel = channels[at];
    if (channel.slot !== "zone") {
      out.push(row(String(channel.offset), channel, slotLabel(channel), channel));
      continue;
    }
    const start = at;
    while (at + 1 < channels.length && channels[at + 1].slot === "zone") at += 1;
    out.push(zoneRow(channels.slice(start, at + 1)));
  }
  return out;
}

const paneId = (entry) => `pane-dmx-${entry.personality}`;
const tabId = (entry) => `tab-dmx-${entry.personality}`;

// One tab per personality. The width sits in the tab because that is what an
// operator compares before opening one.
function tab(entry, at) {
  const width = entry.channels ? `${entry.width} channels` : "no table";
  return `<button type="button" role="tab" id="${escapeAttr(tabId(entry))}"
            aria-controls="${escapeAttr(paneId(entry))}" data-personality="${escapeAttr(entry.personality)}"
            aria-selected="${at === 0}" tabindex="${at === 0 ? "0" : "-1"}">${escapeHtml(entry.personality)}<span
            class="dmx-tab-width">${escapeHtml(width)}</span></button>`;
}

/** One personality: its table, or the reason it serves none. */
function pane(entry, at) {
  const frame = `class="dmx-pane" role="tabpanel" id="${escapeAttr(paneId(entry))}"
          aria-labelledby="${escapeAttr(tabId(entry))}" tabindex="0"
          data-personality="${escapeAttr(entry.personality)}"${at === 0 ? "" : " hidden"}`;
  if (!entry.channels) {
    return `<div ${frame}>
          <p class="mode-note">${escapeHtml(entry.error ?? "No channel table.")}</p>
        </div>`;
  }
  return `<div ${frame}>
          <div class="tablewrap">
            <table class="matrix matrix-dmx">
              <thead><tr><th scope="col">Channel</th>
                <th scope="col"><span class="visually-hidden">Mark</span></th>
                <th scope="col">Slot</th>
                <th scope="col">Range</th><th scope="col">Steps</th></tr></thead>
              <tbody>
            ${rows(entry.channels).join("\n            ")}
              </tbody>
            </table>
          </div>
        </div>`;
}

/**
 * The section, or an empty string where the device file gives the bridge no
 * way to drive one channel of any personality. The caller places it.
 */
export function dmx(device) {
  const entries = device.dmx?.personalities ?? [];
  if (!entries.length) return "";
  return `<h2 id="dmx">DMX</h2>
      <div class="dmx" data-dmx>
        <div class="tabs tabs-dmx" role="tablist" aria-label="Personality">
          ${entries.map(tab).join("\n          ")}
        </div>
        ${entries.map(pane).join("\n        ")}
      </div>`;
}
