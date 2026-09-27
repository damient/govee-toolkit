// The reference page: one entry per command, with an example in each language.
// It sits inside the documentation and carries the same frame.

import { base } from "./config.ts";
import { docShell } from "./docs.ts";
import { examples } from "./examples.ts";
import { escapeAttr, escapeHtml, filled, inline } from "./html.ts";
import { LANGUAGES, langBlock } from "./languages.ts";
import { modeBadge } from "./mode-badge.ts";
import { paramTables } from "./params.ts";
import type { Api, Mode, NavEntry, RefEntry, RefGroup, Reference } from "./types.ts";

/** Where the page sits in the documentation menu. */
export const REFERENCE: NavEntry = { url: "docs/reference/", title: "Reference", order: 5 };

/** The page, from `content/reference.json`, `dist/api.json` and the menu. */
export function referencePage(reference: Reference, api: Api, nav: NavEntry[]): string {
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
${reference.groups.map((group) => referenceGroup(group, api)).join("\n")}`,
  });
}

// A `modes` list names the modes that reach the item. It is absent where every
// mode reaches it, so a badge marks the exception and not the rule.
const modeMarks = (modes: Mode[] | undefined): string =>
  modes ? `<span class="ref-modes">${modes.map((mode) => modeBadge(mode)).join("")}</span>` : "";

function referenceGroup(group: RefGroup, api: Api): string {
  const entries = group.entries.map((entry) => referenceEntry(entry, api)).join("\n");
  return `        <section class="ref-group">
          <h2 id="${escapeAttr(group.id)}"><a class="anchor" href="#${escapeAttr(group.id)}">${escapeHtml(group.title)}</a>${modeMarks(group.modes)}</h2>
${entries}
        </section>`;
}

// The detail opens from a mark after the summary, on hover and on focus, so a
// tap on a phone opens it too.
function help(entry: RefEntry): string {
  if (!filled(entry.detail)) return "";
  const id = `tip-${entry.id}`;
  return `<button class="ref-help tip" type="button" aria-label="More about ${escapeAttr(entry.title)}" aria-describedby="${id}">?<span class="tip-box" role="tooltip" id="${id}">${inline(entry.detail)}</span></button>`;
}

function referenceEntry(entry: RefEntry, api: Api): string {
  const tables = paramTables(entry, api, LANGUAGES.map((lang) => lang.id));
  return `          <article class="ref-entry" id="${escapeAttr(entry.id)}">
            <div class="ref-head">
              <h3><a class="anchor" href="#${entry.id}"><code>${escapeHtml(entry.title)}</code></a></h3>
              <p class="summary">${escapeHtml(entry.summary)}${help(entry)}</p>${modeMarks(entry.modes)}
            </div>
            ${langBlock(entry.id, examples(entry, {}, null), tables)}
          </article>`;
}
