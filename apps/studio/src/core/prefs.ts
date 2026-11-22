// The Studio settings. The server keeps them (`/api/settings`), so that
// the desktop app and each browser show the same choices. `localStorage`
// keeps a copy of the theme for the script in index.html, which paints the
// page before the server answers, and the layout choices of this screen.

import { Failure, request, send } from "./http.ts";

const PREFIX = "studio.";

export interface Prefs {
  theme: string;
  /** `null` until the user picks a language. */
  language: string | null;
  advanced: boolean;
  /** Colors as `#rrggbb`, which the panel of every light offers. 32 at most. */
  favorites: string[];
}

/** The most favorite colors that the server keeps. */
export const FAVORITES = 32;

/** The versions that the server gives with the settings. A client cannot change them. */
export interface Server {
  /** The version of govee-server. */
  version: string;
  /** The version of the govee-toolkit crate in the server. */
  sdk: string;
}

function local(name: string): string | null {
  try {
    return localStorage.getItem(PREFIX + name);
  } catch {
    return null;
  }
}

function keep(name: string, value: string): void {
  try {
    localStorage.setItem(PREFIX + name, value);
  } catch {
    // Private mode or blocked storage: the next first paint uses the default.
  }
}

let current: Prefs = { theme: local("theme") ?? "dark", language: null, advanced: false, favorites: [] };

function isPrefs(value: unknown): value is Prefs {
  if (typeof value !== "object" || value === null) return false;
  return (
    "theme" in value &&
    typeof value.theme === "string" &&
    "language" in value &&
    (value.language === null || typeof value.language === "string") &&
    "advanced" in value &&
    typeof value.advanced === "boolean" &&
    "favorites" in value &&
    Array.isArray(value.favorites) &&
    value.favorites.every((color: unknown) => typeof color === "string")
  );
}

let facts: Server | undefined;

function isServer(value: unknown): value is Server {
  if (typeof value !== "object" || value === null) return false;
  return "version" in value && typeof value.version === "string" && "sdk" in value && typeof value.sdk === "string";
}

function answer(body: unknown): void {
  if (!isPrefs(body)) throw new Failure("unreadable");
  current = { theme: body.theme, language: body.language, advanced: body.advanced, favorites: body.favorites };
  if ("server" in body && isServer(body.server)) facts = body.server;
  keep("theme", current.theme);
}

/** The versions of the server, once the settings have loaded. */
export function server(): Server | undefined {
  return facts;
}

/** The preferences as the server last gave them, or the cached theme before `load`. */
export function prefs(): Readonly<Prefs> {
  return current;
}

/** `GET /api/settings`. */
export async function load(): Promise<void> {
  answer(await request("/api/settings"));
}

/** `PUT /api/settings` with the keys to change. `prefs()` gives the change at once. */
export async function save(change: Partial<Prefs>): Promise<void> {
  current = { ...current, ...change };
  keep("theme", current.theme);
  answer(await send("/api/settings", "PUT", JSON.stringify(change)));
}

/** A layout choice of this screen, which stays in this browser: `fallback` when it is missing or unknown. */
export function readLocal<T extends string>(name: string, allowed: readonly T[], fallback: T): T {
  const value = local(name);
  return allowed.find((option) => option === value) ?? fallback;
}

/** Keep a layout choice of this screen in this browser. */
export function writeLocal(name: string, value: string): void {
  keep(name, value);
}
