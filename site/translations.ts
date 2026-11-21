// Checks that every language carries each English text, translated from the
// English text as it is now. A translated file names the English file it comes
// from by the git blob id of that file: `source:` in the front matter of a
// Markdown page, `$source` in a JSON file.
//
//   node translations.ts                 reports, and exits 1 on a gap
//   node translations.ts --stamp <file>  records the current English blob in <file>
//
// Stamp a file after its translation follows the English text again.

import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { root } from "./lib/config.ts";
import { isRecord } from "./lib/json.ts";
import { frontMatter } from "./lib/content.ts";
import { ENGLISH, type Locale, TRANSLATIONS, contentDir, readTable, tablePath } from "./lib/locales.ts";

/** The id git gives the content of a file, so that `git diff <old> <new>` shows the change. */
function blob(path: string): string {
  const body = readFileSync(path);
  return createHash("sha1").update(`blob ${body.length}\0`).update(body).digest("hex");
}

const FRONT = /^---\n([\s\S]*?)\n---/u;
const SOURCE_LINE = /^source:\s*(\S*)\s*$/mu;

/** The English file that one translated file follows. */
function english(locale: Locale, path: string): string {
  const rel = relative(contentDir(locale), path);
  return rel.startsWith("..") ? tablePath(ENGLISH.id) : join(contentDir(ENGLISH), rel);
}

function recorded(path: string): string | undefined {
  const text = readFileSync(path, "utf8");
  if (path.endsWith(".json")) {
    const value: unknown = JSON.parse(text);
    return isRecord(value) && typeof value["$source"] === "string" ? value["$source"] : undefined;
  }
  return frontMatter(text).meta["source"];
}

function stamp(path: string): void {
  const locale = TRANSLATIONS.find((l) => path.startsWith(`${contentDir(l)}/`) || path === tablePath(l.id));
  if (!locale) throw new Error(`${path}: no language in the path`);
  const from = blob(english(locale, path));
  const text = readFileSync(path, "utf8");
  if (path.endsWith(".json")) {
    const value: unknown = JSON.parse(text);
    if (!isRecord(value)) throw new Error(`${path}: not an object`);
    const rest = Object.entries(value).filter(([key]) => key !== "$source");
    writeFileSync(path, `${JSON.stringify(Object.fromEntries([["$source", from], ...rest]), null, 2)}\n`);
  } else {
    const front = FRONT.exec(text);
    if (!front) throw new Error(`${path}: no front matter`);
    const body = front[1] ?? "";
    const next = SOURCE_LINE.test(body) ? body.replace(SOURCE_LINE, `source: ${from}`) : `${body}\nsource: ${from}`;
    writeFileSync(path, text.replace(body, next));
  }
  console.log(`${relative(root, path)}: source ${from.slice(0, 12)}`);
}

const markdown = (dir: string): string[] =>
  existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith(".md")).map((f) => join(dir, f)) : [];

/** The gaps of one language, one line each. */
function gaps(locale: Locale): string[] {
  const out: string[] = [];
  const dir = contentDir(locale);
  const pairs: [string, string][] = [
    tablePath(locale.id),
    join(dir, "faq.md"),
    join(dir, "reference.json"),
    ...markdown(join(contentDir(ENGLISH), "docs")).map((f) => join(dir, "docs", relative(join(contentDir(ENGLISH), "docs"), f))),
  ].map((own) => [own, english(locale, own)]);
  for (const own of markdown(join(dir, "docs"))) {
    if (!existsSync(english(locale, own))) out.push(`${relative(root, own)}: no English page of that name`);
  }
  for (const [own, en] of pairs) {
    const name = relative(root, own);
    if (!existsSync(own)) {
      out.push(`${name}: missing`);
      continue;
    }
    const was = recorded(own);
    const now = blob(en);
    if (was === undefined) out.push(`${name}: no source, stamp it`);
    else if (was !== now) out.push(`${name}: ${relative(root, en)} changed since — git diff ${was.slice(0, 12)} ${now.slice(0, 12)}`);
  }
  out.push(...referenceGaps(locale));
  const en = readTable(ENGLISH.id);
  const own = readTable(locale.id);
  for (const key of Object.keys(en)) if (!(key in own)) out.push(`content/i18n/${locale.id}.json: no \`${key}\``);
  for (const key of Object.keys(own)) {
    if (!(key in en) && key !== "$source" && key !== "categories") out.push(`content/i18n/${locale.id}.json: \`${key}\` is not in en.json`);
  }
  return out;
}

// Every entry of the reference takes a summary, and a model page block where
// the English entry draws one.
function referenceGaps(locale: Locale): string[] {
  const path = join(contentDir(locale), "reference.json");
  if (!existsSync(path)) return [];
  const own: unknown = JSON.parse(readFileSync(path, "utf8"));
  const source: unknown = JSON.parse(readFileSync(join(contentDir(ENGLISH), "reference.json"), "utf8"));
  if (!isRecord(own) || !isRecord(source) || !Array.isArray(source["groups"])) return [`${relative(root, path)}: unexpected shape`];
  const entries = isRecord(own["entries"]) ? own["entries"] : {};
  const out: string[] = [];
  for (const group of source["groups"]) {
    if (!isRecord(group) || !Array.isArray(group["entries"])) continue;
    for (const entry of group["entries"]) {
      if (!isRecord(entry) || typeof entry["id"] !== "string") continue;
      const mine = entries[entry["id"]];
      if (!isRecord(mine)) out.push(`${relative(root, path)}: no entry \`${entry["id"]}\``);
      else if (isRecord(entry["action"]) && !isRecord(mine["action"])) out.push(`${relative(root, path)}: no action for \`${entry["id"]}\``);
    }
  }
  return out;
}

const at = process.argv.indexOf("--stamp");
if (at === -1) {
  const found = TRANSLATIONS.flatMap((l) => gaps(l));
  for (const line of found) console.error(line);
  if (found.length > 0) {
    console.error(`\n${found.length} translation gaps. Translate the change, then: node translations.ts --stamp <file>`);
    process.exit(1);
  }
  console.log(`translations: ${TRANSLATIONS.length} languages follow the English text`);
} else {
  for (const path of process.argv.slice(at + 1)) stamp(join(process.cwd(), path));
}
