// The "What you can send" section of a model page: one block per action the
// device file serves, with the bounds of this model and the same action in
// each language. The reference page renders those same entries with the
// generic values, so the two pages cannot drift. See ../README.md.

import { MODES } from "./config.ts";
import { examples } from "./examples.ts";
import { escapeAttr, escapeHtml, filled } from "./html.ts";
import { langBlock } from "./languages.ts";
import { modeBadge, modeMark } from "./mode-badge.ts";
import type { Device, Mode, Range, RefEntry, Reference, Values } from "./types.ts";

/** A reference entry that a model page renders. */
type ActionEntry = RefEntry & { action: NonNullable<RefEntry["action"]> };

/** The ranges of one argument role, per mode. */
type ByMode = Map<Mode, Range>;

interface RoleEntry {
  modes: Set<Mode>;
  ranges: Map<string, ByMode>;
}

/** The roles that serve one action, merged. `has` names the roles found. */
interface Served extends RoleEntry {
  has: Set<string>;
}

/** One role can hold a different range in each mode. */
function index(device: Device): Map<string, RoleEntry> {
  const roles = new Map<string, RoleEntry>();
  for (const mode of MODES) {
    for (const command of Object.values(device.commands?.[mode] ?? {})) {
      if (!filled(command.role)) continue;
      const entry: RoleEntry = roles.get(command.role) ?? { modes: new Set(), ranges: new Map() };
      entry.modes.add(mode);
      for (const arg of Object.values(command.args ?? {})) {
        if (!filled(arg.role) || !arg.range) continue;
        const byMode: ByMode = entry.ranges.get(arg.role) ?? new Map();
        byMode.set(mode, arg.range);
        entry.ranges.set(arg.role, byMode);
      }
      roles.set(command.role, entry);
    }
  }
  return roles;
}

function merge(roles: Map<string, RoleEntry>, wanted: string[]): Served | null {
  const entry: Served = { modes: new Set(), ranges: new Map(), has: new Set() };
  for (const role of wanted) {
    const part = roles.get(role);
    if (!part) continue;
    entry.has.add(role);
    for (const mode of part.modes) entry.modes.add(mode);
    for (const [arg, byMode] of part.ranges) {
      const into: ByMode = entry.ranges.get(arg) ?? new Map();
      for (const [mode, range] of byMode) into.set(mode, range);
      entry.ranges.set(arg, into);
    }
  }
  return entry.has.size > 0 ? entry : null;
}

/** A value inside the range of every mode, so one example holds wherever the
 * reader sends it. Falls back where the device file bounds nothing. */
function pick(entry: Served, role: string, fallback: Values[string]): Values[string] {
  const byMode = entry.ranges.get(role);
  if (!byMode || byMode.size === 0) return fallback;
  const ranges = [...byMode.values()];
  const low = Math.max(...ranges.map((r) => r[0]));
  const high = Math.min(...ranges.map((r) => r[1]));
  if (low > high) return fallback;
  const span = high - low;
  const step = span <= 20 ? 1 : span <= 200 ? 5 : 100;
  const value = Math.round((low + span / 2) / step) * step;
  return Math.min(high, Math.max(low, value));
}

const boundsList = (rows: string[]): string => (rows.length > 0 ? `<ul class="bounds">${rows.join("")}</ul>` : "");

// The modes share a row where they agree on the range, and hold one row each
// where they do not.
function bounds(entry: Served, args: Record<string, string>): string {
  const rows: string[] = [];
  for (const [label, role] of Object.entries(args)) {
    const byMode = entry.ranges.get(role);
    if (!byMode || byMode.size === 0) continue;
    const forms = new Set([...byMode.values()].map((r) => `${r[0]}–${r[1]}`));
    if (forms.size === 1 && byMode.size === entry.modes.size) {
      rows.push(`<li><code>${escapeHtml(label)}</code> ${[...forms][0]}</li>`);
      continue;
    }
    for (const mode of MODES) {
      const range = byMode.get(mode);
      if (range) rows.push(`<li>${modeMark(mode)}<code>${escapeHtml(label)}</code> ${range[0]}–${range[1]}</li>`);
    }
  }
  return boundsList(rows);
}

const zones = (device: Device) => device.capabilities?.segments ?? null;

// True where the unit renders more LEDs than it has zones, so a frame can
// state one color per LED.
function native(device: Device): boolean {
  const seg = zones(device);
  return Boolean(seg?.native_pixels) && seg?.native_pixels !== seg?.count;
}

function segments(device: Device): string {
  const seg = zones(device);
  const rows: string[] = [];
  if (seg?.count !== undefined && seg.count > 0) rows.push(`<li><code>zones</code> ${seg.count}</li>`);
  if (seg?.native_pixels !== undefined && seg.native_pixels > 0) rows.push(`<li><code>pixels</code> ${seg.native_pixels}</li>`);
  return boundsList(rows);
}

// The values this unit puts in the placeholders, and the names it satisfies.
function unit(item: RefEntry, entry: Served, device: Device): { values: Values; can: Set<string> } {
  const values: Values = {};
  for (const [name, role] of Object.entries(item.args ?? {})) {
    values[name] = pick(entry, role, item.values?.[name]);
  }
  const can = new Set(entry.has);
  if (native(device)) can.add("native_pixels");
  return { values, can };
}

// A badge marks the exception: an action every mode serves carries none.
function block(item: ActionEntry, entry: Served, device: Device): string {
  const { action } = item;
  const served = MODES.filter((m) => entry.modes.has(m));
  const marks = served.length === MODES.length
    ? ""
    : `<span class="ref-modes">${served.map((mode) => modeBadge(mode)).join("")}</span>`;
  const summary = filled(action.summary) ? `<p class="summary">${escapeHtml(action.summary)}</p>` : "";
  const extra = action.segments === true ? segments(device) : "";
  const { values, can } = unit(item, entry, device);
  return `<article class="ref-entry" id="send-${escapeAttr(item.id)}">
            <div class="ref-text">
              <h3><a class="anchor" href="#send-${escapeAttr(item.id)}">${escapeHtml(action.title)}</a>${marks}</h3>
              ${summary}${extra}${bounds(entry, item.args ?? {})}
            </div>
            ${langBlock(`send-${item.id}`, examples(item, values, can))}
          </article>`;
}

/**
 * The section, or an empty string where the device file serves no action. The
 * caller places it: this returns markup and reads nothing but the catalog and
 * the reference content.
 */
export function usage(device: Device, reference: Reference): string {
  const roles = index(device);
  const actions = reference.groups
    .flatMap((group) => group.entries)
    .filter((item): item is ActionEntry => Boolean(item.action))
    .toSorted((a, b) => a.action.order - b.action.order);
  const blocks = actions.map((item) => {
    const entry = merge(roles, item.roles ?? []);
    return entry ? block(item, entry, device) : "";
  }).filter(Boolean);
  if (blocks.length === 0) return "";
  return `<h2 id="send">What you can send</h2>
      <div class="usage">
          ${blocks.join("\n          ")}
      </div>`;
}
