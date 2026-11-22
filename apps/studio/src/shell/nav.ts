import "./shell.css";
import { type Key, t } from "../core/i18n.ts";
import { currentId, go, listen, pathOf } from "../core/router.ts";

/** A view at the path `/<id>`. */
export interface Route {
  id: string;
  label: Key;
  /**
   * The smallest screen that shows the view. The navigation of a smaller
   * screen leaves it out, and the view says that it needs a larger screen.
   */
  min?: "tablet" | "desktop";
  /** `narrow` centers the content at one width. A view is full width otherwise. */
  width?: "narrow";
  render(view: HTMLElement): void;
}

/** A space of the UI: one entry in the navigation, one view. */
export interface Page extends Route {
  /** The `d` attribute of a 24 × 24 stroked path. */
  icon: string;
}

/** The phone tab bar holds this many pages; the rest go to "More". */
const TABS = 4;

const MORE_ICON =
  "M4 12a1 1 0 1 0 2 0 1 1 0 1 0-2 0M11 12a1 1 0 1 0 2 0 1 1 0 1 0-2 0M18 12a1 1 0 1 0 2 0 1 1 0 1 0-2 0";

const SVG = "http://www.w3.org/2000/svg";

/** The arrow of a button that goes to the next view. */
export const ARROW = "M5 12h14M13 18l6-6M13 6l6 6";

/** A 24 × 24 icon, stroked in the current color. */
export function icon(d: string): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.75");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", d);
  svg.append(path);
  return svg;
}

function label(text: string): HTMLSpanElement {
  const span = document.createElement("span");
  span.textContent = text;
  return span;
}

function link(page: Page): HTMLAnchorElement {
  const a = document.createElement("a");
  a.href = pathOf(page.id);
  a.dataset["page"] = page.id;
  if (page.min) a.dataset["min"] = page.min;
  // The name of the link when the rail hides the labels.
  a.title = t(page.label);
  a.append(icon(page.icon), label(t(page.label)));
  return a;
}

/** The "More" entry of the phone tab bar, and the menu that it opens. */
function more(pages: Page[]): { button: HTMLButtonElement; menu: HTMLDivElement } {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "more";
  button.setAttribute("aria-expanded", "false");
  button.append(icon(MORE_ICON), label(t("nav.more")));
  const menu = document.createElement("div");
  menu.className = "more-menu";
  menu.hidden = true;
  menu.append(...pages.map((page) => link(page)));
  const toggle = (open: boolean): void => {
    menu.hidden = !open;
    button.setAttribute("aria-expanded", String(open));
  };
  button.addEventListener("click", () => {
    toggle(Boolean(menu.hidden));
  });
  menu.addEventListener("click", () => {
    toggle(false);
  });
  return { button, menu };
}

/** The note that replaces a view on a screen smaller than `min`. CSS shows it. */
function tooSmall(min: NonNullable<Route["min"]>): HTMLElement {
  const note = document.createElement("p");
  note.className = "too-small";
  note.textContent = t(min === "tablet" ? "small.tablet" : "small.desktop");
  return note;
}

/** Empty `view` for `route`, and name it. */
function clear(view: HTMLElement, route: Route): void {
  view.replaceChildren();
  view.className = route.width === "narrow" ? "view narrow" : "view";
  view.scrollTop = 0;
  view.setAttribute("aria-label", t(route.label));
  document.title = route.id === "home" ? "Govee Toolkit Studio" : `${t(route.label)} · Studio`;
  if (route.min) {
    view.dataset["min"] = route.min;
    view.append(tooSmall(route.min));
  } else {
    delete view.dataset["min"];
  }
}

/**
 * Draw the navigation and route on the path. `bottom` goes after a spacer, at
 * the foot of the side rail. On a phone, the pages after the first `TABS` go
 * to the "More" menu. `hidden` are views with no entry in the navigation.
 * Call the function that it returns to draw the navigation and the view
 * again, after a change of language.
 */
export function mount(nav: HTMLElement, view: HTMLElement, top: Page[], bottom: Page[], hidden: Route[] = []): () => void {
  const pages = [...top, ...bottom];
  const routes: Route[] = [...pages, ...hidden];

  const draw = (): void => {
    nav.setAttribute("aria-label", t("nav.main"));
    const spacer = document.createElement("div");
    spacer.className = "spacer";
    const links = pages.map((page) => link(page));
    for (const a of links.slice(TABS)) a.classList.add("overflow");
    nav.replaceChildren(...links.slice(0, top.length), spacer, ...links.slice(top.length));
    if (pages.length > TABS) {
      const { button, menu } = more(pages.slice(TABS).filter((page) => page.min === undefined));
      nav.append(button, menu);
    }
  };

  const show = (): void => {
    const id = currentId();
    const page = routes.find((p) => p.id === id);
    if (!page) {
      go(pathOf("home"), true);
      return;
    }
    for (const a of nav.querySelectorAll<HTMLAnchorElement>("a")) {
      if (a.dataset["page"] === page.id) a.setAttribute("aria-current", "page");
      else a.removeAttribute("aria-current");
    }
    const inMore = pages.findIndex((p) => p.id === page.id) >= TABS && page.min === undefined;
    nav.querySelector(".more")?.classList.toggle("current", inMore);
    clear(view, page);
    page.render(view);
  };

  listen(show);
  const refresh = (): void => {
    draw();
    show();
  };
  refresh();
  return refresh;
}
