// The "What you can send" section of a model page: one block per action the
// device file serves, with the bounds of this model and the same action in
// each language.
//
// The actions are the entries of `content/reference.json` that carry a
// `roles:` and an `action:`. The reference page renders those same entries
// with the generic values, so the two pages cannot drift.
//
// Nothing here names a model or a command the device file declares. An action
// reaches the device file through a `role:`, which is what the command line
// already names, and every number comes from the catalog.

import { MODES } from "./config.mjs";
import { examples } from "./examples.mjs";
import { escapeAttr, escapeHtml } from "./html.mjs";
import { langBlock } from "./languages.mjs";
import { modeBadge, modeMark } from "./mode-badge.mjs";

/** Collects the roles the device file serves, and where each argument is
 * bounded. One role can hold a different range in each mode. */
function index(device) {
  const roles = new Map();
  for (const mode of MODES) {
    for (const command of Object.values(device.commands?.[mode] ?? {})) {
      if (!command.role) continue;
      const entry = roles.get(command.role) ?? { modes: new Set(), ranges: new Map() };
      entry.modes.add(mode);
      for (const arg of Object.values(command.args ?? {})) {
        if (!arg.role || !arg.range) continue;
        const byMode = entry.ranges.get(arg.role) ?? new Map();
        byMode.set(mode, arg.range);
        entry.ranges.set(arg.role, byMode);
      }
      roles.set(command.role, entry);
    }
  }
  return roles;
}

// Merges the roles one action covers into a single entry.
function merge(roles, wanted) {
  const entry = { modes: new Set(), ranges: new Map(), has: new Set() };
  for (const role of wanted) {
    const part = roles.get(role);
    if (!part) continue;
    entry.has.add(role);
    for (const mode of part.modes) entry.modes.add(mode);
    for (const [arg, byMode] of part.ranges) {
      const into = entry.ranges.get(arg) ?? new Map();
      for (const [mode, range] of byMode) into.set(mode, range);
      entry.ranges.set(arg, into);
    }
  }
  return entry.has.size ? entry : null;
}

/** A value inside the range of every mode, so one example holds wherever the
 * reader sends it. Falls back where the device file bounds nothing. */
function pick(entry, role, fallback) {
  const byMode = entry.ranges.get(role);
  if (!byMode?.size) return fallback;
  const ranges = [...byMode.values()];
  const low = Math.max(...ranges.map((r) => r[0]));
  const high = Math.min(...ranges.map((r) => r[1]));
  if (low > high) return fallback;
  const span = high - low;
  const step = span <= 20 ? 1 : span <= 200 ? 5 : 100;
  const value = Math.round((low + span / 2) / step) * step;
  return Math.min(high, Math.max(low, value));
}

const boundsList = (rows) => (rows.length ? `<ul class="bounds">${rows.join("")}</ul>` : "");

// One row per bounded argument. The modes share a row where they agree on the
// range, and hold one row each where they do not.
function bounds(entry, args) {
  const rows = [];
  for (const [label, role] of Object.entries(args)) {
    const byMode = entry.ranges.get(role);
    if (!byMode?.size) continue;
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

const zones = (device) => device.capabilities?.segments ?? null;

// True where the unit renders more LEDs than it has zones, so a frame can
// state one color per LED.
function native(device) {
  const seg = zones(device);
  return Boolean(seg?.native_pixels) && seg.native_pixels !== seg.count;
}

function segments(device) {
  const seg = zones(device);
  const rows = [];
  if (seg?.count) rows.push(`<li><code>zones</code> ${seg.count}</li>`);
  if (seg?.native_pixels) rows.push(`<li><code>pixels</code> ${seg.native_pixels}</li>`);
  return boundsList(rows);
}

// The values this unit puts in the placeholders, and the names it satisfies.
// An `adds` block that asks for a name the unit misses stays out.
function unit(item, entry, device) {
  const values = {};
  for (const [name, role] of Object.entries(item.args ?? {})) {
    values[name] = pick(entry, role, item.values?.[name]);
  }
  const can = new Set(entry.has);
  if (native(device)) can.add("native_pixels");
  return { values, can };
}

// A badge marks the exception: an action every mode serves carries none.
function block(item, entry, device) {
  const { action } = item;
  const served = MODES.filter((m) => entry.modes.has(m));
  const marks = served.length === MODES.length
    ? ""
    : `<span class="ref-modes">${served.map(modeBadge).join("")}</span>`;
  const summary = action.summary ? `<p class="summary">${escapeHtml(action.summary)}</p>` : "";
  const extra = action.segments ? segments(device) : "";
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
export function usage(device, reference) {
  const roles = index(device);
  const actions = reference.groups
    .flatMap((group) => group.entries)
    .filter((item) => item.action)
    .sort((a, b) => a.action.order - b.action.order);
  const blocks = actions.map((item) => {
    const entry = merge(roles, item.roles);
    return entry ? block(item, entry, device) : "";
  }).filter(Boolean);
  if (!blocks.length) return "";
  return `<h2 id="send">What you can send</h2>
      <div class="usage">
          ${blocks.join("\n          ")}
      </div>`;
}
