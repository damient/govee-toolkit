// The text of the device file stays in English: a name, a family, a note.

import { deviceTrail } from "./categories.ts";
import { MODES } from "./config.ts";
import { crumbs } from "./crumbs.ts";
import { dmx } from "./device-dmx.ts";
import { usage } from "./device-usage.ts";
import { CHECK, hasDmx, label, order, reached, support } from "./devices.ts";
import { escapeAttr, escapeHtml, filled } from "./html.ts";
import { icon } from "./icons.ts";
import { listedFile, listedVerification } from "./listed.ts";
import type { T } from "./locales.ts";
import { categoryBadge, deviceBadge, dmxBadge, familyBadge, modeBadge } from "./mode-badge.ts";
import type { Capability, Crumb, Device, Mode, Reference } from "./types.ts";

/**
 * Builds the page of one model. The caller emits it: this returns the URL, the
 * title, the description and the body, and never touches the file system.
 */
export function devicePage(d: Device, reference: Reference, pages: Device[], t: T) {
  const trail = deviceTrail(d, t);
  return {
    url: `devices/${d.sku}/`,
    title: `Govee ${d.sku} — ${d.name}`,
    description: describe(d, t),
    body: pageBody(d, reference, trail, pages, t),
    breadcrumb: trail,
  };
}

function describe(d: Device, t: T): string {
  const paths = reached(d).map((m: Mode) => t(`model.path_${m}`));
  const names = { sku: d.sku, name: d.name };
  const what = paths.length > 0
    ? t("model.description_reach", { ...names, paths: joined(paths, t), dmx: hasDmx(d) ? t("model.description_dmx") : "" })
    : t("model.description_probe", names);
  if (filled(d.verified?.date)) return `${what} ${t("model.description_verified", { date: d.verified.date })}`;
  return d.listing === undefined ? what : `${what} ${t("model.description_untested")}`;
}

function joined(words: string[], t: T): string {
  return words.length < 2 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} ${t("model.and")} ${words.at(-1)}`;
}

function pageBody(d: Device, reference: Reference, trail: Crumb[], pages: Device[], t: T): string {
  const title = d.name ? `${d.sku} — ${escapeHtml(d.name)}` : d.sku;
  const [family, check, file] = d.listing === undefined
    ? [familyBadge(d.family), verification(d, t), deviceFile(d, t)]
    : ["", listedVerification(t), listedFile(d, t)];

  return `<section class="pagehead">
  <div class="shell">
    ${crumbs(trail, t)}
    <h1>${title}</h1>
    <p class="mode-line">${categoryBadge(d.category, d.shape ?? d.family, t)}${family}${reached(d).map((m) => modeBadge(m)).join("")}${hasDmx(d) ? dmxBadge() : ""}${badges(d, t)}</p>
  </div>
</section>

<section class="slab">
  <div class="shell">
    <article class="prose">
      ${capabilities(d, t)}
      ${modeSections(d, t)}
      ${usage(d, reference, t)}
      ${dmx(d, t)}
      ${aliases(d, pages, t)}
      <div class="endgrid">
        <section>${check}</section>
        <section>${file}</section>
      </div>
      <p class="endcta"><a class="btn" href="{{base}}devices/">${escapeHtml(t("model.discover"))}</a></p>
    </article>
  </div>
</section>`;
}

function deviceFile(d: Device, t: T): string {
  const link = `<a href="{{repo}}/blob/main/devices/${d.sku}.yaml"><code>devices/${d.sku}.yaml</code></a>`;
  return `<h2 id="the-device-file">${escapeHtml(t("model.file_title"))}</h2>
          <p>${t("model.file_text", { file: link })}</p>`;
}

function badges(d: Device, t: T): string {
  return filled(d.verified?.date)
    ? `<span class="badge badge-ok">${CHECK}${escapeHtml(t("model.verified"))}</span>`
    : `<span class="badge badge-unknown">${escapeHtml(t("model.not_verified"))}</span>`;
}

function capabilities(d: Device, t: T): string {
  const caps = Object.entries(d.capabilities ?? {});
  if (caps.length === 0) return "";
  const list = caps
    .toSorted(([a], [b]) => order(a) - order(b))
    .map(([key, value]) => `<li>${icon(key)}${escapeHtml(label(key, t))}${counts(key, value, d, t)}</li>`)
    .join("");
  return `<h2 id="what-the-hardware-does">${escapeHtml(t("model.hardware"))}</h2>
      <ul class="caps caps-lg">${list}</ul>`;
}

// Where one zone is one LED, the grid carries the shape that the counts do not.
function counts(key: string, value: Capability | null, device: Device, t: T): string {
  if (key !== "segments" || !value) return "";
  const parts: string[] = [];
  const grid = device.measurements?.segment_grid;
  const oneZonePerPixel = (value.count ?? 0) > 0 && value.count === value.native_pixels;
  if (oneZonePerPixel && Array.isArray(grid) && grid.length === 2) {
    parts.push(escapeHtml(t("model.grid", { cols: String(grid[0]), rows: String(grid[1]) })));
  } else if ((value.count ?? 0) > 0) {
    parts.push(escapeHtml(t("model.zones", { count: value.count ?? 0 })));
  }
  if ((value.native_pixels ?? 0) > 0) parts.push(escapeHtml(t("model.pixels", { count: value.native_pixels ?? 0 })));
  return parts.length > 0
    ? `<span class="badge-count"><span aria-hidden="true">|</span> ${parts.join(" · ")}</span>`
    : "";
}

function modeCaps(d: Device, mode: Mode, t: T): string {
  const caps = (d.modes?.[mode]?.capabilities ?? []).toSorted((a, b) => order(a) - order(b));
  if (caps.length > 0) {
    const chips = caps.map((c) => `<li>${icon(c)}${escapeHtml(label(c, t))}</li>`).join("");
    return `<ul class="caps">${chips}</ul>`;
  }
  const probed = support(d, mode) !== "unknown";
  return `<span class="mode-note">${escapeHtml(t(probed ? "model.not_available" : "model.not_probed"))}</span>`;
}

function modeSections(d: Device, t: T): string {
  const rows = MODES.map(
    (m) => `<tr><th scope="row">${modeBadge(m)}</th>
            <td>${modeCaps(d, m, t)}</td></tr>`,
  );

  return `<h2 id="modes">${escapeHtml(t("model.modes"))}</h2>
      <div class="tablewrap">
        <table class="matrix matrix-modes">
          <thead><tr><th scope="col">${escapeHtml(t("model.mode"))}</th><th scope="col">${escapeHtml(t("model.capabilities"))}</th></tr></thead>
          <tbody>
            ${rows.join("\n            ")}
          </tbody>
        </table>
      </div>`;
}

// A link goes both ways: a model also lists the file that names it and its other SKUs.
function related(d: Device, pages: Device[]): string[] {
  const skus = new Set([...(d.aliases ?? []), ...(d.candidate_aliases ?? [])]);
  for (const o of pages) {
    const group = [o.sku, ...(o.aliases ?? []), ...(o.candidate_aliases ?? [])];
    if (!group.includes(d.sku) || o.sku === d.sku) continue;
    for (const s of group) skus.add(s);
  }
  skus.delete(d.sku);
  return [...skus];
}

function aliases(d: Device, pages: Device[], t: T): string {
  const skus = related(d, pages);
  if (skus.length === 0) return "";
  const item = (sku: string): string => {
    const page = pages.find((o) => o.sku === sku);
    return page === undefined
      ? deviceBadge(sku)
      : deviceBadge(sku, page.name, page.shape ?? page.family, `{{base}}devices/${sku}/`);
  };
  return `<h2 id="similar-devices">${escapeHtml(t("model.similar"))}</h2>
      <p class="similar">${skus.map((s) => item(s)).join("")}</p>`;
}

function verification(d: Device, t: T): string {
  const head = `<h2 id="verification">${escapeHtml(t("model.verification"))}</h2>`;
  if (!filled(d.verified?.date)) return `${head}\n      <p>${t("model.unverified")}</p>`;
  const firmware = filled(d.verified.firmware)
    ? t("model.on_firmware", { firmware: `<code>${escapeHtml(d.verified.firmware)}</code>` })
    : "";
  const by = filled(d.verified.by) ? t("model.by", { who: `<code>${escapeHtml(d.verified.by)}</code>` }) : "";
  const date = `<time datetime="${escapeAttr(d.verified.date)}">${escapeHtml(d.verified.date)}</time>`;
  return `${head}\n      <p>${t("model.verified_on", { by, date, firmware })}</p>`;
}
