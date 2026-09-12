// Progressive enhancement only. Every page reads and works with this file
// blocked: the rope and the strip are decoration, and the device table is in
// the HTML before the filter touches it.

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

// How many zones the decoration draws. This is a drawing count and not a
// measurement: a real zone count belongs to a device file.
const ZONES = 42;

const PALETTE = [
  [255, 61, 0],
  [255, 179, 92],
  [255, 94, 168],
  [141, 107, 255],
  [69, 227, 208],
];

function build(el, tag) {
  const zones = [];
  for (let i = 0; i < ZONES; i += 1) {
    zones.push(el.appendChild(document.createElement(tag)));
  }
  return zones;
}

function rgb([r, g, b], alpha = 1) {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function mix(a, b, t) {
  return a.map((value, i) => Math.round(value + (b[i] - value) * t));
}


function rope(el) {
  const zones = build(el, "i");

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

  // A timer in a background tab costs battery and paints nothing anybody
  // sees, so the rope repaints only while it is on screen and the tab is.
  let offset = 0;
  let timer = null;
  let visible = true;
  const start = () => {
    if (timer || !visible || document.hidden) return;
    timer = setInterval(() => {
      offset += 0.12;
      paint(offset);
    }, 900);
  };
  const stop = () => {
    clearInterval(timer);
    timer = null;
  };
  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    visible ? start() : stop();
  }).observe(el);
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));
  start();
}


function strip(el) {
  const zones = build(el, "b");
  // How far the light carries on each side of the pointer.
  const REACH = 6;

  const off = (zone) => {
    zone.style.removeProperty("--zone");
    zone.removeAttribute("data-lit");
  };

  // The window that is lit now, so a move writes the zones that change and
  // leaves the rest alone.
  let from = 0;
  let to = -1;
  const light = (index) => {
    const first = Math.max(0, index - REACH);
    const last = Math.min(ZONES - 1, index + REACH);
    for (let i = from; i <= to; i += 1) {
      if (i < first || i > last) off(zones[i]);
    }
    for (let i = first; i <= last; i += 1) {
      const distance = Math.abs(i - index);
      const fall = 1 - distance / (REACH + 1);
      const hue = mix(PALETTE[1], PALETTE[0], distance / (REACH + 1));
      zones[i].style.setProperty("--zone", rgb(hue, 0.25 + fall * 0.75));
      zones[i].dataset.lit = "";
    }
    from = first;
    to = last;
  };

  const clear = () => {
    for (let i = from; i <= to; i += 1) off(zones[i]);
    from = 0;
    to = -1;
  };

  // The box is read when the pointer arrives and when the layout can have
  // moved, never inside the move: reading it there forces a layout per event.
  let box = null;
  const measure = () => (box = el.getBoundingClientRect());
  // One frame in flight, holding the last position the pointer reached: a
  // burst of moves inside one frame paints once.
  let x = null;
  let frame = false;
  const paint = () => {
    frame = false;
    if (x === null) return;
    if (!box) measure();
    light(Math.max(0, Math.min(ZONES - 1, Math.floor(((x - box.left) / box.width) * ZONES))));
  };
  el.addEventListener("pointerenter", measure);
  el.addEventListener("pointermove", (event) => {
    x = event.clientX;
    if (frame) return;
    frame = true;
    requestAnimationFrame(paint);
  });
  el.addEventListener("pointerleave", () => {
    x = null;
    box = null;
    clear();
  });
  addEventListener("resize", () => (box = null), { passive: true });
  addEventListener("scroll", () => (box = null), { passive: true });

  light(Math.floor(ZONES / 2));
}


const COPY_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/></svg>';
const DONE_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 12.5 9.5 18 20 6.5"/></svg>';

// The button carries the icon and the label from here: a page without the
// script then shows nothing to press.
function copy(button) {
  const label = button.textContent.trim() || "Copy";
  const rest = () => {
    button.innerHTML = COPY_ICON;
    button.removeAttribute("data-state");
    say(label);
  };
  const say = (text) => {
    button.setAttribute("aria-label", text);
    button.title = text;
  };

  let queued = null;
  rest();
  button.addEventListener("click", async () => {
    clearTimeout(queued);
    try {
      await navigator.clipboard.writeText(button.dataset.copy);
      button.innerHTML = DONE_ICON;
      button.dataset.state = "done";
      say("Copied");
    } catch {
      button.dataset.state = "failed";
      say("Copy failed");
    }
    queued = setTimeout(rest, 1600);
  });
}


function filter(input) {
  const rows = [...document.querySelectorAll("[data-rows] tr")];
  const count = document.querySelector("[data-filter-count]");
  const total = rows.length;

  const apply = () => {
    const needle = input.value.trim().toLowerCase();
    let shown = 0;
    for (const row of rows) {
      const match = !needle || row.dataset.search.includes(needle);
      if (row.hidden === match) row.hidden = !match;
      if (match) shown += 1;
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


// The link in the first cell stays the target, so the keyboard and a middle
// click reach the page through it.
function rowLink(body) {
  body.addEventListener("click", (event) => {
    if (event.target.closest("a")) return;
    if (String(getSelection())) return;
    const row = event.target.closest("tr");
    const link = row?.querySelector("a[href]");
    if (link) link.click();
  });
}


// A link to a heading of the page you are on moves nothing else, so the close
// is the only answer the reader gets.
function docSelect(box) {
  box.addEventListener("click", (event) => {
    if (event.target.closest("a")) box.open = false;
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") box.open = false;
  });
}


// The head applies the theme before the first paint; this module only writes
// it. sessionStorage keeps the choice for the tab and no longer.
function theme(button) {
  const system = matchMedia("(prefers-color-scheme: dark)");
  const current = () => document.documentElement.dataset.theme
    || (system.matches ? "dark" : "light");

  button.addEventListener("click", () => {
    const next = current() === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    try {
      sessionStorage.setItem("theme", next);
    } catch {
      // A browser that refuses storage still switches the theme.
    }
  });
}


// One choice for the whole page: a reader who works in Python reads Python
// everywhere.
function tabs() {
  const elements = [...document.querySelectorAll("[data-langs]")];
  if (!elements.length) return;

  // Queried once: a language change writes these and queries nothing.
  const groups = elements.map((el) => ({
    el,
    buttons: [...el.querySelectorAll(".tabs button")],
    panes: [...el.querySelectorAll(".pane")],
    on: null,
  }));

  const show = (language) => {
    for (const group of groups) {
      const wanted = group.buttons.some((button) => button.dataset.lang === language);
      const chosen = wanted ? language : group.buttons[0].dataset.lang;
      if (chosen === group.on) continue;
      group.on = chosen;
      for (const button of group.buttons) {
        const on = button.dataset.lang === chosen;
        button.setAttribute("aria-selected", String(on));
        // One stop in the tab order per group: the arrow keys move inside it.
        button.tabIndex = on ? 0 : -1;
      }
      for (const pane of group.panes) {
        pane.hidden = pane.dataset.lang !== chosen;
      }
    }
  };

  const choose = (language) => {
    show(language);
    try {
      localStorage.setItem("language", language);
    } catch { /* private mode: the choice lasts for this page only. */ }
  };

  for (const group of groups) {
    group.el.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-lang]");
      if (button) choose(button.dataset.lang);
    });
    group.el.addEventListener("keydown", (event) => {
      const button = event.target.closest("button[data-lang]");
      if (!button) return;
      const { buttons } = group;
      const step = { ArrowLeft: -1, ArrowRight: 1 }[event.key];
      let next = null;
      if (step) next = buttons[(buttons.indexOf(button) + step + buttons.length) % buttons.length];
      if (event.key === "Home") next = buttons[0];
      if (event.key === "End") next = buttons[buttons.length - 1];
      if (!next) return;
      event.preventDefault();
      choose(next.dataset.lang);
      next.focus();
    });
  }

  let stored = null;
  try {
    stored = localStorage.getItem("language");
  } catch { /* nothing stored. */ }
  if (stored) show(stored);
}


// Marks the entry the reader is on, and keeps that link inside the menu's own
// scroll.
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


const WIRING = [
  ["[data-rope]", rope],
  ["[data-strip]", strip],
  ["[data-copy]", copy],
  ["[data-filter]", filter],
  ["[data-rows]", rowLink],
  ["[data-doc-select]", docSelect],
  ["[data-theme-toggle]", theme],
  ["[data-menu-toggle]", menu],
  ["[data-spy]", spy],
];

for (const [selector, wire] of WIRING) {
  document.querySelectorAll(selector).forEach(wire);
}
tabs();
