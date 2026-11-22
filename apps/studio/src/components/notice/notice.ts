import "./notice.css";
import { icon } from "../../shell/nav.ts";

export interface Notice {
  /** The `d` attribute of the icon. */
  icon: string;
  title: string;
  body?: string;
  /** A short label over the title, such as "Coming soon". */
  badge?: string;
  /** `ember` marks the next action, `dashed` a view that has no content yet. */
  tone?: "plain" | "ember" | "dashed";
  /** Buttons or links under the text. */
  actions?: Node[];
  /** `h1` when the card is the whole view. */
  level?: "h1" | "h2";
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** A centered card: an icon, a title, an optional text and actions. */
export function notice(card: Notice): HTMLElement {
  const box = element("section", `notice ${card.tone ?? "plain"}`);
  const mark = element("span", "notice-icon");
  mark.append(icon(card.icon));
  box.append(mark);
  if (card.badge !== undefined) box.append(element("span", "notice-badge", card.badge));
  box.append(element(card.level ?? "h2", "notice-title", card.title));
  if (card.body !== undefined) box.append(element("p", "notice-body", card.body));
  if (card.actions !== undefined && card.actions.length > 0) {
    const actions = element("div", "notice-actions");
    actions.append(...card.actions);
    box.append(actions);
  }
  return box;
}
