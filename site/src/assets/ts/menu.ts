// A link to a heading of the page you are on moves nothing else, so the close
// is the only answer the reader gets.
export function docSelect(box: HTMLDetailsElement): void {
  box.addEventListener("click", (event) => {
    if (event.target instanceof Element && event.target.closest("a")) box.open = false;
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") box.open = false;
  });
}
export function menu(button: HTMLButtonElement): void {
  const bar = button.closest<HTMLElement>(".topbar");
  const links = document.getElementById(button.getAttribute("aria-controls") ?? "");
  if (!bar || !links) return;
  const wide = matchMedia("(min-width: 821px)");

  const set = (open: boolean) => {
    bar.dataset.menu = open ? "open" : "closed";
    button.setAttribute("aria-expanded", String(open));
    button.setAttribute("aria-label", open ? "Close the menu" : "Open the menu");
  };

  set(false);
  button.addEventListener("click", () => set(bar.dataset.menu !== "open"));
  links.addEventListener("click", (event) => {
    if (event.target instanceof Element && event.target.closest("a")) set(false);
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
