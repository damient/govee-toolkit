// The "DMX" section of a model page: the channel table an operator patches a
// desk with. `dist/catalog.json` carries the table, derived by
// `govee-toolkit-dmx` from the device file, so this page and the DMX bridge
// cannot disagree. Nothing below derives a channel of its own.

import { DASH, escapeAttr, escapeHtml } from "./html.ts";
import { icon, slotIcon } from "./icons.ts";
import type { Channel, Device, Personality, Range } from "./types.ts";

// A slot kind as a reader reads it. An unknown kind falls back to its own
// name, so a new slot reaches the page without a change here.
const SLOTS = new Map([
  ["dimmer", "Dimmer"],
  ["white_temp", "White"],
  ["mode", "Mode"],
]);


// A slot that drives a capability carries that capability's mark, so a reader
// finds the same icon and the same color as in the lists above. `mode` drives
// none and carries a mark of its own.
const SLOT_CAPS = new Map([
  ["dimmer", "brightness"],
  ["white_temp", "colortemp"],
  ["zone", "segments"],
]);

// A color channel carries the component itself rather than a drawing of one.
const swatch = (component: string | undefined): string =>
  `<svg class="dmx-swatch" data-component="${escapeAttr(component ?? "")}" viewBox="0 0 24 24"
            aria-hidden="true"><rect x="2" y="2" width="20" height="20" rx="5" fill="currentColor"/></svg>`;

function mark(channel: Channel): string {
  if (channel.slot === "color") return swatch(channel.component);
  const cap = SLOT_CAPS.get(channel.slot);
  return cap === undefined ? slotIcon(channel.slot) : icon(cap);
}

/**
 * What one channel drives. A color names its component, and a channel this
 * model reaches through nothing says so: it holds its place in the table and
 * drives nothing.
 */
function slotLabel(channel: Channel): string {
  const name = escapeHtml(
    channel.slot === "color"
      ? capitalize(channel.component ?? "color")
      : (SLOTS.get(channel.slot) ?? channel.slot.replaceAll("_", " ")),
  );
  if (channel.unreached !== true) return name;
  return `${name}
            <span class="dmx-order">this model drives nothing here</span>`;
}

const capitalize = (word: string): string => word.charAt(0).toUpperCase() + word.slice(1);

// A band of slots, as a desk numbers them. `catalog.json` carries the bands,
// so the page never works out what a slot means.
const slots = ([low, high]: Range): string => (low === high ? String(low) : `${low} – ${high}`);

const valueList = (lines: string[]): string => `<ul class="dmx-values">
              ${lines.join("\n              ")}
            </ul>`;

function values(channel: Channel): string {
  const bands = channel.values ?? [];
  if (bands.length === 0) return DASH;
  return valueList(
    bands.map(
      (band) =>
        `<li><span class="dmx-slots">${escapeHtml(slots(band.slots))}</span>
              ${escapeHtml(band.label)}</li>`,
    ),
  );
}

// One channel as a column of a desk: the number on top, then the slot, then
// the bands. Every column takes one width, as the faders of a desk do.
function column(head: string, channel: Channel, slot: string, cell: string): string {
  const unreached = channel.unreached === true ? " data-unreached" : "";
  return `<li class="dmx-column"${unreached}>
              <span class="dmx-channel">${head}${mark(channel)}</span>
              <span class="dmx-slot">${slot}</span>
              <div class="dmx-bands">${cell}</div>
            </li>`;
}

// A zone triple repeats once per zone, and a 132-zone strip holds 396 of them.
// The run reads as one column: the operator needs where it starts, how wide it
// is and the order inside it.
function zoneColumn(run: [Channel, ...Channel[]]): string {
  const first = run[0];
  const last = run.at(-1) ?? first;
  const zones = new Set(run.map((c) => c.zone)).size;
  const components = [...new Set(run.map((c) => c.component))].join(", ");
  const head = `${first.offset} – ${last.offset}`;
  const cell = valueList([
    "<li>three channels each</li>",
    `<li>${escapeHtml(components)}, in zone order</li>`,
  ]);
  return column(head, first, `${zones} zones`, cell);
}

/** Every channel as a column, with the zone channels folded into one. */
function columns(channels: Channel[]): string[] {
  const out: string[] = [];
  for (let at = 0; at < channels.length; at += 1) {
    const channel = channels[at];
    if (!channel) break;
    if (channel.slot !== "zone") {
      out.push(column(String(channel.offset), channel, slotLabel(channel), values(channel)));
      continue;
    }
    const start = at;
    while (channels[at + 1]?.slot === "zone") at += 1;
    out.push(zoneColumn([channel, ...channels.slice(start + 1, at + 1)]));
  }
  return out;
}

const paneId = (entry: Personality): string => `pane-dmx-${entry.personality}`;
const tabId = (entry: Personality): string => `tab-dmx-${entry.personality}`;

// One tab per personality. The width sits in the tab because that is what an
// operator compares before opening one.
function tab(entry: Personality, at: number): string {
  const width = entry.channels ? `${entry.width} channels` : "no table";
  return `<button type="button" role="tab" id="${escapeAttr(tabId(entry))}"
            aria-controls="${escapeAttr(paneId(entry))}" data-personality="${escapeAttr(entry.personality)}"
            aria-selected="${at === 0}" tabindex="${at === 0 ? "0" : "-1"}">${escapeHtml(entry.personality)}<span
            class="dmx-tab-width">${escapeHtml(width)}</span></button>`;
}

/** One personality: its table, or the reason it serves none. */
function pane(entry: Personality, at: number): string {
  const frame = `class="dmx-pane" role="tabpanel" id="${escapeAttr(paneId(entry))}"
          aria-labelledby="${escapeAttr(tabId(entry))}" tabindex="0"
          data-personality="${escapeAttr(entry.personality)}"${at === 0 ? "" : " hidden"}`;
  if (!entry.channels) {
    return `<div ${frame}>
          <p class="mode-note">${escapeHtml(entry.error ?? "No channel table.")}</p>
        </div>`;
  }
  return `<div ${frame}>
          <div class="dmx-frame">
            <ol class="dmx-desk" aria-label="Channels">
            ${columns(entry.channels).join("\n            ")}
            </ol>
          </div>
        </div>`;
}

/**
 * The section, or an empty string where the device file gives the bridge no
 * way to drive one channel of any personality. The caller places it.
 */
export function dmx(device: Device): string {
  const entries = device.dmx?.personalities ?? [];
  if (entries.length === 0) return "";
  return `<h2 id="dmx">DMX</h2>
      <div class="dmx" data-dmx>
        <div class="tabs tabs-dmx" role="tablist" aria-label="Personality">
          ${entries.map((entry, at) => tab(entry, at)).join("\n          ")}
        </div>
        ${entries.map((entry, at) => pane(entry, at)).join("\n        ")}
      </div>`;
}
