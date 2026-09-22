// The head applies the theme before the first paint; this module only writes
// it. sessionStorage keeps the choice for the tab and not beyond it.
export function theme(button: HTMLButtonElement): void {
  const system = matchMedia("(prefers-color-scheme: dark)");
  const current = () => {
    const chosen = document.documentElement.dataset.theme;
    if (chosen !== undefined && chosen !== "") return chosen;
    return system.matches ? "dark" : "light";
  };

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
