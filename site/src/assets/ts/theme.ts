// The head applies the theme before the first paint; this module only writes
// it. sessionStorage keeps the choice for the tab and not beyond it.
const current = (): string => document.documentElement.dataset.theme ?? "dark";

export function theme(button: HTMLButtonElement): void {
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
