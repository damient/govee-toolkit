// The pages of Studio are paths: `/`, `/settings`, `/setup`. govee-server
// answers `index.html` for a path with no dot, so a reload keeps the page.

/** The path of the page `id`: `/` for home. */
export function pathOf(id: string): string {
  return id === "home" ? "/" : `/${id}`;
}

/** The id of the page at the current path: `home` for `/`. */
export function currentId(): string {
  return location.pathname.replaceAll(/^\/+|\/+$/gu, "") || "home";
}

let changed = (): void => {};

/** Show the page at `path`. `replace` changes the current entry of the history. */
export function go(path: string, replace = false): void {
  if (replace) history.replaceState(null, "", path);
  else if (path !== location.pathname) history.pushState(null, "", path);
  changed();
}

/** The link of `event` when Studio shows its page: same origin, no API, no new tab, no download. */
function internal(event: MouseEvent): URL | null {
  if (event.defaultPrevented || event.button !== 0) return null;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return null;
  if (!(event.target instanceof Element)) return null;
  const link = event.target.closest("a");
  if (link === null || !link.hasAttribute("href") || link.target !== "" || link.hasAttribute("download")) return null;
  const url = new URL(link.href);
  if (url.origin !== location.origin || url.pathname === "/api" || url.pathname.startsWith("/api/")) return null;
  return url;
}

/** Route the links of the page, and the back and forward buttons. `change` draws the page at the new path. */
export function listen(change: () => void): void {
  changed = change;
  addEventListener("popstate", change);
  document.addEventListener("click", (event) => {
    const url = internal(event);
    if (!url) return;
    event.preventDefault();
    go(url.pathname);
  });
}
