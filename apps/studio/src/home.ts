import { t } from "./i18n.ts";
import { icon } from "./nav.ts";
import { notice } from "./notice.ts";
import { pathOf } from "./router.ts";

const WAND =
  "M6 21L21 6l-3-3L3 18zM15 6l3 3M9 3a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2M19 13a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2";
export const ARROW = "M5 12h14M13 18l6-6M13 6l6 6";

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
