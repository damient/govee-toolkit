// Renders the device catalog from `dist/catalog.json`, which
// `cargo run -p xtask -- catalog` writes from `devices/*.yaml`. Nothing below
// names a model or a command: a renderer that did would disagree with the
// device files the day one of them changes.

import { MODES } from "./config.mjs";
import { escapeAttr, escapeHtml, fill } from "./html.mjs";
import { icon } from "./icons.mjs";
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

// Why a capability is out of a mode's reach. The vocabulary is
// `docs/compatibility.md`; these are the same three reasons, for a reader.
const REASONS = new Map([
  ["transport", "this transport does not carry it"],
  ["unimplemented", "the transport carries it, this device file does not declare it yet"],
  ["unprobed", "nobody has checked"],
]);

const CAP_LABELS = new Map(CAPS);
const CAP_ORDER = new Map(CAPS.map(([key], at) => [key, at]));

/** Sort rank of a capability key. An unknown key sorts last. */
const order = (key) => CAP_ORDER.get(key) ?? CAPS.length;

/** Sorts by SKU, so that two builds of one catalog give one page. */
export function sorted(catalog) {
  return [...catalog.devices].sort((a, b) => a.sku.localeCompare(b.sku));
}

const support = (device, mode) => device.modes?.[mode]?.support ?? "unknown";
const pill = (value) =>
  `<span class="pill pill-${value}">${value === "unknown" ? "?" : value}</span>`;

/** A key such as `segment_brightness` as a reader reads it. */
function label(key) {
  return CAP_LABELS.get(key) ?? key.replace(/_/g, " ");
}

export function renderIndex(template, devices) {
  return fill(template, { devices_rows: devices.map(row).join("\n") });
}

function row(d) {
  // A mode the hardware cannot do reads as a dash: a pill would give the
  // eye a value to compare, and there is nothing to compare.
  const cells = MODES.map((m) => {
    const state = support(d, m);
    return `<td>${state === "none" ? '<span class="muted">—</span>' : pill(state)}</td>`;
  }).join("");
  const names = [d.sku, d.name, ...(d.aliases ?? [])].join(" ").toLowerCase();
  return `          <tr data-search="${escapeAttr(names)}">
            <th scope="row"><a href="{{base}}devices/${escapeAttr(d.sku)}/">${escapeHtml(d.sku)}</a></th>
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
    <p class="mode-line">${MODES.filter((m) => support(d, m) !== "none")
      .map((m) => modeBadge(m, support(d, m)))
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
    .map(([key, value]) => `<li>${icon(key)}${escapeHtml(label(key))}${counts(key, value)}</li>`)
    .join("");
  return `<h2 id="what-the-hardware-does">What the hardware does</h2>
      <ul class="caps caps-lg">${list}</ul>`;
}

// A capability is a yes or a no, except the segments: the zone and pixel
// counts decide what a reader can paint.
function counts(key, value) {
  if (key !== "segments" || !value) return "";
  const parts = [];
  if (value.count) parts.push(`${escapeHtml(value.count)} zones`);
  if (value.native_pixels) parts.push(`${escapeHtml(value.native_pixels)} pixels`);
  return parts.length
    ? `<span class="badge-count"><span aria-hidden="true">|</span> ${parts.join(" · ")}</span>`
    : "";
}


function modeSections(d) {
  const blocks = MODES.map((m) => {
    const caps = [...(d.modes?.[m]?.capabilities ?? [])].sort((a, b) => order(a) - order(b));
    if (!caps.length) return "";
    const chips = caps.map((c) => `<li>${icon(c)}${escapeHtml(label(c))}</li>`).join("");
    return `<h3 id="mode-${m}">${modeBadge(m, support(d, m))}</h3>
      <ul class="caps">${chips}</ul>${outOfReach(d, m)}`;
  }).filter(Boolean);

  // With one mode, the section would repeat the chips over the title.
  if (blocks.length < 2) return "";
  return `<h2 id="modes">What each mode reaches</h2>
      ${blocks.join("\n      ")}`;
}

// A `capped` or `partial` badge states that a mode falls short; this states of
// what, and why. Without it the badge names a verdict and hides the evidence.
function outOfReach(d, mode) {
  const out = Object.entries(d.modes?.[mode]?.unreachable ?? {});
  if (!out.length) return "";
  const items = out
    .sort(([a], [b]) => order(a) - order(b))
    .map(([key, reason]) => {
      const why = REASONS.get(reason) ?? reason;
      return `<li><strong>${escapeHtml(label(key))}</strong> — ${escapeHtml(why)}</li>`;
    })
    .join("");
  return `
      <ul class="out-of-reach">${items}</ul>`;
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
      <table>
        <thead><tr><th scope="col">Command</th><th scope="col">Arguments</th></tr></thead>
        <tbody>
          ${rows}
        </tbody>
      </table>`;
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
