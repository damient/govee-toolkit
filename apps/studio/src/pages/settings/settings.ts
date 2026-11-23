import "./settings.css";
import { backup, fail } from "./data.ts";
import { type Key, LANGUAGES, type Language, NAMES, language, t } from "../../core/i18n.ts";
import { pathOf } from "../../core/router.ts";
import { share } from "../../core/api.ts";
import { server } from "../../core/prefs.ts";
import { resetButton } from "./reset.ts";
import { THEMES, type Theme, setTheme, theme } from "../../core/theme.ts";
import { VERSION } from "../../core/version.ts";

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

function card(...rows: HTMLLIElement[]): HTMLUListElement {
  const list = document.createElement("ul");
  list.className = "list";
  list.append(...rows);
  return list;
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

function setupLink(): HTMLAnchorElement {
  const link = document.createElement("a");
  link.className = "button xs";
  link.href = pathOf("setup");
  link.textContent = t("settings.setup.open");
  return link;
}

/** Draw the Settings view. `change` applies a new language. */
export function settings(view: HTMLElement, change: (code: Language) => void): void {
  const note = document.createElement("p");
  note.className = "settings-note";
  const lights = card(row("settings.setup", setupLink()));
  const look = card(row("settings.theme", themes(note)), row("settings.language", languages(change)));
  // Only this computer can export, import or reset.
  const local = share().then((state) => state.local);
  const data = card(row("settings.data", backup(note, local)), row("settings.reset", resetButton(note, local)));
  const line = document.createElement("p");
  line.className = "settings-versions";
  const facts = server();
  line.textContent = t("settings.versions", { studio: VERSION, server: facts?.version ?? "?", sdk: facts?.sdk ?? "?" });
  view.append(lights, look, data, note, line);
}
