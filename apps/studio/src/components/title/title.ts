import "./title.css";
import { t } from "../../core/i18n.ts";
import { BACK, icon } from "../icon/icon.ts";


export interface Title {
  text: string;
  /** The path of the back link, on the left of the title. */
  back?: string;
  /** The content on the right: steps, buttons. */
  end?: Node[];
}

export function title(head: Title): HTMLElement {
  const row = document.createElement("header");
  row.className = "page-head";
  if (head.back !== undefined) {
    const back = document.createElement("a");
    back.className = "icon-button page-back";
    back.href = head.back;
    back.setAttribute("aria-label", t("title.back"));
    back.title = t("title.back");
    back.append(icon(BACK));
    row.append(back);
  }
  const h1 = document.createElement("h1");
  h1.className = "page-title";
  h1.textContent = head.text;
  row.append(h1);
  if (head.end !== undefined && head.end.length > 0) {
    const end = document.createElement("div");
    end.className = "page-end";
    end.append(...head.end);
    row.append(end);
  }
  return row;
}
