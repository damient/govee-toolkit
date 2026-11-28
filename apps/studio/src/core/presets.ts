// `/api/presets`: the server keeps the presets, so every client shows the same list.

import { request, send } from "./http.ts";

/** A look saved on one model: `color`, `kelvin`, or `cells` with one color per zone or LED. */
export interface Preset {
  /** Empty until the server gives one. */
  id: string;
  name: string;
  /** The model that the preset was saved on. */
  sku: string;
  color?: string;
  kelvin?: number;
  cells?: string[];
  /** In percent. */
  brightness?: number;
}

let current: Preset[] = [];
const changes = new EventTarget();

function strings(list: unknown): boolean {
  return Array.isArray(list) && list.every((item) => typeof item === "string");
}

function isPreset(value: unknown): value is Preset {
  if (typeof value !== "object" || value === null) return false;
  return (
    "id" in value &&
    typeof value.id === "string" &&
    "name" in value &&
    typeof value.name === "string" &&
    "sku" in value &&
    typeof value.sku === "string" &&
    (!("color" in value) || typeof value.color === "string") &&
    (!("kelvin" in value) || typeof value.kelvin === "number") &&
    (!("brightness" in value) || typeof value.brightness === "number") &&
    (!("cells" in value) || strings(value.cells))
  );
}

function answer(body: unknown): void {
  const list = typeof body === "object" && body !== null && "presets" in body ? body.presets : null;
  current = Array.isArray(list) ? list.filter((item) => isPreset(item)) : [];
  changes.dispatchEvent(new Event("presets"));
}

/** The presets as the server last gave them. */
export function presets(): readonly Preset[] {
  return current;
}

/** Runs `listener` after each change of the list. The caller removes it with the function that this returns. */
export function onPresets(listener: () => void): () => void {
  changes.addEventListener("presets", listener);
  return () => {
    changes.removeEventListener("presets", listener);
  };
}

/** `GET /api/presets`. */
export async function loadPresets(): Promise<void> {
  answer(await request("/api/presets"));
}

function path(id: string): string {
  return `/api/presets/${encodeURIComponent(id)}`;
}

/** `PUT /api/presets/{id}`: `preset` replaces the preset `id`, in its place. */
export async function updatePreset(id: string, preset: Omit<Preset, "id">): Promise<void> {
  answer(await send(path(id), "PUT", JSON.stringify(preset)));
}

/** `DELETE /api/presets/{id}`. */
export async function deletePreset(id: string): Promise<void> {
  answer(await request(path(id), { method: "DELETE" }));
}

/** `POST /api/presets`: the server adds `preset` to its own list, so a list that another client changed stays whole. */
export async function addPreset(preset: Omit<Preset, "id">): Promise<void> {
  answer(await send("/api/presets", "POST", JSON.stringify(preset)));
}
