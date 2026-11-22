import "./alert.css";
import { icon } from "../../shell/nav.ts";

const ICONS = {
  success: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8 12.5l3 3 5-6",
  info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8h.01",
  warning: "M12 9v4M12 16h.01M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z",
  error: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM15 9l-6 6M9 9l6 6",
};

export interface Alert {
  tone: keyof typeof ICONS;
  title: string;
  body?: string;
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** A line in the flow of a view: the icon of its tone, a title and an optional text. An error is read out at once. */
export function alert(box: Alert): HTMLElement {
  const node = element("div", `alert ${box.tone}`);
  node.setAttribute("role", box.tone === "error" ? "alert" : "status");
  const mark = element("span", "alert-icon");
  mark.append(icon(ICONS[box.tone]));
  const text = element("div", "alert-text");
  text.append(element("p", "alert-title", box.title));
  if (box.body !== undefined) text.append(element("p", "alert-body", box.body));
  node.append(mark, text);
  return node;
}
