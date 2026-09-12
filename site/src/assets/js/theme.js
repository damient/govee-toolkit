// The head applies the theme before the first paint; this module only writes
// it. sessionStorage keeps the choice for the tab and no longer.
export function theme(button) {
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
