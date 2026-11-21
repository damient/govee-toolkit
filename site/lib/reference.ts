// The reference page: one entry per command, with an example in each language.
// It sits inside the documentation and carries the same frame. Another language
// translates the text of `content/reference.json` in `content/<id>/reference.json`.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { base } from "./config.ts";
import { docShell } from "./docs.ts";
import { examples } from "./examples.ts";
import { escapeAttr, escapeHtml, filled, inline } from "./html.ts";
import { LANGUAGES, langBlock } from "./languages.ts";
import { isRecord } from "./json.ts";
import { ENGLISH, type Locale, type T, contentDir } from "./locales.ts";
import { modeBadge } from "./mode-badge.ts";
import { paramTables } from "./params.ts";
import type { Api, Mode, NavEntry, RefEntry, RefGroup, Reference } from "./types.ts";

/** Where the page sits in the documentation menu. */
export const REFERENCE_URL = "docs/reference/";
export const referenceNav = (t: T): NavEntry => ({ url: REFERENCE_URL, title: t("ref.title"), order: 5 });

/** The text of the reference in another language. Each key names an `id` of `reference.json`. */
interface Overlay {
  intro?: string;
  groups?: Record<string, string>;
  entries?: Record<string, { summary?: string; detail?: string; action?: { title?: string; summary?: string } }>;
}

// The guard checks the top level only, as the guards of `json.ts` do.
function isOverlay(value: unknown): value is Overlay {
  return isRecord(value) && (value.entries === undefined || isRecord(value.entries));
}

/**
 * The reference in one language, or `null` where the language does not carry
 * it. A text that the language lacks stays in English: `tools/translations.ts`
 * reports it.
 */
export function localReference(reference: Reference, locale: Locale): Reference | null {
  if (locale.id === ENGLISH.id) return reference;
  const path = join(contentDir(locale), "reference.json");
  if (!existsSync(path)) return null;
  const parsed: unknown = JSON.parse(readFileSync(path, "utf8"));
  if (!isOverlay(parsed)) throw new Error(`${path}: unexpected shape`);
  const overlay = parsed;
  return {
    ...reference,
    intro: overlay.intro ?? reference.intro,
    groups: reference.groups.map((group) => ({
      ...group,
      title: overlay.groups?.[group.id] ?? group.title,
      entries: group.entries.map((entry) => {
        const own = overlay.entries?.[entry.id] ?? {};
        return Object.assign({}, entry, {
          summary: own.summary ?? entry.summary,
          ...(entry.detail === undefined ? {} : { detail: own.detail ?? entry.detail }),
          ...(entry.action === undefined ? {} : { action: localAction(entry.action, own.action) }),
        });
      }),
    })),
  };
}

// The block a model page draws for the entry, under "What you can send".
function localAction(action: NonNullable<RefEntry["action"]>, own: { title?: string; summary?: string } | undefined) {
  return Object.assign({}, action, {
    title: own?.title ?? action.title,
    ...(action.summary === undefined ? {} : { summary: own?.summary ?? action.summary }),
  });
}

/** The page, from `content/reference.json`, `dist/api.json` and the menu. */
export function referencePage(reference: Reference, api: Api, nav: NavEntry[], t: T): string {
  const toc = reference.groups.map((group) => ({
    id: group.id,
    text: group.title,
    children: group.entries.map((entry) => ({ id: entry.id, text: entry.title, code: true })),
  }));

  return docShell({
    t,
    base,
    nav,
    current: REFERENCE_URL,
    toc,
    collapse: true,
    klass: "reference",
    body: `<h1>${escapeHtml(t("ref.title"))}</h1>
        <p class="lede">${escapeHtml(reference.intro)}</p>
${reference.groups.map((group) => referenceGroup(group, api, t)).join("\n")}`,
  });
}

// A `modes` list names the modes that reach the item. It is absent where every
// mode reaches it, so a badge marks the exception and not the rule.
const modeMarks = (modes: Mode[] | undefined): string =>
  modes ? `<span class="ref-modes">${modes.map((mode) => modeBadge(mode)).join("")}</span>` : "";

function referenceGroup(group: RefGroup, api: Api, t: T): string {
  const entries = group.entries.map((entry) => referenceEntry(entry, api, t)).join("\n");
  return `        <section class="ref-group">
          <h2 id="${escapeAttr(group.id)}"><a class="anchor" href="#${escapeAttr(group.id)}">${escapeHtml(group.title)}</a>${modeMarks(group.modes)}</h2>
${entries}
        </section>`;
}

// A button, so that a tap on a phone opens the detail too.
function help(entry: RefEntry, t: T): string {
  if (!filled(entry.detail)) return "";
  const id = `tip-${entry.id}`;
  return `<button class="ref-help tip" type="button" aria-label="${escapeAttr(t("ref.more", { name: entry.title }))}" aria-describedby="${id}">?<span class="tip-box" role="tooltip" id="${id}">${inline(entry.detail)}</span></button>`;
}

function referenceEntry(entry: RefEntry, api: Api, t: T): string {
  const tables = paramTables(entry, api, LANGUAGES.map((lang) => lang.id), t);
  return `          <article class="ref-entry" id="${escapeAttr(entry.id)}">
            <div class="ref-head">
              <h3><a class="anchor" href="#${entry.id}"><code>${escapeHtml(entry.title)}</code></a></h3>
              <p class="summary">${escapeHtml(entry.summary)}${help(entry, t)}</p>${modeMarks(entry.modes)}
            </div>
            ${langBlock(entry.id, examples(entry, {}, null), tables, t)}
          </article>`;
}
