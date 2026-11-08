// The parameter tables of a reference entry, one for each language. The rows
// come from `dist/api.json`, which `xtask api` reads from the code, so a table
// follows the signature it describes.

import { CHEVRON } from "./docs.ts";
import { escapeHtml } from "./html.ts";
import type { Api, ApiParam, RefEntry } from "./types.ts";

/**
 * The methods one language shows for the entry. The CLI names its own; Python
 * and Node.js take the methods that carry the Rust ones. Throws on a method
 * that the file does not hold, so a renamed method fails the build.
 */
function methods(entry: RefEntry, api: Api, lang: string): string[] {
  const named = lang === "cli" ? (entry.cli ?? []) : lang === "rust" ? (entry.api ?? []) : null;
  const found = named ?? (entry.api ?? []).flatMap((rust) => {
    const carried = api.surfaces[rust]?.[lang];
    if (!carried) throw new Error(`reference.json: \`${entry.id}\` names \`${rust}\`, which dist/api.json does not hold`);
    return carried;
  });
  for (const method of found) {
    if (!api.params[lang]?.[method]) {
      throw new Error(`reference.json: \`${entry.id}\` names \`${method}\`, which dist/api.json does not hold for ${lang}`);
    }
  }
  return found;
}

const cell = (text: string | null): string => (text === null ? "—" : `<code>${escapeHtml(text)}</code>`);

function table(rows: ApiParam[]): string {
  const body = rows
    .map((row) => `<tr><td>${cell(row.name)}</td><td>${cell(row.type)}</td><td>${row.required ? "required" : "optional"}</td><td>${cell(row.default)}</td></tr>`)
    .join("");
  return `<table class="ref-table"><thead><tr><th>Name</th><th>Type</th><th>Required</th><th>Default</th></tr></thead><tbody>${body}</tbody></table>`;
}

/** The tables of each language, for the methods that take a parameter. */
export function paramTables(entry: RefEntry, api: Api, langs: string[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const lang of langs) {
    // A method that takes no parameter carries no block.
    const blocks = methods(entry, api, lang)
      .map((method) => [method, api.params[lang]?.[method] ?? []] as const)
      .filter(([, rows]) => rows.length > 0)
      .map(([method, rows]) => `<div class="ref-params"><p class="ref-sig"><code>${escapeHtml(method)}</code></p>${table(rows)}</div>`);
    if (blocks.length > 0) out[lang] = `<details class="ref-fold"><summary>Parameters${CHEVRON}</summary>${blocks.join("")}</details>`;
  }
  return out;
}
