// The documentation pages: one Markdown file under `content/docs/`, one page.

import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { Marked, type Token, type Tokens } from "marked";
import { codeBlock, highlight } from "./code.ts";
import { base, repoUrl, root } from "./config.ts";
import { docShell } from "./docs.ts";
import { escapeAttr, fill, slugify } from "./html.ts";
import { modeBadges } from "./mode-badge.ts";
import type { NavEntry } from "./types.ts";
import { versionChips } from "./versions.ts";

interface Heading {
  id: string;
  text: string;
}

/** One documentation page, read from its Markdown file. */
export interface Doc {
  slug: string;
  title: string;
  description: string;
  order: number;
  faq: boolean;
  headings: Heading[];
  html: string;
  sections: { title: string; answer: string }[];
}

/** Every documentation page, in menu order. */
export async function readDocs(): Promise<Doc[]> {
  const dir = join(root, "content/docs");
  const files = (await readdir(dir)).filter((f) => f.endsWith(".md"));
  const docs = await Promise.all(
    files.map(async (file) => renderDoc(file, await readFile(join(dir, file), "utf8"))),
  );
  return docs.sort((a, b) => a.order - b.order);
}

/** One documentation page, inside the frame every one of them shares. */
export function docPage(doc: Doc, nav: NavEntry[]): string {
  return docShell({
    base,
    nav,
    current: `docs/${doc.slug}/`,
    toc: doc.headings.map((h) => ({ id: h.id, text: h.text })),
    body: doc.html,
  });
}

function renderDoc(file: string, raw: string): Doc {
  const { meta, body } = frontMatter(raw);
  // The heading renderer fills this: the id it writes into the anchor is the
  // id the table of contents links to, derived once.
  const headings: Heading[] = [];
  const md = new Marked({ async: false });
  md.use({
    renderer: {
      code({ text, lang }: Tokens.Code) {
        const name = (lang ?? "").trim().split(/\s+/)[0] ?? "";
        return codeBlock(highlight(text, name), text);
      },
      heading({ depth, tokens }: Tokens.Heading) {
        if (depth !== 2) return `<h${depth}>${this.parser.parseInline(tokens)}</h${depth}>\n`;
        // A heading that ends with inline HTML, a version chip for one, keeps
        // that part outside the anchor: the link is the name alone.
        const at = tokens.findIndex((t) => t.type === "html");
        const named = at === -1 ? tokens : tokens.slice(0, at);
        const trailing = at === -1 ? "" : this.parser.parseInline(tokens.slice(at));
        const label = headingLabel(tokens);
        const id = slugify(label);
        headings.push({ id, text: label });
        return `<h2 id="${escapeAttr(id)}"><a class="anchor" href="#${escapeAttr(id)}">`
          + `${this.parser.parseInline(named)}</a>${trailing}</h2>\n`;
      },
    },
  });
  const html = md.parse(fill(body, { base, repo: repoUrl, ...modeBadges(), ...versionChips() }), { async: false });
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
function sections(html: string, headings: Heading[]): Doc["sections"] {
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

function plainText(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">")
    .replace(/&#39;|&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim();
}

// A heading can end with inline HTML, a state badge for one. The anchor and
// the table of contents take what stands before it.
function headingLabel(tokens: Token[] = []): string {
  const words: string[] = [];
  for (const token of tokens) {
    if (token.type === "html") break;
    words.push("text" in token && typeof token.text === "string" ? token.text : token.raw);
  }
  return words.join("").trim();
}

function frontMatter(raw: string): { meta: Record<string, string>; body: string } {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) return { meta: {}, body: raw };
  const meta: Record<string, string> = {};
  for (const line of (match[1] ?? "").split("\n")) {
    const at = line.indexOf(":");
    if (at > 0) meta[line.slice(0, at).trim()] = line.slice(at + 1).trim();
  }
  return { meta, body: raw.slice(match[0].length) };
}
