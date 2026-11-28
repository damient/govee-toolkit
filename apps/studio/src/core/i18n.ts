import en from "../assets/locales/en.json" with { type: "json" };
import { Failure } from "./http.ts";
import { prefs } from "./prefs.ts";

/** English is the reference: a key exists when `locales/en.json` has it. */
export type Key = keyof typeof en;

/** Each code needs a file `locales/<code>.json`. */
export const LANGUAGES = ["en", "fr", "de", "es", "it"] as const;
export type Language = (typeof LANGUAGES)[number];

/** The name of each language, in that language. */
export const NAMES: Record<Language, string> = {
  en: "English",
  fr: "Français",
  de: "Deutsch",
  es: "Español",
  it: "Italiano",
};

// The bundled English text stays until a file loads, so that an error can show.
let strings: Record<string, string> = en;

function isStrings(value: unknown): value is Record<string, string> {
  if (typeof value !== "object" || value === null) return false;
  return Object.values(value).every((item) => typeof item === "string");
}

/** The text of `key`, with each `{name}` replaced by `vars[name]`. A missing key falls back to English. */
export function t(key: Key, vars: Record<string, string> = {}): string {
  const text = strings[key] ?? en[key];
  return text.replaceAll(/\{(\w+)\}/gu, (match, name: string) => vars[name] ?? match);
}

function isLanguage(code: string): code is Language {
  return LANGUAGES.some((item) => item === code);
}

/** The first tag of `navigator.languages` whose primary subtag has a file: `fr-CA` gives `fr`. */
function system(): Language | undefined {
  for (const tag of navigator.languages) {
    const code = tag.toLowerCase().split("-")[0] ?? "";
    if (isLanguage(code)) return code;
  }
  return undefined;
}

/** The language that the user picked, else the language of the system or browser, else English. */
export function language(): Language {
  const picked = prefs().language;
  if (picked !== null && isLanguage(picked)) return picked;
  return system() ?? "en";
}

/** The file is fetched each time, so that a change shows without a rebuild. */
export async function load(code: Language): Promise<void> {
  const response = await fetch(`/locales/${code}.json`, { cache: "no-store" });
  if (!response.ok) throw new Error(`locales/${code}.json: HTTP ${response.status}`);
  const data: unknown = await response.json();
  if (!isStrings(data)) throw new Error(`locales/${code}.json: not a map of strings`);
  strings = data;
  document.documentElement.lang = code;
}

function isKey(value: string | undefined): value is Key {
  return value !== undefined && value in en;
}

/** Fill `data-i18n` (the text) and `data-i18n-label` (`aria-label`) under `root`. */
export function translate(root: ParentNode = document): void {
  for (const element of root.querySelectorAll<HTMLElement>("[data-i18n]")) {
    const key = element.dataset["i18n"];
    if (isKey(key)) element.textContent = t(key);
  }
  for (const element of root.querySelectorAll<HTMLElement>("[data-i18n-label]")) {
    const key = element.dataset["i18nLabel"];
    if (isKey(key)) element.setAttribute("aria-label", t(key));
  }
}

/** A sentence for the user about `error`: a request that failed, or another fault. */
export function explain(error: unknown): string {
  if (error instanceof Failure) {
    return error.kind === "refused" ? t("error.refused", { reason: error.message }) : t(`error.${error.kind}`);
  }
  return t("error.other", { reason: error instanceof Error ? error.message : String(error) });
}
