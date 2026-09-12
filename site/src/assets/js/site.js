// Progressive enhancement only. Every page reads and works with this file
// blocked: the rope and the strip are decoration, and the device table is in
// the HTML before the filter touches it.

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

// The zone count of a verified 3 m rope. The number is a device fact; it lives
// in devices/H61A0.yaml and is repeated here only to size a decoration.
const ZONES = 42;

const PALETTE = [
  [255, 61, 0],
  [255, 179, 92],
  [255, 94, 168],
  [141, 107, 255],
  [69, 227, 208],
];

function rgb([r, g, b], alpha = 1) {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function mix(a, b, t) {
  return a.map((value, i) => Math.round(value + (b[i] - value) * t));
}

// --- The rope under the top bar -------------------------------------------

function rope(el) {
  const zones = [];
  for (let i = 0; i < ZONES; i += 1) {
    const zone = document.createElement("i");
    el.append(zone);
    zones.push(zone);
  }

  const paint = (offset) => {
    zones.forEach((zone, i) => {
      const position = (i / ZONES) * (PALETTE.length - 1) + offset;
      const low = Math.floor(position) % PALETTE.length;
      const high = (low + 1) % PALETTE.length;
      const color = mix(PALETTE[low], PALETTE[high], position % 1);
      zone.style.setProperty("--zone", rgb(color, 0.85));
    });
  };

  paint(0);
  if (reduced) return;

  let offset = 0;
  setInterval(() => {
    offset += 0.12;
    paint(offset);
  }, 900);
}

// --- The strip the pointer paints ------------------------------------------

function strip(el) {
  const zones = [];
  for (let i = 0; i < ZONES; i += 1) {
    const zone = document.createElement("b");
    el.append(zone);
    zones.push(zone);
  }

  const color = PALETTE[0];
  const light = (index) => {
    zones.forEach((zone, i) => {
      const distance = Math.abs(i - index);
      if (distance > 6) {
        zone.style.removeProperty("--zone");
        zone.removeAttribute("data-lit");
        return;
      }
      const fall = 1 - distance / 7;
      const hue = mix(PALETTE[1], color, distance / 7);
      zone.style.setProperty("--zone", rgb(hue, 0.25 + fall * 0.75));
      zone.dataset.lit = "";
    });
  };

  const indexAt = (x) => {
    const box = el.getBoundingClientRect();
    return Math.max(0, Math.min(ZONES - 1, Math.floor(((x - box.left) / box.width) * ZONES)));
  };

  el.addEventListener("pointermove", (event) => light(indexAt(event.clientX)));
  el.addEventListener("pointerleave", () => {
    zones.forEach((zone) => {
      zone.style.removeProperty("--zone");
      zone.removeAttribute("data-lit");
    });
  });

  light(Math.floor(ZONES / 2));
}

// --- Copy a command --------------------------------------------------------

function copy(button) {
  button.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(button.dataset.copy);
      const previous = button.textContent;
      button.textContent = "Copied";
      setTimeout(() => { button.textContent = previous; }, 1600);
    } catch {
      button.textContent = "Copy failed";
    }
  });
}

// --- Filter the device table ----------------------------------------------

function filter(input) {
  const rows = [...document.querySelectorAll("[data-rows] tr")];
  const cards = [...document.querySelectorAll("[data-cards] .device")];
  const count = document.querySelector("[data-filter-count]");
  const total = rows.length;

  const apply = () => {
    const needle = input.value.trim().toLowerCase();
    let shown = 0;
    for (const row of rows) {
      const match = !needle || row.dataset.search.includes(needle);
      row.hidden = !match;
      if (match) shown += 1;
    }
    for (const card of cards) {
      card.hidden = Boolean(needle) && !card.dataset.search.includes(needle);
    }
    if (count) {
      count.textContent = needle
        ? `${shown} of ${total} models match.`
        : `${total} models in the catalog.`;
    }
  };

  input.addEventListener("input", apply);
  apply();
}

// --- Theme -----------------------------------------------------------------

// The system setting decides, and the button switches away from it for as
// long as the reader stays on the page. Nothing is stored: the next page
// starts from the system setting again.
function theme(button) {
  const system = matchMedia("(prefers-color-scheme: dark)");
  const current = () => document.documentElement.dataset.theme
    || (system.matches ? "dark" : "light");

  button.addEventListener("click", () => {
    document.documentElement.dataset.theme = current() === "dark" ? "light" : "dark";
  });
}

// --- Language tabs ---------------------------------------------------------

// One choice for the whole page: a reader who works in Python reads Python
// everywhere, and does not click through twenty blocks.
function tabs() {
  const groups = [...document.querySelectorAll("[data-langs]")];
  if (!groups.length) return;

  const show = (language) => {
    for (const group of groups) {
      const buttons = [...group.querySelectorAll(".tabs button")];
      const wanted = buttons.some((button) => button.dataset.lang === language);
      for (const button of buttons) {
        const on = button.dataset.lang === (wanted ? language : buttons[0].dataset.lang);
        button.setAttribute("aria-selected", String(on));
      }
      for (const pane of group.querySelectorAll(".pane")) {
        pane.hidden = pane.dataset.lang !== (wanted ? language : group.querySelector(".pane").dataset.lang);
      }
    }
  };

  for (const group of groups) {
    group.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-lang]");
      if (!button) return;
      show(button.dataset.lang);
      try {
        localStorage.setItem("language", button.dataset.lang);
      } catch { /* private mode: the choice lasts for this page only. */ }
    });
  }

  let stored = null;
  try {
    stored = localStorage.getItem("language");
  } catch { /* nothing stored. */ }
  if (stored) show(stored);
}

// --- Follow the reader in the menu -----------------------------------------

// Marks the entry the reader is on, and keeps that link inside the menu's own
// scroll. The menu scrolls on its own, so a long list never forces the reader
// to the bottom of the page to reach the last item.
function spy(nav) {
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

// --- The folded menu -------------------------------------------------------

function menu(button) {
  const bar = button.closest(".topbar");
  const links = document.getElementById(button.getAttribute("aria-controls"));
  const wide = matchMedia("(min-width: 821px)");

  const set = (open) => {
    bar.dataset.menu = open ? "open" : "closed";
    button.setAttribute("aria-expanded", String(open));
    button.setAttribute("aria-label", open ? "Close the menu" : "Open the menu");
  };

  set(false);
  button.addEventListener("click", () => set(bar.dataset.menu !== "open"));
  links.addEventListener("click", (event) => {
    if (event.target.closest("a")) set(false);
  });
  addEventListener("keydown", (event) => {
    if (event.key === "Escape" && bar.dataset.menu === "open") {
      set(false);
      button.focus();
    }
  });
  wide.addEventListener("change", (event) => {
    if (event.matches) set(false);
  });
}

// --- Wire it up ------------------------------------------------------------

document.querySelectorAll("[data-rope]").forEach(rope);
document.querySelectorAll("[data-strip]").forEach(strip);
document.querySelectorAll("[data-copy]").forEach(copy);
document.querySelectorAll("[data-filter]").forEach(filter);
document.querySelectorAll("[data-theme-toggle]").forEach(theme);
document.querySelectorAll("[data-menu-toggle]").forEach(menu);
document.querySelectorAll("[data-spy]").forEach(spy);
tabs();
