// A tab list inside a Markdown page. `src/assets/ts/choices.ts` reads
// `data-tabs`, and one choice holds for every list of the same group.
//
//   ::: tabs client
//   @tab Claude Code
//   Markdown of the first pane.
//   @tab Codex
//   Markdown of the second pane.
//   :::

import type { MarkedExtension, Token, Tokens } from "marked";
import { escapeAttr, escapeHtml, slugify } from "./html.ts";

interface Pane {
  label: string;
  tokens: Token[];
}

interface TabsToken extends Tokens.Generic {
  type: "tabs";
  group: string;
  panes: Pane[];
}

const BLOCK = /^::: tabs (\S+)\n([\s\S]*?)\n:::(?:\n|$)/u;
const TAB = /^@tab (.+)$/mu;

function isTabs(token: Tokens.Generic): token is TabsToken {
  return token.type === "tabs" && typeof token["group"] === "string" && Array.isArray(token["panes"]);
}

/** The extension for one page. The count keeps the identifiers unique inside it. */
export function tabsExtension(): MarkedExtension {
  let count = 0;
  return {
    extensions: [{
      name: "tabs",
      level: "block",
      start: (src: string) => src.match(/^::: tabs /mu)?.index,
      tokenizer(src: string): TabsToken | undefined {
        const match = BLOCK.exec(src);
        if (!match) return undefined;
        const [raw, group = "", body = ""] = match;
        // `split` with a capture group alternates text and labels; the text
        // before the first `@tab` is dropped.
        const parts = body.split(new RegExp(TAB.source, "mu")).slice(1);
        const panes: Pane[] = [];
        for (let i = 0; i < parts.length; i += 2) {
          panes.push({ label: (parts[i] ?? "").trim(), tokens: this.lexer.blockTokens(parts[i + 1] ?? "", []) });
        }
        return { type: "tabs", raw, group, panes };
      },
      renderer(token) {
        if (!isTabs(token)) return false;
        const { group, panes } = token;
        count += 1;
        const id = (pane: Pane) => `${group}-${count}-${slugify(pane.label)}`;
        const buttons = panes
          .map((pane, i) => `<button type="button" role="tab" id="tab-${id(pane)}" aria-controls="pane-${id(pane)}"`
            + ` data-tab="${escapeAttr(slugify(pane.label))}" aria-selected="${i === 0}" tabindex="${i === 0 ? "0" : "-1"}">`
            + `${escapeHtml(pane.label)}</button>`)
          .join("");
        const bodies = panes
          .map((pane, i) => `<div class="tab-pane" role="tabpanel" id="pane-${id(pane)}" aria-labelledby="tab-${id(pane)}"`
            + ` data-tab="${escapeAttr(slugify(pane.label))}"${i === 0 ? "" : " hidden"}>\n${this.parser.parse(pane.tokens)}</div>`)
          .join("\n");
        return `<div class="choice" data-tabs="${escapeAttr(group)}">\n`
          + `<div class="tabs" role="tablist" aria-label="${escapeAttr(group.charAt(0).toUpperCase() + group.slice(1))}">${buttons}</div>\n${bodies}\n</div>\n`;
      },
    }],
  };
}
