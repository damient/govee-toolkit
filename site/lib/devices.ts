// Nothing here names a model or a command: the device files own them.

import { type ListView, categoryOptions, members } from "./categories.ts";
import { MODES } from "./config.ts";
import { DASH, escapeAttr, escapeHtml, fill } from "./html.ts";
import { icon, shapeIcon, sharesMark } from "./icons.ts";
import { infoBadge } from "./listed.ts";
import { dmxMark } from "./mode-badge.ts";
import type { T } from "./locales.ts";
import type { Device, Mode, Support } from "./types.ts";

// The order a reader looks for, not the order the catalog holds.
const CAPS = ["power", "color", "colortemp", "brightness", "music", "segments", "segment_brightness"];

const CAP_ORDER = new Map(CAPS.map((key, at) => [key, at]));

/** Sort rank of a capability key. An unknown key sorts last. */
export const order = (key: string): number => CAP_ORDER.get(key) ?? CAPS.length;

export const support = (device: Device, mode: Mode): Support => device.modes?.[mode]?.support ?? "unknown";

const REACHES = new Set<Support>(["full", "capped", "partial"]);

// An `unknown` mode that declares capabilities counts: a listed model has no other.
export const reached = (d: Device): Mode[] => MODES.filter((m) => REACHES.has(support(d, m))
  || (support(d, m) === "unknown" && (d.modes?.[m]?.capabilities?.length ?? 0) > 0));

export const CHECK = '<svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"'
  + ' stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'
  + '<path d="M4.5 12.6 9.4 17.5 19.5 7"/></svg>';
const pill = (value: Support, t: T): string =>
  `<span class="pill pill-${value}">${value === "unknown" ? "?" : escapeHtml(t.or(`pill.${value}`, value))}</span>`;

/** A key such as `segment_brightness` as a reader reads it. */
export const label = (key: string, t: T): string => t.or(`cap.${key}`, key.replaceAll("_", " "));

/** The device list of one view: every model, the tested ones or one category. */
export function renderIndex(template: string, devices: Device[], view: ListView, t: T): string {
  const rows = members(devices, view);
  const heading = view.kind === "all"
    ? t("list.heading")
    : view.kind === "tested" ? t("list.heading_tested") : t("list.heading_category", { name: t.category(view.name) });
  const scope = view.kind === "all"
    ? t("list.scope")
    : view.kind === "tested" ? t("list.scope_tested") : t("list.scope_category", { name: t.category(view.name) });
  return fill(template, {
    devices_rows: rows.map((d) => row(d, t)).join("\n"),
    devices_categories: categoryOptions(devices, view, t),
    caps_legend: capsLegend(rows, t),
    devices_heading: escapeHtml(heading),
    devices_scope: escapeAttr(scope),
  });
}

// The capabilities that any mode reaches, one mark each. A model that no mode
// reaches shows its state instead.
function capsCell(d: Device, t: T): string {
  // Two capabilities that share a mark show once. The model page names both.
  const caps = [...new Set(reached(d).flatMap((m) => d.modes?.[m]?.capabilities ?? []))]
    .filter((c) => !sharesMark(c))
    .toSorted((a, b) => order(a) - order(b));
  const dmx = hasDmx(d) ? dmxMark() : "";
  if (caps.length === 0) {
    const none = MODES.every((m) => support(d, m) === "none");
    return `<div class="caps-cell">${none ? DASH : pill("unknown", t)}${dmx}</div>`;
  }
  const items = caps
    .map(
      (c) => `<li class="cap-chip" data-tip="${escapeAttr(label(c, t))}">${icon(c)}`
        + `<span class="visually-hidden">${escapeHtml(label(c, t))}</span></li>`,
    )
    .join("");
  return `<div class="caps-cell"><ul class="caps-row">${items}</ul>${dmx}</div>`;
}

// A device file that declares a new capability adds its row to the legend.
function capsLegend(devices: Device[], t: T): string {
  const keys = new Set(devices.flatMap((d) => Object.keys(d.capabilities ?? {})));
  return [...keys]
    .filter((key) => icon(key) !== "" && !sharesMark(key))
    .toSorted((a, b) => order(a) - order(b))
    .map((key) => `<li>${icon(key)}${escapeHtml(label(key, t))}</li>`)
    .join("\n        ");
}

export function hasDmx(d: Device): boolean {
  return (d.dmx?.personalities?.length ?? 0) > 0;
}

function row(d: Device, t: T): string {
  const names = [d.sku, d.name, ...(d.aliases ?? [])].join(" ").toLowerCase();
  const mark = d.listing === undefined ? "" : ` ${infoBadge(t)}`;
  return `          <tr class="has-page" data-search="${escapeAttr(names)}">
            <td class="shape">${shapeIcon(d.shape ?? d.family)}</td>
            <th scope="row"><a class="model-name" href="{{base}}devices/${escapeAttr(d.sku)}/">${escapeHtml(d.name)}</a>${mark}
              <button type="button" class="sku" data-sku="${escapeAttr(d.sku)}">${escapeHtml(d.sku)}</button></th>
            <td>${capsCell(d, t)}</td>
          </tr>`;
}
