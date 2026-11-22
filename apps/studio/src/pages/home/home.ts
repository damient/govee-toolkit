import { t } from "../../core/i18n.ts";
import { ARROW, icon } from "../../shell/nav.ts";
import { notice } from "../../components/notice/notice.ts";
import { pathOf } from "../../core/router.ts";

const WAND =
  "M6 21L21 6l-3-3L3 18zM15 6l3 3M9 3a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2M19 13a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2";

/** Draw the Home view: the welcome card opens the setup. */
export function home(view: HTMLElement): void {
  view.classList.add("centered");
  const start = document.createElement("a");
  start.className = "button primary";
  start.href = pathOf("setup");
  start.append(t("home.welcome.start"), icon(ARROW));
  view.append(
    notice({ icon: WAND, title: t("home.welcome.title"), body: t("home.welcome.body"), tone: "ember", level: "h1", actions: [start] }),
  );
}
