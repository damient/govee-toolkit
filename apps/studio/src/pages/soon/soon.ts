import { type Key, t } from "../../core/i18n.ts";
import { notice } from "../../components/notice/notice.ts";

export interface Soon {
  title: Key;
  /** What the view will do, in one or two sentences. */
  body: Key;
  /** The `d` attribute of the icon of the page. */
  icon: string;
}

export function soon(view: HTMLElement, page: Soon): void {
  view.classList.add("centered");
  view.append(
    notice({ icon: page.icon, title: t(page.title), body: t(page.body), badge: t("soon.badge"), tone: "dashed", level: "h1" }),
  );
}
