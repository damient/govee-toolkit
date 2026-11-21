// The documentation pages: one Markdown file under `content/docs/`, one page.
// Another language holds its pages under `content/<id>/docs/`.

import { existsSync } from "node:fs";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { Marked, type Token, type Tokens } from "marked";
import { codeBlock, highlight } from "./code.ts";
import { base, repoUrl } from "./config.ts";
import { docShell } from "./docs.ts";
import { escapeAttr, fill, slugify } from "./html.ts";
import { type Locale, type T, contentDir, strings } from "./locales.ts";
import { modeBadges } from "./mode-badge.ts";
import { tabsExtension } from "./tabs.ts";
import type { NavEntry } from "./types.ts";
import { versionChips } from "./versions.ts";

interface Heading {
  id: string;
  text: string;
  /** The label a menu shows, from `<!-- menu: Label -->` at the end of the heading. */
  menu?: string;
}

/** One documentation page, read from its Markdown file. */
export interface Doc {
  slug: string;
  title: string;
  description: string;
  order: number;
  headings: Heading[];
  html: string;
}

/** One `##` section of a page: the heading, and the text under it as plain text. */
export interface Section {
  title: string;
  answer: string;
}

/** Every documentation page of a language, in menu order. */
export async function readDocs(locale: Locale): Promise<Doc[]> {
  const dir = join(contentDir(locale), "docs");
  if (!existsSync(dir)) return [];
  const files = (await readdir(dir)).filter((f) => f.endsWith(".md"));
  const t = strings(locale);
  const docs = await Promise.all(
    files.map(async (file) => renderDoc(file, await readFile(join(dir, file), "utf8"), t)),
  );
  return docs.toSorted((a, b) => a.order - b.order);
}

/**
 * One Markdown file of a language that sits outside the documentation menu.
 * `null` where the language does not carry it.
 */
export async function readPage(file: string, locale: Locale): Promise<Doc | null> {
  const path = join(contentDir(locale), file);
  if (!existsSync(path)) return null;
  return renderDoc(file, await readFile(path, "utf8"), strings(locale));
}

/** One documentation page, inside the frame every one of them shares. */
export function docPage(doc: Doc, nav: NavEntry[], t: T): string {
  return docShell({
    t,
    base,
    nav,
    current: `docs/${doc.slug}/`,
    toc: doc.headings.map((h) => ({ id: h.id, text: h.text })),
    body: doc.html,
  });
}

const MENU = /<!--\s*menu:\s*(.+?)\s*-->/u;

// `## Titre {#title}` keeps the anchor of the English page, so that a link to
// `#title` works in every language.
const ANCHOR = /\s*\{#([\w-]+)\}/u;

function renderDoc(file: string, raw: string, t: T): Doc {
  const { meta, body } = frontMatter(raw);
  // The heading renderer fills this, so that the anchor id and the table of
  // contents id are one value.
  const headings: Heading[] = [];
  const md = new Marked({ async: false });
  md.use(tabsExtension(), {
    renderer: {
      code({ text, lang }: Tokens.Code) {
        const name = (lang ?? "").trim().split(/\s+/u)[0] ?? "";
        return codeBlock(highlight(text, name), text, t("docs.copy"));
      },
      heading({ depth, tokens, text }: Tokens.Heading) {
        if (depth !== 2) return `<h${depth}>${this.parser.parseInline(tokens)}</h${depth}>\n`;
        // Inline HTML at the end of a heading stays outside the anchor.
        const at = tokens.findIndex((token) => token.type === "html");
        const named = at === -1 ? tokens : tokens.slice(0, at);
        const trailing = at === -1 ? "" : this.parser.parseInline(tokens.slice(at)).replace(MENU, "").replace(ANCHOR, "");
        const label = headingLabel(tokens).replace(ANCHOR, "");
        const id = ANCHOR.exec(text)?.[1] ?? slugify(label);
        const menu = at === -1 ? undefined : MENU.exec(tokens.slice(at).map((token) => token.raw).join(""))?.[1];
        headings.push({ id, text: label, ...(menu === undefined ? {} : { menu }) });
        return `<h2 id="${escapeAttr(id)}"><a class="anchor" href="#${escapeAttr(id)}">`
          + `${this.parser.parseInline(named).replace(ANCHOR, "")}</a>${trailing}</h2>\n`;
      },
    },
  });
  const html = md.parse(fill(body, { base, repo: repoUrl, ...modeBadges(), ...versionChips() }), { async: false });
  return {
    slug: meta.slug ?? file.replace(/\.md$/u, ""),
    title: meta.title ?? file,
    description: meta.description ?? "",
    order: Number(meta.order ?? 99),
    headings,
    html,
  };
}

// The heading is the question: the answer starts after the `</h2>`.
export function sections({ html, headings }: Doc): Section[] {
  const parts = html.split(/<h2 id="[^"]*">/u).slice(1);
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
    .replaceAll(/<[^>]+>/gu, " ")
    .replaceAll("&amp;", "&").replaceAll("&lt;", "<").replaceAll("&gt;", ">")
    .replaceAll("&#39;", "'").replaceAll("&quot;", '"')
    .replaceAll(/\s+/gu, " ")
    .replaceAll(/ ([,.;:!?)])/gu, "$1")
    .trim();
}

function headingLabel(tokens: Token[] = []): string {
  const words: string[] = [];
  for (const token of tokens) {
    if (token.type === "html") break;
    words.push("text" in token && typeof token.text === "string" ? token.text : token.raw);
  }
  return words.join("").trim();
}

function frontMatter(raw: string): { meta: Record<string, string>; body: string } {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n?/u);
  if (!match) return { meta: {}, body: raw };
  const meta: Record<string, string> = {};
  for (const line of (match[1] ?? "").split("\n")) {
    const at = line.indexOf(":");
    if (at > 0) meta[line.slice(0, at).trim()] = line.slice(at + 1).trim();
  }
  return { meta, body: raw.slice(match[0].length) };
}
