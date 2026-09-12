// The reference page: one entry per command, with an example in each language.
// It sits inside the documentation and carries the same frame.

import { highlight } from "./code.mjs";
import { base } from "./config.mjs";
import { docShell } from "./docs.mjs";
import { escapeAttr, escapeHtml, inline } from "./html.mjs";

/** Where the page sits in the documentation menu. */
export const REFERENCE = { url: "reference/", title: "Reference", order: 4 };

// `planned` marks a package that has no code yet: the pane says so over the
// example.
const LANGUAGES = [
  { id: "cli", label: "Command line" },
  { id: "rust", label: "Rust" },
  { id: "python", label: "Python", planned: true },
  { id: "node", label: "Node.js", planned: true },
];

/** The page, from `content/reference.json` and the documentation menu. */
export function referencePage(reference, nav) {
  const toc = reference.groups.map((group) => ({
    id: group.id,
    text: group.title,
    children: group.entries.map((entry) => ({ id: entry.id, text: entry.title, code: true })),
  }));

  return docShell({
    base,
    nav,
    current: REFERENCE.url,
    toc,
    klass: "reference",
    body: `<h1>Reference</h1>
        <p class="lede">${escapeHtml(reference.intro)}</p>
${reference.groups.map(referenceGroup).join("\n")}`,
  });
}

function referenceGroup(group) {
  const entries = group.entries.map(referenceEntry).join("\n");
  return `        <section class="ref-group">
          <h2 id="${escapeAttr(group.id)}"><a class="anchor" href="#${escapeAttr(group.id)}">${escapeHtml(group.title)}</a></h2>
${entries}
        </section>`;
}

// Each tab names the panel it controls, and each panel names its tab, so a
// screen reader on a panel tells the reader which language it is.
function referenceEntry(entry) {
  const available = LANGUAGES.filter((lang) => entry.examples[lang.id]);
  const id = (lang) => `${entry.id}-${lang.id}`;
  const tabs = available
    .map((lang, index) => `<button type="button" role="tab" id="tab-${id(lang)}" aria-controls="pane-${id(lang)}" data-lang="${lang.id}" aria-selected="${index === 0}" tabindex="${index === 0 ? "0" : "-1"}">${lang.label}</button>`)
    .join("");
  const panes = available
    .map((lang, index) => {
      const planned = lang.planned
        ? `<p class="planned">Planned — the shape it will have.</p>`
        : "";
      const source = entry.examples[lang.id];
      return `<div class="pane" role="tabpanel" id="pane-${id(lang)}" aria-labelledby="tab-${id(lang)}" tabindex="0" data-lang="${lang.id}"${index === 0 ? "" : " hidden"}>${planned}<pre><code>${highlight(source, lang.id)}</code></pre><button class="copy" type="button" data-copy="${escapeAttr(source)}">Copy</button></div>`;
    })
    .join("\n              ");
  const detail = entry.detail ? `<p>${inline(entry.detail)}</p>` : "";
  return `          <article class="ref-entry" id="${escapeAttr(entry.id)}">
            <div class="ref-text">
              <h3><a class="anchor" href="#${entry.id}"><code>${escapeHtml(entry.title)}</code></a></h3>
              <p class="summary">${escapeHtml(entry.summary)}</p>
              ${detail}
            </div>
            <div class="langs" data-langs>
              <div class="tabs" role="tablist" aria-label="Language">${tabs}</div>
              ${panes}
            </div>
          </article>`;
}
