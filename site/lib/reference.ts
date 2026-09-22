// The reference page: one entry per command, with an example in each language.
// It sits inside the documentation and carries the same frame.

import { base } from "./config.ts";
import { docShell } from "./docs.ts";
import { examples } from "./examples.ts";
import { escapeAttr, escapeHtml, filled, inline } from "./html.ts";
import { langBlock } from "./languages.ts";
import { modeBadge } from "./mode-badge.ts";
import type { Mode, NavEntry, RefEntry, RefGroup, Reference } from "./types.ts";

/** Where the page sits in the documentation menu. */
export const REFERENCE: NavEntry = { url: "reference/", title: "Reference", order: 4 };

/** The page, from `content/reference.json` and the documentation menu. */
export function referencePage(reference: Reference, nav: NavEntry[]): string {
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
    collapse: true,
    klass: "reference",
    body: `<h1>Reference</h1>
        <p class="lede">${escapeHtml(reference.intro)}</p>
${reference.groups.map(referenceGroup).join("\n")}`,
  });
}

// A `modes` list names the modes that reach the item. It is absent where every
// mode reaches it, so a badge marks the exception and not the rule.
const modeMarks = (modes: Mode[] | undefined): string =>
  modes ? `<span class="ref-modes">${modes.map((mode) => modeBadge(mode)).join("")}</span>` : "";

function referenceGroup(group: RefGroup): string {
  const entries = group.entries.map(referenceEntry).join("\n");
  return `        <section class="ref-group">
          <h2 id="${escapeAttr(group.id)}"><a class="anchor" href="#${escapeAttr(group.id)}">${escapeHtml(group.title)}</a>${modeMarks(group.modes)}</h2>
${entries}
        </section>`;
}

function referenceEntry(entry: RefEntry): string {
  const detail = filled(entry.detail) ? `<p>${inline(entry.detail)}</p>` : "";
  return `          <article class="ref-entry" id="${escapeAttr(entry.id)}">
            <div class="ref-text">
              <h3><a class="anchor" href="#${entry.id}"><code>${escapeHtml(entry.title)}</code></a>${modeMarks(entry.modes)}</h3>
              <p class="summary">${escapeHtml(entry.summary)}</p>
              ${detail}
            </div>
            ${langBlock(entry.id, examples(entry, {}, null))}
          </article>`;
}
