// Syntax colour, applied when the site is built. Nothing ships to the
// browser: the classes are in the HTML and the palette is in `site.css`.

import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import yaml from "highlight.js/lib/languages/yaml";

import { escapeHtml } from "./html.mjs";

for (const [name, grammar] of Object.entries({ bash, javascript, json, python, rust, yaml })) {
  hljs.registerLanguage(name, grammar);
}

// The names the site writes, and the grammar each one reads.
const ALIAS = {
  cli: "bash",
  sh: "bash",
  shell: "bash",
  node: "javascript",
  js: "javascript",
  py: "python",
  yml: "yaml",
};

/**
 * Colours one block of code. A language the build carries no grammar for
 * comes back escaped and plain, which is what an unmarked fence gets.
 */
export function highlight(source, language) {
  const name = ALIAS[language] ?? language;
  if (!name || !hljs.getLanguage(name)) return escapeHtml(source);
  const html = hljs.highlight(source, { language: name, ignoreIllegals: true }).value;
  return name === "bash" ? command(html) : html;
}

// No grammar knows `govee`. It stands at the head of a line, so it is never
// inside a span already.
function command(html) {
  return html.replace(/^(\s*)(govee)\b/gm, '$1<span class="hljs-built_in">$2</span>');
}
