// The helpers every renderer shares.

import { Marked } from "marked";

const marked = new Marked({ async: false });

/** Replaces every `{{ name }}` the map holds. An unknown name is left alone. */
export function fill(template, vars) {
  return template.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (all, key) =>
    key in vars ? String(vars[key]) : all);
}

/** Escapes the three characters that change the meaning of markup. */
export function escapeHtml(text) {
  return String(text).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);
}

/** Escapes for an attribute value that double quotes delimit. */
export function escapeAttr(text) {
  return escapeHtml(text).replace(/"/g, "&quot;");
}

/**
 * Renders one line of Markdown, without a paragraph around it. The device
 * files write a command name and a byte sequence between backticks, and this
 * is what turns those into `<code>`. Marked escapes the markup itself.
 */
export function inline(text) {
  return marked.parseInline(String(text));
}

/** Turns a heading into the identifier its anchor uses. */
export function slugify(text) {
  return text.toLowerCase().replace(/[^\w]+/g, "-").replace(/^-|-$/g, "");
}
