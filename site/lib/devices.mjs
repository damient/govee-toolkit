// Renders the device catalog from `dist/catalog.json`, which
// `cargo run -p xtask -- catalog` writes from `devices/*.yaml`. Nothing below
// names a model or a command: a renderer that did would disagree with the
// device files the day one of them changes.

import { MODES } from "./config.mjs";
import { escapeAttr, escapeHtml, fill } from "./html.mjs";
import { familyIcon, icon } from "./icons.mjs";
import { modeBadge } from "./mode-badge.mjs";

// The order a reader looks for, not the order the catalog holds.
const CAPS = [
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
const order = (key) => CAP_ORDER.get(key) ?? CAPS.length;

// `segment_brightness` shares the icon of `segments`, and only the color tells
// the two apart. Side by side as badges they read as one mark repeated, so the
// list carries the zones alone. The model page names both.
const LIST_SKIP = new Set(["segment_brightness"]);

/** Sorts by SKU, so that two builds of one catalog give one page. */
export function sorted(catalog) {
  return [...catalog.devices].sort((a, b) => a.sku.localeCompare(b.sku));
}

const support = (device, mode) => device.modes?.[mode]?.support ?? "unknown";

// The support values that say a mode reaches the device. `none` is the hardware
// that cannot do it and `unknown` is the mode nobody probed, and neither is a
// mode a reader can drive.
const REACHES = new Set(["full", "capped", "partial"]);
const pill = (value) =>
  `<span class="pill pill-${value}">${value === "unknown" ? "?" : value}</span>`;

/** A key such as `segment_brightness` as a reader reads it. */
function label(key) {
  return CAP_LABELS.get(key) ?? key.replace(/_/g, " ");
}

export function renderIndex(template, devices) {
  return fill(template, {
    devices_rows: devices.map(row).join("\n"),
    caps_legend: capsLegend(devices),
  });
}

// One cell of the list: the capabilities the mode reaches, as icons. A mode
// that reaches none carries its state instead, and the two states are opposite
// claims — the hardware cannot do it, or nobody looked.
function modeCell(d, mode) {
  const state = support(d, mode);
  const caps = [...(d.modes?.[mode]?.capabilities ?? [])]
    .filter((c) => !LIST_SKIP.has(c))
    .sort((a, b) => order(a) - order(b));
  if (!caps.length) {
    if (state === "none") return '<span class="muted">—</span>';
    return state === "unknown" ? pill("unknown") : pill(state);
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
function capsLegend(devices) {
  const keys = new Set(devices.flatMap((d) => Object.keys(d.capabilities ?? {})));
  return [...keys]
    .filter((key) => icon(key) && !LIST_SKIP.has(key))
    .sort((a, b) => order(a) - order(b))
    .map((key) => `<li>${icon(key)}${escapeHtml(label(key))}</li>`)
    .join("\n        ");
}

function row(d) {
  const cells = MODES.map((m) => `<td>${modeCell(d, m)}</td>`).join("");
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
export function devicePage(d) {
  const modes = MODES.map((m) => `${m} ${support(d, m)}`).join(", ");
  const verified = d.verified?.date
    ? `Verified on ${d.verified.date}.`
    : "Nobody has verified this model yet.";
  return {
    url: `devices/${d.sku}/`,
    title: `${d.sku} — ${d.name}`,
    description: `What govee-toolkit reaches on the ${d.sku} (${d.name}): ${modes}. ${verified}`,
    body: pageBody(d),
    breadcrumb: [["Devices", "devices/"], [d.sku, `devices/${d.sku}/`]],
  };
}

function pageBody(d) {
  const title = d.name ? `${d.sku} — ${escapeHtml(d.name)}` : d.sku;

  return `<section class="pagehead">
  <div class="shell">
    <nav class="crumbs" aria-label="Breadcrumb">
      <a href="{{base}}devices/">Devices</a> <span aria-hidden="true">/</span> <span>${d.sku}</span>
    </nav>
    <h1>${title}</h1>
    <p class="mode-line">${familyBadge(d)}${MODES.filter((m) => REACHES.has(support(d, m)))
      .map((m) => modeBadge(m))
      .join("")}${badges(d)}</p>
  </div>
</section>

<section class="slab">
  <div class="shell">
    <article class="prose">
      ${capabilities(d)}
      ${modeSections(d)}
      ${commands(d)}
      ${aliases(d)}
      ${verification(d)}
      <h2 id="the-device-file">The device file</h2>
      <p>This page states what <a href="{{repo}}/blob/main/devices/${d.sku}.yaml"><code>devices/${d.sku}.yaml</code></a>
      holds.</p>
      <p><a href="{{base}}devices/">Back to every model</a></p>
    </article>
  </div>
</section>`;
}

// The family, in the badge the modes use: the shape and the key name the
// product, and the badges beside it name what reaches it.
function familyBadge(d) {
  if (!d.family) return "";
  return `<span class="mbadge mbadge-family">${familyIcon(d.family)}${escapeHtml(d.family)}</span>`;
}

function badges(d) {
  const check = `<svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4.5 12.6 9.4 17.5 19.5 7"/></svg>`;
  return d.verified?.date
    ? `<span class="badge badge-ok">${check}verified</span>`
    : `<span class="badge badge-unknown">not verified</span>`;
}

function capabilities(d) {
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
function counts(key, value, device) {
  if (key !== "segments" || !value) return "";
  const parts = [];
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


// Every mode holds a row. A mode that reaches nothing answers in words, where
// the list of models answers in a mark: the row has the width for it.
function modeCaps(d, mode) {
  const caps = [...(d.modes?.[mode]?.capabilities ?? [])].sort((a, b) => order(a) - order(b));
  if (caps.length) {
    const chips = caps.map((c) => `<li>${icon(c)}${escapeHtml(label(c))}</li>`).join("");
    return `<ul class="caps">${chips}</ul>`;
  }
  const probed = support(d, mode) !== "unknown";
  return `<span class="mode-note">${probed ? "Not available" : "Not probed yet"}</span>`;
}

function modeSections(d) {
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

function commands(d) {
  const sections = MODES.map((m) => {
    const table = d.commands?.[m] ?? {};
    const names = Object.keys(table).sort();
    if (!names.length) return "";
    const rows = names
      .map((name) => {
        const command = table[name];
        const args = Object.keys(command.args ?? {});
        const takes = args.length ? args.map((a) => `<code>${escapeHtml(a)}</code>`).join(", ") : "—";
        return `<tr><th scope="row"><code>${escapeHtml(name)}</code></th><td>${takes}</td></tr>`;
      })
      .join("\n          ");
    return `<h3 id="commands-${m}">${modeBadge(m)}</h3>
      <div class="tablewrap">
        <table class="matrix">
          <thead><tr><th scope="col">Command</th><th scope="col">Arguments</th></tr></thead>
          <tbody>
            ${rows}
          </tbody>
        </table>
      </div>`;
  }).filter(Boolean).join("\n      ");

  if (!sections) return "";
  return `<h2 id="commands">The commands the file declares</h2>
      ${sections}`;
}

function aliases(d) {
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

function verification(d) {
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
