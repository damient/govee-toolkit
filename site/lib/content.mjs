// The documentation pages: one Markdown file under `content/docs/`, one page.

import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { Marked } from "marked";
import { highlight } from "./code.mjs";
import { base, repoUrl, root } from "./config.mjs";
import { docShell } from "./docs.mjs";
import { escapeAttr, fill, slugify } from "./html.mjs";
import { modeBadges } from "./mode-badge.mjs";

/** Every documentation page, in menu order. */
export async function readDocs() {
  const dir = join(root, "content/docs");
  const files = (await readdir(dir)).filter((f) => f.endsWith(".md"));
  const raw = await Promise.all(files.map((f) => readFile(join(dir, f), "utf8")));
  const docs = files.map((file, at) => renderDoc(file, raw[at]));
  return docs.sort((a, b) => a.order - b.order);
}

/** One documentation page, inside the frame every one of them shares. */
export function docPage(doc, nav) {
  return docShell({
    base,
    nav,
    current: `docs/${doc.slug}/`,
    toc: doc.headings.map((h) => ({ id: h.id, text: h.text })),
    body: doc.html,
  });
}

function renderDoc(file, raw) {
  const { meta, body } = frontMatter(raw);
  // The heading renderer fills this: the id it writes into the anchor is the
  // id the table of contents links to, derived once.
  const headings = [];
  const md = new Marked({ async: false });
  md.use({
    renderer: {
      code({ text, lang }) {
        const name = (lang ?? "").trim().split(/\s+/)[0];
        return `<pre><code>${highlight(text, name)}</code></pre>\n`;
      },
      heading({ depth, tokens }) {
        const inner = this.parser.parseInline(tokens);
        if (depth !== 2) return `<h${depth}>${inner}</h${depth}>\n`;
        const label = headingLabel(tokens);
        const id = slugify(label);
        headings.push({ id, text: label });
        return `<h2 id="${escapeAttr(id)}"><a class="anchor" href="#${escapeAttr(id)}">${inner}</a></h2>\n`;
      },
    },
  });
  const html = md.parse(fill(body, { base, repo: repoUrl, ...modeBadges() }));
  return {
    slug: meta.slug ?? file.replace(/\.md$/, ""),
    title: meta.title ?? file,
    description: meta.description ?? "",
    order: Number(meta.order ?? 99),
    faq: meta.faq === "true",
    headings,
    html,
    sections: sections(html, headings),
  };
}

// The heading is the question, so the answer must not repeat it: the cut
// starts after the `</h2>`.
function sections(html, headings) {
  const parts = html.split(/<h2 id="[^"]*">/).slice(1);
  return headings.map((heading, index) => {
    const part = parts[index] ?? "";
    const close = part.indexOf("</h2>");
    return {
      title: heading.text,
      answer: plainText(close === -1 ? part : part.slice(close + 5)),
    };
  });
}

function plainText(html) {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">")
    .replace(/&#39;|&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim();
}

// A heading can end with inline HTML, a state badge for one. The anchor and
// the table of contents take what stands before it.
function headingLabel(tokens = []) {
  const words = [];
  for (const token of tokens) {
    if (token.type === "html") break;
    words.push(token.text ?? token.raw ?? "");
  }
  return words.join("").trim();
}

function frontMatter(raw) {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) return { meta: {}, body: raw };
  const meta = {};
  for (const line of match[1].split("\n")) {
    const at = line.indexOf(":");
    if (at > 0) meta[line.slice(0, at).trim()] = line.slice(at + 1).trim();
  }
  return { meta, body: raw.slice(match[0].length) };
}
