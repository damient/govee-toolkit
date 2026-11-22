import { backup, fail } from "./data.ts";
import { type Key, LANGUAGES, type Language, NAMES, language, t } from "./i18n.ts";
import { THEMES, type Theme, setTheme, theme } from "./theme.ts";

const THEME_LABELS: Record<Theme, Key> = {
  dark: "settings.theme.dark",
  light: "settings.theme.light",
  auto: "settings.theme.auto",
};

function row(name: Key, control: HTMLElement): HTMLLIElement {
  const li = document.createElement("li");
  const title = document.createElement("span");
  title.className = "grow";
  title.textContent = t(name);
  li.append(title, control);
  return li;
}

function themes(note: HTMLElement): HTMLElement {
  const group = document.createElement("div");
  group.className = "seg";
  group.setAttribute("role", "radiogroup");
  group.setAttribute("aria-label", t("settings.theme"));
  const current = theme();
  for (const value of THEMES) {
    const label = document.createElement("label");
    const input = document.createElement("input");
    input.type = "radio";
    input.name = "theme";
    input.value = value;
    input.checked = value === current;
    input.addEventListener("change", () => {
      setTheme(value).catch((failure: unknown) => {
        fail(note, failure);
      });
    });
    label.append(input, t(THEME_LABELS[value]));
    group.append(label);
  }
  return group;
}

function languages(change: (code: Language) => void): HTMLElement {
  const select = document.createElement("select");
  select.setAttribute("aria-label", t("settings.language"));
  for (const code of LANGUAGES) select.add(new Option(NAMES[code], code, false, code === language()));
  select.addEventListener("change", () => {
    const code = LANGUAGES.find((item) => item === select.value);
    if (code) change(code);
  });
  return select;
}

/** Draw the Settings view. `change` applies a new language. */
export function settings(view: HTMLElement, change: (code: Language) => void): void {
  const list = document.createElement("ul");
  list.className = "list";
  const note = document.createElement("p");
  note.className = "settings-note";
  list.append(
    row("settings.theme", themes(note)),
    row("settings.language", languages(change)),
    row("settings.data", backup(note)),
  );
  view.append(list, note);
}
