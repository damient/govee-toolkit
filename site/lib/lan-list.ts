// Renders the list of models that carry the LAN switch, from
// `docs/lan-supported-devices.json`. `xtask lan` writes the table in
// `docs/lan-supported-devices.md` from the same file.

import { readFile } from "node:fs/promises";
import { lanListPath } from "./config.ts";
import { escapeAttr, escapeHtml, fill } from "./html.ts";
import type { Device, LanList } from "./types.ts";

export async function readLanList(): Promise<LanList> {
  return JSON.parse(await readFile(lanListPath, "utf8"));
}

// An alias is verified to behave as its model, so it shares the model page. A
// candidate alias is not, and it gets no link.
function pages(devices: Device[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const d of devices) {
    for (const sku of [d.sku, ...(d.aliases ?? [])]) map.set(sku, d.sku);
  }
  return map;
}

// One option per category, with its model count, in name order.
function categories(models: LanList["models"]): string {
  const counts = new Map<string, number>();
  for (const m of models) counts.set(m.category, (counts.get(m.category) ?? 0) + 1);
  return [...counts]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, n]) => `<option value="${escapeAttr(name)}">${escapeHtml(name)} (${n})</option>`)
    .join("\n          ");
}

export function renderLanList(template: string, devices: Device[], list: LanList): string {
  const known = pages(devices);
  const models = [...list.models].sort((a, b) => a.sku.localeCompare(b.sku));
  const rows = models.map((m) => {
    const page = known.get(m.sku);
    const sku = page
      ? `<a href="{{base}}devices/${escapeAttr(page)}/">${escapeHtml(m.sku)}</a>`
      : escapeHtml(m.sku);
    const here = page
      ? `<a href="{{base}}devices/${escapeAttr(page)}/">${page === m.sku ? "Model page" : `Model page (${escapeHtml(page)})`}</a>`
      : '<a class="muted" href="{{base}}devices/add/">Add it</a>';
    const search = [m.sku, m.name, m.category].join(" ").toLowerCase();
    return `          <tr data-search="${escapeAttr(search)}" data-category="${escapeAttr(m.category)}"${page ? ' class="has-page"' : ""}>
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
