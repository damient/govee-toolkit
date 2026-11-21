// The device list. Nothing here names a model or a command: the device files
// own them. `device-page.ts` builds the page of one model.

import { categoryOptions } from "./categories.ts";
import { MODES } from "./config.ts";
import { DASH, escapeAttr, escapeHtml, fill } from "./html.ts";
import { icon, shapeIcon, sharesMark } from "./icons.ts";
import { infoBadge } from "./listed.ts";
import type { T } from "./locales.ts";
import type { Device, Mode, Support } from "./types.ts";

// The order a reader looks for, not the order the catalog holds. `cap.<key>`
// in `content/i18n/` names each one.
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

/** The device list. `current` names the category of a category page, which
 * lists the rows of that category alone. */
export function renderIndex(template: string, devices: Device[], current: string | undefined, t: T): string {
  const rows = current === undefined ? devices : devices.filter((d) => d.category === current);
  return fill(template, {
    devices_rows: rows.map((d) => row(d, t)).join("\n"),
    devices_categories: categoryOptions(devices, current, t),
    caps_legend: capsLegend(rows, t),
    check_mark: CHECK,
    devices_heading: current === undefined ? escapeHtml(t("list.heading")) : escapeHtml(t("list.heading_category", { name: t.category(current) })),
    devices_scope: escapeAttr(current === undefined ? t("list.scope") : t("list.scope_category", { name: t.category(current) })),
  });
}

// A mode that reaches no capability shows its state instead.
function modeCell(d: Device, mode: Mode, t: T): string {
  const state = support(d, mode);
  // Two capabilities that share a mark show once. The model page names both.
  const caps = [...(d.modes?.[mode]?.capabilities ?? [])]
    .filter((c) => !sharesMark(c))
    .toSorted((a, b) => order(a) - order(b));
  if (caps.length === 0) {
    if (state === "none") return DASH;
    return pill(state, t);
  }
  const items = caps
    .map(
      (c) => `<li class="cap-chip" data-tip="${escapeAttr(label(c, t))}">${icon(c)}`
        + `<span class="visually-hidden">${escapeHtml(label(c, t))}</span></li>`,
    )
    .join("");
  return `<ul class="caps-row">${items}</ul>`;
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

function dmxCell(d: Device): string {
  if (!hasDmx(d)) return DASH;
  return `<span class="dmx-yes">${CHECK}<span class="visually-hidden">DMX</span></span>`;
}

function row(d: Device, t: T): string {
  const cells = MODES.map((m) => `<td>${modeCell(d, m, t)}</td>`).join("")
    + `<td>${dmxCell(d)}</td>`;
  const names = [d.sku, d.name, ...(d.aliases ?? [])].join(" ").toLowerCase();
  const mark = d.listing === undefined ? "" : ` ${infoBadge(t)}`;
  return `          <tr class="has-page" data-search="${escapeAttr(names)}">
            <th scope="row"><a href="{{base}}devices/${escapeAttr(d.sku)}/">${escapeHtml(d.sku)}</a></th>
            <td class="shape">${shapeIcon(d.shape ?? d.family)}</td>
            <td>${escapeHtml(d.name)}${mark}</td>${cells}
          </tr>`;
}
