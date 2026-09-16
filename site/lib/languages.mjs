// The language tabs, shared by the reference page and the model pages. One
// choice holds for the whole page: `assets/js/tabs.js` reads `data-langs`.

import { highlight } from "./code.mjs";
import { escapeAttr } from "./html.mjs";

export const LANGUAGES = [
  { id: "cli", label: "Command line" },
  { id: "rust", label: "Rust" },
  { id: "python", label: "Python" },
  { id: "node", label: "Node.js" },
];

/**
 * One tab list and one pane per language the map holds, in the order above.
 * `prefix` makes the identifiers unique inside the page. Each tab names the
 * pane it controls, and each pane names its tab, so a screen reader on a pane
 * tells the reader which language it is.
 */
export function langBlock(prefix, examples) {
  const available = LANGUAGES.filter((lang) => examples[lang.id]);
  if (!available.length) return "";
  const id = (lang) => `${prefix}-${lang.id}`;
  const tabs = available
    .map((lang, index) => `<button type="button" role="tab" id="tab-${id(lang)}" aria-controls="pane-${id(lang)}" data-lang="${lang.id}" aria-selected="${index === 0}" tabindex="${index === 0 ? "0" : "-1"}">${lang.label}</button>`)
    .join("");
  const panes = available
    .map((lang, index) => {
      const source = examples[lang.id];
      return `<div class="pane" role="tabpanel" id="pane-${id(lang)}" aria-labelledby="tab-${id(lang)}" tabindex="0" data-lang="${lang.id}"${index === 0 ? "" : " hidden"}><div class="code"><pre><code>${highlight(source, lang.id)}</code></pre><button class="copy" type="button" data-copy="${escapeAttr(source)}">Copy</button></div></div>`;
    })
    .join("\n              ");
  return `<div class="langs" data-langs>
              <div class="tabs" role="tablist" aria-label="Language">${tabs}</div>
              ${panes}
            </div>`;
}
