import { type Key, t } from "./i18n.ts";

/** A space of the UI: one entry in the navigation, one view. */
export interface Page {
  id: string;
  label: Key;
  /** The `d` attribute of a 24 × 24 stroked path. */
  icon: string;
  render(view: HTMLElement): void;
}

/** The phone tab bar holds this many pages; the rest go to "More". */
const TABS = 4;

const MORE_ICON =
  "M4 12a1 1 0 1 0 2 0 1 1 0 1 0-2 0M11 12a1 1 0 1 0 2 0 1 1 0 1 0-2 0M18 12a1 1 0 1 0 2 0 1 1 0 1 0-2 0";

const SVG = "http://www.w3.org/2000/svg";

function icon(d: string): SVGSVGElement {
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
  a.href = `#${page.id}`;
  a.dataset["page"] = page.id;
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

/**
 * Draw the navigation and route on the hash. `bottom` goes after a spacer, at
 * the foot of the side rail. On a phone, the pages after the first `TABS` go
 * to the "More" menu. Call the function that it returns to draw the
 * navigation and the view again, after a change of language.
 */
export function mount(nav: HTMLElement, view: HTMLElement, top: Page[], bottom: Page[]): () => void {
  const pages = [...top, ...bottom];

  const draw = (): void => {
    nav.setAttribute("aria-label", t("nav.main"));
    const spacer = document.createElement("div");
    spacer.className = "spacer";
    const links = pages.map((page) => link(page));
    for (const a of links.slice(TABS)) a.classList.add("overflow");
    nav.replaceChildren(...links.slice(0, top.length), spacer, ...links.slice(top.length));
    if (pages.length > TABS) {
      const { button, menu } = more(pages.slice(TABS));
      nav.append(button, menu);
    }
  };

  const show = (): void => {
    const id = location.hash.slice(1);
    const page = pages.find((p) => p.id === id) ?? pages[0];
    if (!page) return;
    for (const a of nav.querySelectorAll<HTMLAnchorElement>("a")) {
      if (a.dataset["page"] === page.id) a.setAttribute("aria-current", "page");
      else a.removeAttribute("aria-current");
    }
    const inMore = pages.indexOf(page) >= TABS;
    nav.querySelector(".more")?.classList.toggle("current", inMore);
    view.replaceChildren();
    view.setAttribute("aria-label", t(page.label));
    page.render(view);
  };

  addEventListener("hashchange", show);
  const refresh = (): void => {
    draw();
    show();
  };
  refresh();
  return refresh;
}
