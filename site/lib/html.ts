// The helpers every renderer shares.

import { Marked } from "marked";

const marked = new Marked({ async: false });

/** Replaces every `{{ name }}` the map holds. An unknown name is left alone. */
export function fill(template: string, vars: Record<string, unknown>): string {
  return template.replaceAll(/\{\{\s*([\w.]+)\s*\}\}/gu, (all, key: string) =>
    key in vars ? String(vars[key]) : all);
}

/** True where the text is set and not empty. */
export function filled(text: string | undefined): text is string {
  return text !== undefined && text !== "";
}

/** Escapes the three characters that change the meaning of markup. */
const ENTITIES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;" };

export function escapeHtml(text: unknown): string {
  return String(text).replaceAll(/[&<>]/gu, (c) => ENTITIES[c] ?? c);
}

/** Escapes for an attribute value that double quotes delimit. */
export function escapeAttr(text: unknown): string {
  return escapeHtml(text).replaceAll('"', "&quot;");
}

/**
 * Renders one line of Markdown, without a paragraph around it. The device
 * files write a command name and a byte sequence between backticks, and this
 * is what turns those into `<code>`. Marked escapes the markup itself.
 */
export function inline(text: unknown): string {
  return marked.parseInline(String(text), { async: false });
}

/** Turns a heading into the identifier its anchor uses. */
export function slugify(text: string): string {
  return text.toLowerCase().replaceAll(/[^\w]+/gu, "-").replaceAll(/^-|-$/gu, "");
}

/** What a cell shows where the catalog carries no answer. */
export const DASH = '<span class="muted">—</span>';
