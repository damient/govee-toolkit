// `xtask lan` writes `docs/lan-supported-devices.md` from the same JSON file.

import { lanListPath } from "./config.ts";
import { escapeAttr, escapeHtml, fill } from "./html.ts";
import { isLanList, readJson } from "./json.ts";
import type { Device, LanList } from "./types.ts";

export function readLanList(): Promise<LanList> {
  return readJson(lanListPath, isLanList);
}

// A candidate alias is not verified, so it gets no link.
function pages(devices: Device[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const d of devices) {
    for (const sku of [d.sku, ...(d.aliases ?? [])]) map.set(sku, d.sku);
  }
  return map;
}

function categories(models: LanList["models"]): string {
  const counts = new Map<string, number>();
  for (const m of models) counts.set(m.category, (counts.get(m.category) ?? 0) + 1);
  return [...counts]
    .toSorted(([a], [b]) => a.localeCompare(b))
    .map(([name, n]) => `<option value="${escapeAttr(name)}">${escapeHtml(name)} (${n})</option>`)
    .join("\n          ");
}

export function renderLanList(template: string, devices: Device[], list: LanList): string {
  const known = pages(devices);
  const models = list.models.toSorted((a, b) => a.sku.localeCompare(b.sku));
  const rows = models.map((m) => {
    const page = known.get(m.sku);
    const sku = page === undefined
      ? escapeHtml(m.sku)
      : `<a href="{{base}}devices/${escapeAttr(page)}/">${escapeHtml(m.sku)}</a>`;
    const here = page === undefined
      ? '<a class="muted" href="{{base}}devices/add/">Add it</a>'
      : `<a href="{{base}}devices/${escapeAttr(page)}/">${page === m.sku ? "Model page" : `Model page (${escapeHtml(page)})`}</a>`;
    const search = [m.sku, m.name, m.category].join(" ").toLowerCase();
    return `          <tr data-search="${escapeAttr(search)}" data-category="${escapeAttr(m.category)}"${page === undefined ? "" : ' class="has-page"'}>
            <th scope="row">${sku}</th>
            <td>${escapeHtml(m.name)}</td>
            <td>${escapeHtml(m.category)}</td>
            <td>${here}</td>
          </tr>`;
  });
  return fill(template, {
    lan_rows: rows.join("\n"),
    lan_categories: categories(models),
    lan_count: String(models.length),
    lan_paged: String(models.filter((m) => known.has(m.sku)).length),
    lan_source: escapeAttr(list.source),
    lan_retrieved: escapeHtml(list.retrieved),
  });
}
