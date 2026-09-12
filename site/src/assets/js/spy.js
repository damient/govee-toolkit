// Marks the entry the reader is on, and keeps that link inside the menu's own
// scroll.
export function spy(nav) {
  if (!nav.querySelector('a[href^="#"]')) return;
  const links = new Map();
  for (const link of nav.querySelectorAll('a[href^="#"]')) {
    links.set(link.getAttribute("href").slice(1), link);
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

    for (const link of links.values()) link.removeAttribute("aria-current");
    const link = links.get(current);
    if (!link) return;
    link.setAttribute("aria-current", "true");

    const group = link.closest(".sub-group");
    if (group) group.querySelector("a").setAttribute("aria-current", "true");

    // Measured, not read off offsetTop: the menu is the offset parent of its
    // own links once it is sticky.
    const top = link.getBoundingClientRect().top
      - nav.getBoundingClientRect().top + nav.scrollTop;
    if (top < nav.scrollTop + 40) nav.scrollTop = Math.max(0, top - 40);
    if (top > nav.scrollTop + nav.clientHeight - 60) {
      nav.scrollTop = top - nav.clientHeight + 60;
    }
  };

  let queued = false;
  const schedule = () => {
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
