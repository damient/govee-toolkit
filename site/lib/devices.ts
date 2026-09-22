// Renders the device catalog from `dist/catalog.json`, which
// `cargo run -p xtask -- catalog` writes from `devices/*.yaml`. Nothing below
// names a model or a command: a renderer that did would disagree with the
// device files the day one of them changes.

import { MODES } from "./config.ts";
import { crumbs } from "./crumbs.ts";
import { dmx } from "./device-dmx.ts";
import { usage } from "./device-usage.ts";
import { DASH, escapeAttr, escapeHtml, fill } from "./html.ts";
import { familyIcon, icon, sharesMark } from "./icons.ts";
import { dmxBadge, familyBadge, modeBadge } from "./mode-badge.ts";
import type { Capability, Catalog, Crumb, Device, Mode, Reference, Support } from "./types.ts";

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

/** Sorts by SKU, so that two builds of one catalog give one page. */
export function sorted(catalog: Catalog): Device[] {
  return [...catalog.devices].sort((a, b) => a.sku.localeCompare(b.sku));
}

const support = (device: Device, mode: Mode): Support => device.modes?.[mode]?.support ?? "unknown";

const REACHES = new Set<Support>(["full", "capped", "partial"]);

const CHECK = '<svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"'
  + ' stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">'
  + '<path d="M4.5 12.6 9.4 17.5 19.5 7"/></svg>';
const pill = (value: Support): string =>
  `<span class="pill pill-${value}">${value === "unknown" ? "?" : value}</span>`;

/** A key such as `segment_brightness` as a reader reads it. */
function label(key: string): string {
  return CAP_LABELS.get(key) ?? key.replace(/_/g, " ");
}

export function renderIndex(template: string, devices: Device[]): string {
  return fill(template, {
    devices_rows: devices.map(row).join("\n"),
    caps_legend: capsLegend(devices),
    check_mark: CHECK,
  });
}

// One cell of the list: the capabilities the mode reaches, as icons. A mode
// that reaches none carries its state instead.
function modeCell(d: Device, mode: Mode): string {
  const state = support(d, mode);
  // Two capabilities that share a mark read as one mark repeated, so the list
  // carries one of them. The model page names both.
  const caps = [...(d.modes?.[mode]?.capabilities ?? [])]
    .filter((c) => !sharesMark(c))
    .sort((a, b) => order(a) - order(b));
  if (!caps.length) {
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

// The legend reads the catalog rather than a list of its own, so a device file
// that declares a new capability adds its row here.
function capsLegend(devices: Device[]): string {
  const keys = new Set(devices.flatMap((d) => Object.keys(d.capabilities ?? {})));
  return [...keys]
    .filter((key) => icon(key) && !sharesMark(key))
    .sort((a, b) => order(a) - order(b))
    .map((key) => `<li>${icon(key)}${escapeHtml(label(key))}</li>`)
    .join("\n        ");
}

// The DMX column answers yes or nothing: the bridge derives a channel table
// from the device file, so a model that carries no table answers no desk.
function dmxCell(d: Device): string {
  if (!d.dmx?.personalities?.length) return DASH;
  return `<span class="dmx-yes">${CHECK}<span class="visually-hidden">DMX</span></span>`;
}

function row(d: Device): string {
  const cells = MODES.map((m) => `<td>${modeCell(d, m)}</td>`).join("")
    + `<td>${dmxCell(d)}</td>`;
  const names = [d.sku, d.name, ...(d.aliases ?? [])].join(" ").toLowerCase();
  return `          <tr data-search="${escapeAttr(names)}">
            <th scope="row"><a href="{{base}}devices/${escapeAttr(d.sku)}/">${escapeHtml(d.sku)}</a></th>
            <td class="shape">${familyIcon(d.family)}</td>
            <td>${escapeHtml(d.name)}</td>${cells}
          </tr>`;
}

/**
 * Builds the page of one model. The caller emits it: this returns the URL, the
 * title, the description and the body, and never touches the file system.
 */
export function devicePage(d: Device, reference: Reference) {
  const trail: Crumb[] = [["Devices", "devices/"], [d.sku, `devices/${d.sku}/`]];
  return {
    url: `devices/${d.sku}/`,
    title: `Govee ${d.sku} — ${d.name}`,
    description: describe(d),
    body: pageBody(d, reference, trail),
    breadcrumb: trail,
  };
}

// The description a search result shows. It names the paths that reach the
// model and no other, as the copy of the site does.
const PATHS: Record<Mode, string> = { lan: "Wi-Fi", ble: "Bluetooth", cloud: "the cloud" };

function describe(d: Device): string {
  const paths = MODES.filter((m) => REACHES.has(support(d, m))).map((m) => PATHS[m]);
  const head = `the Govee ${d.sku} (${d.name})`;
  const what = paths.length
    ? `Control ${head} from your own computer, over ${joined(paths)}${d.dmx?.personalities?.length ? ", and from a DMX desk" : ""}.`
    : `What Govee Toolkit reaches on ${head} is still to probe.`;
  const when = d.verified?.date ? ` Verified on ${d.verified.date}.` : "";
  return what + when;
}

function joined(words: string[]): string {
  return words.length < 2 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} and ${words.at(-1)}`;
}

function pageBody(d: Device, reference: Reference, trail: Crumb[]): string {
  const title = d.name ? `${d.sku} — ${escapeHtml(d.name)}` : d.sku;

  return `<section class="pagehead">
  <div class="shell">
    ${crumbs(trail)}
    <h1>${title}</h1>
    <p class="mode-line">${familyBadge(d.family)}${MODES.filter((m) => REACHES.has(support(d, m)))
      .map((m) => modeBadge(m))
      .join("")}${d.dmx?.personalities?.length ? dmxBadge() : ""}${badges(d)}</p>
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
        <section>${verification(d)}</section>
        <section>
          <h2 id="the-device-file">The device file</h2>
          <p>This page states what <a href="{{repo}}/blob/main/devices/${d.sku}.yaml"><code>devices/${d.sku}.yaml</code></a>
          holds.</p>
        </section>
      </div>
      <p class="endcta"><a class="btn" href="{{base}}devices/">Discover other devices</a></p>
    </article>
  </div>
</section>`;
}

function badges(d: Device): string {
  return d.verified?.date
    ? `<span class="badge badge-ok">${CHECK}verified</span>`
    : `<span class="badge badge-unknown">not verified</span>`;
}

function capabilities(d: Device): string {
  const caps = Object.entries(d.capabilities ?? {});
  if (!caps.length) return "";
  const list = caps
    .sort(([a], [b]) => order(a) - order(b))
    .map(([key, value]) => `<li>${icon(key)}${escapeHtml(label(key))}${counts(key, value, d)}</li>`)
    .join("");
  return `<h2 id="what-the-hardware-does">What the hardware does</h2>
      <ul class="caps caps-lg">${list}</ul>`;
}

// A capability is a yes or a no, except the segments: the zone and pixel
// counts decide what a reader can paint. Where one zone is one LED, the grid
// carries the shape that the two equal counts do not.
function counts(key: string, value: Capability | null, device: Device): string {
  if (key !== "segments" || !value) return "";
  const parts: string[] = [];
  const grid = device.measurements?.segment_grid;
  const oneZonePerPixel = value.count && value.count === value.native_pixels;
  if (oneZonePerPixel && Array.isArray(grid) && grid.length === 2) {
    parts.push(`${escapeHtml(grid[0])} × ${escapeHtml(grid[1])} grid`);
  } else if (value.count) {
    parts.push(`${escapeHtml(value.count)} zones`);
  }
  if (value.native_pixels) parts.push(`${escapeHtml(value.native_pixels)} pixels`);
  return parts.length
    ? `<span class="badge-count"><span aria-hidden="true">|</span> ${parts.join(" · ")}</span>`
    : "";
}


// Every mode holds a row, and one that reaches nothing answers in words.
function modeCaps(d: Device, mode: Mode): string {
  const caps = [...(d.modes?.[mode]?.capabilities ?? [])].sort((a, b) => order(a) - order(b));
  if (caps.length) {
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
  if (!same.length && !candidates.length) return "";
  const verified = same.length
    ? `<p><strong>The same device:</strong> ${same.map((s) => `<code>${escapeHtml(s)}</code>`).join(", ")}.
      Each was verified to behave identically.</p>`
    : "";
  const maybe = candidates.length
    ? `<p><strong>Looks like the same product, not verified:</strong> ${candidates.map((s) => `<code>${escapeHtml(s)}</code>`).join(", ")}.
      A different length has a different segment count, so these are candidates
      and nothing more.</p>`
    : "";
  return `<h2 id="other-skus">Other SKUs</h2>
      ${verified}${maybe}`;
}

function verification(d: Device): string {
  if (!d.verified?.date) {
    return `<h2 id="verification">Verification</h2>
      <p>Nobody has verified this model yet. That is a different answer from
      <em>it does not work</em>: an untested model is untested.
      <a href="{{repo}}/issues">Report what you observe</a> and the device file
      takes it.</p>`;
  }
  const firmware = d.verified.firmware
    ? ` on firmware <code>${escapeHtml(d.verified.firmware)}</code>`
    : "";
  const by = d.verified.by ? ` by <code>${escapeHtml(d.verified.by)}</code>` : "";
  return `<h2 id="verification">Verification</h2>
      <p>Verified${by} on <time datetime="${escapeAttr(d.verified.date)}">${escapeHtml(d.verified.date)}</time>${firmware}.</p>`;
}
