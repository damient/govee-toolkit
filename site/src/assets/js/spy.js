// Marks the entry the reader is on, and keeps that link inside the menu's own
// scroll. A folded group opens when the reader scrolls into it — see
// `tocList()` in `lib/docs.mjs`.
//
// A click on the menu holds that marking until the page stops moving. The
// scroll it starts crosses every section between here and there, and the menu
// would otherwise open each group it passes.
const SETTLED_MS = 140;

export function spy(nav) {
  // One query, so the entries stay in menu order, which is page order: a
  // group title sits before the entries of its group. `mark()` reads that
  // order to find the last section above the line.
  const marks = nav.querySelectorAll('a[href^="#"], summary[data-to]');
  if (!marks.length) return;
  const links = new Map();
  for (const item of marks) {
    links.set(item.dataset.to ?? item.getAttribute("href").slice(1), item);
  }
  const targets = [...links.keys()]
    .map((id) => document.getElementById(id))
    .filter(Boolean);
  if (!targets.length) return;

  let current = null;
  const mark = () => {
    const line = 140;
    let found = targets[0];
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

    // Measured, not read off offsetTop: the menu is the offset parent of its
    // own links once it is sticky.
    const top = link.getBoundingClientRect().top
      - nav.getBoundingClientRect().top + nav.scrollTop;
    if (top < nav.scrollTop + 40) nav.scrollTop = Math.max(0, top - 40);
    if (top > nav.scrollTop + nav.clientHeight - 60) {
      nav.scrollTop = top - nav.clientHeight + 60;
    }
  };

  // The menu drives the page: the marking waits for the scroll to settle, and
  // a reader who scrolls themselves takes it back at once.
  let held = null;
  const release = () => {
    if (held === null) return;
    clearTimeout(held);
    held = null;
    mark();
  };
  const hold = () => {
    clearTimeout(held);
    held = setTimeout(release, SETTLED_MS);
  };
  for (const event of ["wheel", "touchstart", "keydown"]) {
    addEventListener(event, release, { passive: true });
  }

  // A group's title goes to its section, and opens the group on the way. A
  // title that closes its group moves the page nowhere.
  //
  // The page moves on `toggle`, where the group carries its new state, and
  // `asked` is what tells a reader's click from the opening `mark()` does.
  for (const summary of nav.querySelectorAll("summary[data-to]")) {
    const group = summary.parentElement;
    let asked = false;
    summary.addEventListener("click", () => {
      asked = true;
    });
    group.addEventListener("toggle", () => {
      if (!asked) return;
      asked = false;
      if (!group.open) return;
      hold();
      document.getElementById(summary.dataset.to)?.scrollIntoView();
    });
  }
  for (const link of nav.querySelectorAll('a[href^="#"]')) {
    link.addEventListener("click", hold);
  }

  let queued = false;
  const schedule = () => {
    if (held !== null) {
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
