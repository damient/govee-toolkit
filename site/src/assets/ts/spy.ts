// Marks the entry the reader is on, and keeps that link inside the menu's own
// scroll. A folded group opens when the reader scrolls into it — see
// `tocList()` in `lib/docs.ts`.
//
// A click on the menu holds that marking until the page stops moving. The
// scroll it starts crosses every section between here and there, and the menu
// would otherwise open each group it passes.
import { byId } from "./dom.ts";

const SETTLED_MS = 140;

// One query, so the entries stay in menu order, which is page order: a group
// title sits before the entries of its group. The marker reads that order to
// find the last section above the line.
function entries(nav: HTMLElement): { links: Map<string, HTMLElement>; targets: HTMLElement[] } {
  const links = new Map<string, HTMLElement>();
  for (const item of nav.querySelectorAll<HTMLElement>('a[href^="#"], summary[data-to]')) {
    links.set(item.dataset.to ?? (item.getAttribute("href") ?? "").slice(1), item);
  }
  const targets = [...links.keys()]
    .map((id) => byId(id))
    .filter((target) => target !== null);
  return { links, targets };
}

function highlight(link: HTMLElement): void {
  link.setAttribute("aria-current", "true");
  const folded = link.closest("details");
  if (folded) folded.open = true;
  const group = link.closest(".sub-group");
  const title = group?.querySelector(":scope > details > summary, :scope > a");
  if (title) title.setAttribute("aria-current", "true");
  // At the top of a group, the first entry is the section the reader is in:
  // the title alone marks the group and no entry of it.
  if (link === title) {
    group?.querySelector("ul a")?.setAttribute("aria-current", "true");
  }
}

// Measured, not read off offsetTop: the menu is the offset parent of its own
// links once it is sticky.
function reveal(nav: HTMLElement, link: HTMLElement): void {
  const top = link.getBoundingClientRect().top
    - nav.getBoundingClientRect().top + nav.scrollTop;
  if (top < nav.scrollTop + 40) nav.scrollTop = Math.max(0, top - 40);
  if (top > nav.scrollTop + nav.clientHeight - 60) {
    nav.scrollTop = top - nav.clientHeight + 60;
  }
}

function marker(nav: HTMLElement, links: Map<string, HTMLElement>, targets: HTMLElement[], first: HTMLElement): () => void {
  let current: string | null = null;
  return () => {
    const line = 140;
    let found = first;
    for (const target of targets) {
      if (target.getBoundingClientRect().top <= line) found = target;
    }
    if (found.id === current) return;
    current = found.id;
    for (const marked of nav.querySelectorAll("[aria-current]")) {
      marked.removeAttribute("aria-current");
    }
    const link = links.get(current);
    if (!link) return;
    highlight(link);
    reveal(nav, link);
  };
}

// The menu drives the page: the marking waits for the scroll to settle, and a
// reader who scrolls themselves takes it back at once.
function holder(mark: () => void): { hold: () => void; holding: () => boolean } {
  let held: ReturnType<typeof setTimeout> | undefined;
  const release = () => {
    if (held === undefined) return;
    clearTimeout(held);
    held = undefined;
    mark();
  };
  const hold = () => {
    clearTimeout(held);
    held = setTimeout(release, SETTLED_MS);
  };
  for (const event of ["wheel", "touchstart", "keydown"]) {
    addEventListener(event, release, { passive: true });
  }
  return { hold, holding: () => held !== undefined };
}

// A group's title goes to its section, and opens the group on the way. A
// title that closes its group moves the page nowhere.
//
// The page moves on `toggle`, where the group carries its new state, and
// `asked` is what tells a reader's click from the opening the marker does.
function follow(nav: HTMLElement, hold: () => void): void {
  for (const summary of nav.querySelectorAll<HTMLElement>("summary[data-to]")) {
    const group = summary.parentElement;
    if (!(group instanceof HTMLDetailsElement)) continue;
    let asked = false;
    summary.addEventListener("click", () => {
      asked = true;
    });
    group.addEventListener("toggle", () => {
      if (!asked) return;
      asked = false;
      if (!group.open) return;
      hold();
      byId(summary.dataset.to ?? "")?.scrollIntoView();
    });
  }
  for (const link of nav.querySelectorAll('a[href^="#"]')) {
    link.addEventListener("click", hold);
  }
}

export function spy(nav: HTMLElement): void {
  const { links, targets } = entries(nav);
  const first = targets[0];
  if (!first) return;
  const mark = marker(nav, links, targets, first);
  const { hold, holding } = holder(mark);
  follow(nav, hold);

  let queued = false;
  const schedule = () => {
    if (holding()) {
      hold();
      return;
    }
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      mark();
    });
  };
  addEventListener("scroll", schedule, { passive: true });
  addEventListener("resize", schedule, { passive: true });
  mark();
}
