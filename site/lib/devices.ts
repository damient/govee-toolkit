// Nothing here names a model or a command: the device files own them.

import { categoryOptions, deviceTrail } from "./categories.ts";
import { MODES } from "./config.ts";
import { crumbs } from "./crumbs.ts";
import { dmx } from "./device-dmx.ts";
import { usage } from "./device-usage.ts";
import { DASH, escapeAttr, escapeHtml, fill, filled } from "./html.ts";
import { icon, shapeIcon, sharesMark } from "./icons.ts";
import { infoBadge, listedFile, listedVerification } from "./listed.ts";
import { categoryBadge, dmxBadge, familyBadge, modeBadge } from "./mode-badge.ts";
import type { Capability, Crumb, Device, Mode, Reference, Support } from "./types.ts";

// The order a reader looks for, not the order the catalog holds.
const CAPS: [string, string][] = [
  ["power", "On / off"],
  ["color", "Color"],
  ["colortemp", "White temperature"],
  ["brightness", "Brightness"],
  ["music", "Music"],
  ["segments", "Segments"],
  ["segment_brightness", "Segment brightness"],
];

const CAP_LABELS = new Map(CAPS);
const CAP_ORDER = new Map(CAPS.map(([key], at) => [key, at]));

/** Sort rank of a capability key. An unknown key sorts last. */
const order = (key: string): number => CAP_ORDER.get(key) ?? CAPS.length;

const support = (device: Device, mode: Mode): Support => device.modes?.[mode]?.support ?? "unknown";

const REACHES = new Set<Support>(["full", "capped", "partial"]);

const CHECK = '<svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"'
  + ' stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'
  + '<path d="M4.5 12.6 9.4 17.5 19.5 7"/></svg>';
const pill = (value: Support): string =>
  `<span class="pill pill-${value}">${value === "unknown" ? "?" : value}</span>`;

/** A key such as `segment_brightness` as a reader reads it. */
function label(key: string): string {
  return CAP_LABELS.get(key) ?? key.replaceAll("_", " ");
}

/** The device list. `current` names the category of a category page, which
 * lists the rows of that category alone. */
export function renderIndex(template: string, devices: Device[], current?: string): string {
  const rows = current === undefined ? devices : devices.filter((d) => d.category === current);
  return fill(template, {
    devices_rows: rows.map((d) => row(d)).join("\n"),
    devices_categories: categoryOptions(devices, current),
    caps_legend: capsLegend(rows),
    check_mark: CHECK,
    devices_heading: current === undefined ? "Which Govee lights work." : `Govee ${escapeHtml(current)}`,
    devices_scope: current === undefined ? "in the catalog" : `in ${escapeAttr(current)}`,
  });
}

// A mode that reaches no capability shows its state instead.
function modeCell(d: Device, mode: Mode): string {
  const state = support(d, mode);
  // Two capabilities that share a mark show once. The model page names both.
  const caps = [...(d.modes?.[mode]?.capabilities ?? [])]
    .filter((c) => !sharesMark(c))
    .toSorted((a, b) => order(a) - order(b));
  if (caps.length === 0) {
    if (state === "none") return DASH;
    return pill(state);
  }
  const items = caps
    .map(
      (c) => `<li class="cap-chip" data-tip="${escapeAttr(label(c))}">${icon(c)}`
        + `<span class="visually-hidden">${escapeHtml(label(c))}</span></li>`,
    )
    .join("");
  return `<ul class="caps-row">${items}</ul>`;
}

// A device file that declares a new capability adds its row to the legend.
function capsLegend(devices: Device[]): string {
  const keys = new Set(devices.flatMap((d) => Object.keys(d.capabilities ?? {})));
  return [...keys]
    .filter((key) => icon(key) !== "" && !sharesMark(key))
    .toSorted((a, b) => order(a) - order(b))
    .map((key) => `<li>${icon(key)}${escapeHtml(label(key))}</li>`)
    .join("\n        ");
}

function hasDmx(d: Device): boolean {
  return (d.dmx?.personalities?.length ?? 0) > 0;
}

function dmxCell(d: Device): string {
  if (!hasDmx(d)) return DASH;
  return `<span class="dmx-yes">${CHECK}<span class="visually-hidden">DMX</span></span>`;
}

function row(d: Device): string {
  const cells = MODES.map((m) => `<td>${modeCell(d, m)}</td>`).join("")
    + `<td>${dmxCell(d)}</td>`;
  const names = [d.sku, d.name, ...(d.aliases ?? [])].join(" ").toLowerCase();
  const mark = d.listing === undefined ? "" : ` ${infoBadge()}`;
  return `          <tr class="has-page" data-search="${escapeAttr(names)}">
            <th scope="row"><a href="{{base}}devices/${escapeAttr(d.sku)}/">${escapeHtml(d.sku)}</a></th>
            <td class="shape">${shapeIcon(d.shape ?? d.family)}</td>
            <td>${escapeHtml(d.name)}${mark}</td>${cells}
          </tr>`;
}

/**
 * Builds the page of one model. The caller emits it: this returns the URL, the
 * title, the description and the body, and never touches the file system.
 */
export function devicePage(d: Device, reference: Reference) {
  const trail = deviceTrail(d);
  return {
    url: `devices/${d.sku}/`,
    title: `Govee ${d.sku} — ${d.name}`,
    description: describe(d),
    body: pageBody(d, reference, trail),
    breadcrumb: trail,
  };
}

const PATHS: Record<Mode, string> = { lan: "Wi-Fi", ble: "Bluetooth", cloud: "the cloud" };

// A mode reaches its capabilities while nobody has probed it: a listed model
// declares them, and every mode of it stays `unknown`.
const reached = (d: Device): Mode[] => MODES.filter((m) => REACHES.has(support(d, m))
  || (support(d, m) === "unknown" && (d.modes?.[m]?.capabilities?.length ?? 0) > 0));

function describe(d: Device): string {
  const paths = reached(d).map((m) => PATHS[m]);
  const head = `the Govee ${d.sku} (${d.name})`;
  const what = paths.length > 0
    ? `Control ${head} from your own computer, over ${joined(paths)}${hasDmx(d) ? ", and from a DMX desk" : ""}.`
    : `What Govee Toolkit reaches on ${head} is still to probe.`;
  if (filled(d.verified?.date)) return `${what} Verified on ${d.verified.date}.`;
  return d.listing === undefined ? what : `${what} Untested on this model.`;
}

function joined(words: string[]): string {
  return words.length < 2 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} and ${words.at(-1)}`;
}

function pageBody(d: Device, reference: Reference, trail: Crumb[]): string {
  const title = d.name ? `${d.sku} — ${escapeHtml(d.name)}` : d.sku;
  const [family, check, file] = d.listing === undefined
    ? [familyBadge(d.family), verification(d), deviceFile(d)]
    : ["", listedVerification(), listedFile(d)];

  return `<section class="pagehead">
  <div class="shell">
    ${crumbs(trail)}
    <h1>${title}</h1>
    <p class="mode-line">${categoryBadge(d.category, d.shape ?? d.family)}${family}${reached(d).map((m) => modeBadge(m)).join("")}${hasDmx(d) ? dmxBadge() : ""}${badges(d)}</p>
  </div>
</section>

<section class="slab">
  <div class="shell">
    <article class="prose">
      ${capabilities(d)}
      ${modeSections(d)}
      ${usage(d, reference)}
      ${dmx(d)}
      ${aliases(d)}
      <div class="endgrid">
        <section>${check}</section>
        <section>${file}</section>
      </div>
      <p class="endcta"><a class="btn" href="{{base}}devices/">Discover other devices</a></p>
    </article>
  </div>
</section>`;
}

function deviceFile(d: Device): string {
  return `<h2 id="the-device-file">The device file</h2>
          <p>This page states what <a href="{{repo}}/blob/main/devices/${d.sku}.yaml"><code>devices/${d.sku}.yaml</code></a>
          holds.</p>`;
}

function badges(d: Device): string {
  return filled(d.verified?.date)
    ? `<span class="badge badge-ok">${CHECK}verified</span>`
    : `<span class="badge badge-unknown">not verified</span>`;
}

function capabilities(d: Device): string {
  const caps = Object.entries(d.capabilities ?? {});
  if (caps.length === 0) return "";
  const list = caps
    .toSorted(([a], [b]) => order(a) - order(b))
    .map(([key, value]) => `<li>${icon(key)}${escapeHtml(label(key))}${counts(key, value, d)}</li>`)
    .join("");
  return `<h2 id="what-the-hardware-does">What the hardware does</h2>
      <ul class="caps caps-lg">${list}</ul>`;
}

// Where one zone is one LED, the grid carries the shape that the counts do not.
function counts(key: string, value: Capability | null, device: Device): string {
  if (key !== "segments" || !value) return "";
  const parts: string[] = [];
  const grid = device.measurements?.segment_grid;
  const oneZonePerPixel = (value.count ?? 0) > 0 && value.count === value.native_pixels;
  if (oneZonePerPixel && Array.isArray(grid) && grid.length === 2) {
    parts.push(`${escapeHtml(grid[0])} × ${escapeHtml(grid[1])} grid`);
  } else if ((value.count ?? 0) > 0) {
    parts.push(`${escapeHtml(value.count)} zones`);
  }
  if ((value.native_pixels ?? 0) > 0) parts.push(`${escapeHtml(value.native_pixels)} pixels`);
  return parts.length > 0
    ? `<span class="badge-count"><span aria-hidden="true">|</span> ${parts.join(" · ")}</span>`
    : "";
}

function modeCaps(d: Device, mode: Mode): string {
  const caps = (d.modes?.[mode]?.capabilities ?? []).toSorted((a, b) => order(a) - order(b));
  if (caps.length > 0) {
    const chips = caps.map((c) => `<li>${icon(c)}${escapeHtml(label(c))}</li>`).join("");
    return `<ul class="caps">${chips}</ul>`;
  }
  const probed = support(d, mode) !== "unknown";
  return `<span class="mode-note">${probed ? "Not available" : "Not probed yet"}</span>`;
}

function modeSections(d: Device): string {
  const rows = MODES.map(
    (m) => `<tr><th scope="row">${modeBadge(m)}</th>
            <td>${modeCaps(d, m)}</td></tr>`,
  );

  return `<h2 id="modes">What each mode reaches</h2>
      <div class="tablewrap">
        <table class="matrix matrix-modes">
          <thead><tr><th scope="col">Mode</th><th scope="col">Capabilities</th></tr></thead>
          <tbody>
            ${rows.join("\n            ")}
          </tbody>
        </table>
      </div>`;
}

function aliases(d: Device): string {
  const same = d.aliases ?? [];
  const candidates = d.candidate_aliases ?? [];
  if (same.length === 0 && candidates.length === 0) return "";
  const verified = same.length > 0
    ? `<p><strong>The same device:</strong> ${same.map((s) => `<code>${escapeHtml(s)}</code>`).join(", ")}.
      Each was verified to behave identically.</p>`
    : "";
  const maybe = candidates.length > 0
    ? `<p><strong>Looks like the same product, not verified:</strong> ${candidates.map((s) => `<code>${escapeHtml(s)}</code>`).join(", ")}.
      A different length has a different segment count, so these are candidates
      and nothing more.</p>`
    : "";
  return `<h2 id="other-skus">Other SKUs</h2>
      ${verified}${maybe}`;
}

function verification(d: Device): string {
  if (!filled(d.verified?.date)) {
    return `<h2 id="verification">Verification</h2>
      <p>Nobody has verified this model yet. That is a different answer from
      <em>it does not work</em>: an untested model is untested.
      <a href="{{repo}}/issues">Report what you observe</a> and the device file
      takes it.</p>`;
  }
  const firmware = filled(d.verified.firmware)
    ? ` on firmware <code>${escapeHtml(d.verified.firmware)}</code>`
    : "";
  const by = filled(d.verified.by) ? ` by <code>${escapeHtml(d.verified.by)}</code>` : "";
  return `<h2 id="verification">Verification</h2>
      <p>Verified${by} on <time datetime="${escapeAttr(d.verified.date)}">${escapeHtml(d.verified.date)}</time>${firmware}.</p>`;
}
