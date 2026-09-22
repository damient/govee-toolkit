// Syntax colour, applied when the site is built. Nothing ships to the
// browser: the classes are in the HTML and the palette is in `site.css`.

import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import yaml from "highlight.js/lib/languages/yaml";

import { escapeAttr, escapeHtml } from "./html.ts";

for (const [name, grammar] of Object.entries({ bash, javascript, json, python, rust, yaml })) {
  hljs.registerLanguage(name, grammar);
}

// The names the site writes, and the grammar each one reads.
const ALIAS: Record<string, string> = {
  cli: "bash",
  sh: "bash",
  shell: "bash",
  node: "javascript",
  js: "javascript",
  py: "python",
  yml: "yaml",
};

/**
 * One block of code in the frame every block on the site takes, with the
 * button `copy()` fills. `html` is the coloured block and `source` is what
 * the button writes to the clipboard.
 */
export function codeBlock(html: string, source: string): string {
  return `<div class="terminal">\n<pre><code>${html}</code></pre>\n`
    + `<button class="copy" type="button" data-copy="${escapeAttr(source)}">Copy</button>\n</div>\n`;
}

/**
 * Colours one block of code. A language the build carries no grammar for
 * comes back escaped and plain, which is what an unmarked fence gets.
 */
export function highlight(source: string, language: string): string {
  const name = ALIAS[language] ?? language;
  if (!name || !hljs.getLanguage(name)) return escapeHtml(source);
  const html = hljs.highlight(source, { language: name, ignoreIllegals: true }).value;
  return name === "bash" ? command(html) : html;
}

// No grammar knows `govee`. It stands at the head of a line, so it is never
// inside a span already.
function command(html: string): string {
  return html.replaceAll(/^(\s*)(govee)\b/gmu, '$1<span class="hljs-built_in">$2</span>');
}
