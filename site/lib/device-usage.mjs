// The "What you can send" section of a model page: one block per action the
// device file serves, with the bounds of this model and the same action in
// each language.
//
// Nothing here names a model or a command the device file declares. An action
// reaches the device file through a `role:`, which is what the command line
// already names, and every number comes from the catalog.

import { MODES } from "./config.mjs";
import { escapeAttr, escapeHtml } from "./html.mjs";
import { langBlock } from "./languages.mjs";
import { modeBadge, modeMark } from "./mode-badge.mjs";

// The identity a reader replaces with their own.
const ID = "DEVICE";
const HEX = "#ff3d00";
const RGB = "255, 61, 0";

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
  const found = wanted.map((role) => roles.get(role)).filter(Boolean);
  if (!found.length) return null;
  const entry = { modes: new Set(), ranges: new Map(), has: new Set() };
  for (const [at, part] of found.entries()) {
    entry.has.add(wanted[at]);
    for (const mode of part.modes) entry.modes.add(mode);
    for (const [role, byMode] of part.ranges) {
      const into = entry.ranges.get(role) ?? new Map();
      for (const [mode, range] of byMode) into.set(mode, range);
      entry.ranges.set(role, into);
    }
  }
  return entry;
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

// One row per bounded argument. The modes share a row where they agree on the
// range, and hold one row each where they do not.
function bounds(entry, args) {
  const rows = [];
  for (const [role, label] of args) {
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
  return rows.length ? `<ul class="bounds">${rows.join("")}</ul>` : "";
}

const zones = (device) => device.capabilities?.segments ?? null;

const ACTIONS = [
  {
    id: "power",
    title: "Turn it on and off",
    roles: ["power"],
    examples: () => ({
      cli: `govee on ${ID}\ngovee off ${ID}`,
      rust: "device.power(true).await?;",
      python: "await device.power(True)",
      node: "await device.power(true)",
    }),
  },
  {
    id: "brightness",
    title: "Set the brightness",
    roles: ["brightness"],
    args: [["brightness", "level"]],
    examples: (entry) => {
      const level = pick(entry, "brightness", 50);
      return {
        cli: `govee brightness ${ID} ${level}`,
        rust: `device.brightness(${level}).await?;`,
        python: `await device.brightness(${level})`,
        node: `await device.brightness(${level})`,
      };
    },
  },
  {
    id: "color",
    title: "Set one color",
    roles: ["color"],
    examples: () => ({
      cli: `govee color ${ID} "${HEX}"`,
      rust: `device.color([${RGB}]).await?;`,
      python: `await device.color((${RGB}))`,
      node: `await device.color([${RGB}])`,
    }),
  },
  {
    id: "colortemp",
    title: "Set the white temperature",
    summary: "White and color are mutually exclusive: this ends the color the device shows.",
    roles: ["color_temp"],
    args: [["color_temp", "kelvin"]],
    examples: (entry) => {
      const kelvin = pick(entry, "color_temp", 4000);
      return {
        cli: `govee colortemp ${ID} ${kelvin}`,
        rust: `device.color_temp(${kelvin}).await?;`,
        python: `await device.color_temp(${kelvin})`,
        node: `await device.colorTemp(${kelvin})`,
      };
    },
  },
  {
    id: "segment",
    title: "Paint the zones",
    roles: ["segment_color", "segment_color_masked"],
    extra: (device) => {
      const seg = zones(device);
      const items = [];
      if (seg?.count) items.push(`<li><code>zones</code> ${seg.count}</li>`);
      if (seg?.native_pixels) items.push(`<li><code>pixels</code> ${seg.native_pixels}</li>`);
      return items.length ? `<ul class="bounds">${items.join("")}</ul>` : "";
    },
    examples: (entry, device) => {
      const seg = zones(device);
      const masked = entry.has.has("segment_color_masked")
        ? `\ngovee segment ${ID} --zones 0,1,2 "${HEX}"`
        : "";
      const native = seg?.native_pixels && seg.native_pixels !== seg.count
        ? `\n# One color per pixel: ${seg.native_pixels} on this unit, in zone order.`
          + `\ngovee segment ${ID} --resolution native "${HEX},#00a3ff,..."`
        : "";
      return {
        cli: `govee segment ${ID} "${HEX}"${masked}${native}`,
        rust: "device.segment(&Paint {\n"
          + "    zones: None,\n    colors: &frame,\n"
          + "    resolution: Resolution::default(),\n    gradient: false,\n}).await?;",
        python: "await device.segment(colors=frame)",
        node: "await device.segment({ colors: frame })",
      };
    },
  },
  {
    id: "gradient",
    title: "Interpolate between the zones",
    summary: "The interpolation wraps from the last zone back to the first.",
    roles: ["segment_gradient"],
    examples: () => ({
      cli: `govee gradient ${ID} on`,
      rust: "device.gradient(true).await?;",
      python: "await device.gradient(True)",
      node: "await device.gradient(true)",
    }),
  },
  {
    id: "music",
    title: "Play a music effect",
    summary: "The device listens on its own microphone.",
    roles: ["music"],
    args: [["effect", "effect"], ["sensitivity", "sensitivity"]],
    examples: (entry) => {
      const effect = pick(entry, "effect", 3);
      const level = pick(entry, "sensitivity", 60);
      return {
        cli: `govee music ${ID} ${effect} --sensitivity ${level}`,
        rust: `device.music(&Music {\n    effect: ${effect},\n    sensitivity: ${level},\n`
          + "    soft: false,\n    color: None,\n}).await?;",
        python: `await device.music(effect=${effect}, sensitivity=${level})`,
        node: `await device.music({ effect: ${effect}, sensitivity: ${level} })`,
      };
    },
  },
  {
    id: "status",
    title: "Read the state back",
    roles: ["status"],
    examples: () => ({
      cli: `govee status ${ID}`,
      rust: "let state = device.status().await?;",
      python: "state = await device.status()",
      node: "const state = await device.status()",
    }),
  },
  {
    id: "provision",
    title: "Put it on a Wi-Fi network",
    roles: ["wifi_provision", "wifi_provision_with_api"],
    examples: () => ({
      cli: `govee provision ${ID} --ssid my-network`,
      rust: "device.provision_wifi(&credentials).await?;",
      python: 'await device.provision_wifi("network name", "password")',
      node: "await device.provisionWifi(credentials)",
    }),
  },
];

// A badge marks the exception: an action every mode serves carries none.
function block(action, entry, device) {
  const served = MODES.filter((m) => entry.modes.has(m));
  const marks = served.length === MODES.length
    ? ""
    : `<span class="ref-modes">${served.map(modeBadge).join("")}</span>`;
  const summary = action.summary ? `<p class="summary">${escapeHtml(action.summary)}</p>` : "";
  return `<article class="ref-entry" id="send-${escapeAttr(action.id)}">
            <div class="ref-text">
              <h3><a class="anchor" href="#send-${escapeAttr(action.id)}">${escapeHtml(action.title)}</a>${marks}</h3>
              ${summary}${action.extra?.(device) ?? ""}${bounds(entry, action.args ?? [])}
            </div>
            ${langBlock(`send-${action.id}`, action.examples(entry, device))}
          </article>`;
}

/**
 * The section, or an empty string where the device file serves no action. The
 * caller places it: this returns markup and reads nothing but the catalog.
 */
export function usage(device) {
  const roles = index(device);
  const blocks = ACTIONS.map((action) => {
    const entry = merge(roles, action.roles);
    return entry ? block(action, entry, device) : "";
  }).filter(Boolean);
  if (!blocks.length) return "";
  return `<h2 id="send">What you can send</h2>
      <div class="usage">
          ${blocks.join("\n          ")}
      </div>`;
}
