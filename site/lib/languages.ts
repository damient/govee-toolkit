// `src/assets/ts/tabs.ts` reads `data-langs`.

import { highlight } from "./code.ts";
import { escapeAttr, filled } from "./html.ts";
import type { Examples } from "./types.ts";

export const LANGUAGES = [
  { id: "rust", label: "Rust" },
  { id: "cli", label: "CLI" },
  { id: "python", label: "Python" },
  { id: "node", label: "Node.js" },
];

// Selected when the block holds it and the reader has not chosen another.
const DEFAULT_LANGUAGE = "cli";

/**
 * One tab list and one pane per language the map holds, in the order above.
 * `prefix` makes the identifiers unique inside the page. Each tab names the
 * pane it controls, and each pane names its tab, so a screen reader on a pane
 * tells the reader which language it is.
 */
export function langBlock(prefix: string, examples: Examples): string {
  const available = LANGUAGES.filter((lang) => filled(examples[lang.id]));
  if (available.length === 0) return "";
  const selected = available.find((lang) => lang.id === DEFAULT_LANGUAGE)?.id ?? available[0]?.id;
  const id = (lang: { id: string }) => `${prefix}-${lang.id}`;
  const tabs = available
    .map((lang) => `<button type="button" role="tab" id="tab-${id(lang)}" aria-controls="pane-${id(lang)}" data-lang="${lang.id}" aria-selected="${lang.id === selected}" tabindex="${lang.id === selected ? "0" : "-1"}">${lang.label}</button>`)
    .join("");
  const panes = available
    .map((lang) => {
      const source = examples[lang.id] ?? "";
      return `<div class="pane" role="tabpanel" id="pane-${id(lang)}" aria-labelledby="tab-${id(lang)}" tabindex="0" data-lang="${lang.id}"${lang.id === selected ? "" : " hidden"}><div class="code"><pre><code>${highlight(source, lang.id)}</code></pre><button class="copy" type="button" data-copy="${escapeAttr(source)}">Copy</button></div></div>`;
    })
    .join("\n              ");
  return `<div class="langs" data-langs="${selected}">
              <div class="tabs" role="tablist" aria-label="Language">${tabs}</div>
              ${panes}
            </div>`;
}
