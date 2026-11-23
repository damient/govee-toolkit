import { element } from "../../core/dom.ts";
import { tooltip } from "../tooltip/tooltip.ts";

/**
 * The mark of a light that has no state: a request runs, or the light did
 * not answer. It takes the focus, so that a keyboard shows its tooltip.
 */
export function statusMark(text: string, content: Node): HTMLElement {
  const mark = element("span", "light-find light-offline");
  mark.tabIndex = 0;
  mark.setAttribute("role", "img");
  mark.setAttribute("aria-label", text);
  mark.append(content);
  tooltip(mark, text);
  return mark;
}
