import { Failure, request, send } from "./http.ts";
import { devicePath, listOf } from "./api.ts";

/** What the mode that answered can take, beside power and brightness. */
export interface Features {
  color: boolean;
  /** The bounds of `Change.kelvin`, or `null` when the mode has no white temperature. */
  colorTemp: [number, number] | null;
  /** The zone count of `Change.segment`, or `null` when the mode paints no zone. */
  segments: number | null;
  /** `Change.gradient` works. */
  gradient: boolean;
  /** The LEDs of `Change.draw`, or `null` when the mode has no drawing. */
  draw: Draw | null;
}

export interface Draw {
  /** The LED count. */
  cells: number;
  /** `[columns, rows]`, row 0 at the top, or `null` for a strip. */
  grid: [number, number] | null;
}

const isPair = (value: unknown): value is [number, number] =>
  Array.isArray(value) && value.length === 2 && value.every((item) => typeof item === "number");

function draw(value: unknown): Draw | null {
  if (typeof value !== "object" || value === null) return null;
  const cells = "cells" in value && typeof value.cells === "number" && value.cells > 0 ? value.cells : null;
  if (cells === null) return null;
  return { cells, grid: "grid" in value && isPair(value.grid) ? value.grid : null };
}

/** A device that `POST /api/apply` can only switch and dim. */
export const NONE: Features = { color: false, colorTemp: null, segments: null, gradient: false, draw: null };

function features(value: unknown): Features {
  if (typeof value !== "object" || value === null) return NONE;
  const segments = "segments" in value && typeof value.segments === "number" ? value.segments : null;
  return {
    color: "color" in value && value.color === true,
    colorTemp: "color_temp" in value && isPair(value.color_temp) ? value.color_temp : null,
    segments: segments !== null && segments > 0 ? segments : null,
    gradient: "gradient" in value && value.gradient === true,
    draw: "draw" in value ? draw(value.draw) : null,
  };
}

/** What a device answered on `GET /api/devices/{id}/status`. A field is `null` when the device does not report it. */
export interface Status {
  on: boolean | null;
  /** A percentage on every unit seen so far. */
  brightness: number | null;
  /** `#RRGGBB`. Black while the device is in white mode. */
  color: string | null;
  /** `0` while the device is in color mode. */
  kelvin: number | null;
  /** The bounds of `Change.brightness`, or `null` when the device has no brightness. */
  range: [number, number] | null;
  features: Features;
  /** The last drawing that the server sent to the light, `#RRGGBB` for each LED, or `null`. */
  drawing: string[] | null;
}

const field = <T>(body: object, key: string, check: (value: unknown) => value is T): T | null => {
  const value: unknown = Reflect.get(body, key);
  return check(value) ? value : null;
};
const isBoolean = (value: unknown): value is boolean => typeof value === "boolean";
const isNumber = (value: unknown): value is number => typeof value === "number";
const isString = (value: unknown): value is string => typeof value === "string";
const isColors = (value: unknown): value is string[] => Array.isArray(value) && value.every((item) => isString(item));

/** `GET /api/devices/{id}/status`. The first call for a device can take the scan window. `null` when the device does not answer. */
export async function status(id: string): Promise<Status | null> {
  const body = await request(devicePath(id, "status"));
  if (typeof body !== "object" || body === null) throw new Failure("unreadable");
  if (Reflect.get(body, "online") === false) return null;
  return {
    on: field(body, "on", isBoolean),
    brightness: field(body, "brightness", isNumber),
    color: field(body, "color", isString),
    kelvin: field(body, "color_temp_kelvin", isNumber),
    range: field(body, "brightness_range", isPair),
    features: features(Reflect.get(body, "features")),
    drawing: field(body, "drawing", isColors),
  };
}

export interface Outcome {
  id: string;
  ok: boolean;
  /** The message of the failure, when `ok` is `false`. */
  error: string | null;
}

function asOutcome(value: unknown): Outcome {
  if (typeof value !== "object" || value === null || !("id" in value) || typeof value.id !== "string") {
    throw new Failure("unreadable");
  }
  const ok = "ok" in value && value.ok === true;
  const failure = "error" in value && typeof value.error === "object" && value.error !== null ? value.error : null;
  const message =
    failure !== null && "message" in failure && typeof failure.message === "string" ? failure.message : null;
  return { id: value.id, ok, error: ok ? null : (message ?? "?") };
}

/** What `apply` changes. A key that is missing stays as it is. */
export interface Change {
  power?: boolean;
  /** In `Status.range`. A level out of it is refused. */
  brightness?: number;
  /** `#RRGGBB` over the whole light. */
  color?: string;
  /** A white temperature, in `Features.colorTemp`. */
  kelvin?: number;
  /** One color over the zero-based zones, which `Features.segments` counts. The other zones keep their color. */
  segment?: { zones: number[]; color: string };
  /** A `segment` carries it: a paint with no `gradient` sets it off. */
  gradient?: boolean;
  /** `#RRGGBB` for each LED that `Features.draw` counts. The device keeps the drawing after the request. */
  draw?: string[];
}

/** `POST /api/apply`: change the lights of `ids` at once. A light that fails does not stop the others. */
export async function apply(ids: string[], change: Change): Promise<Outcome[]> {
  const { kelvin, ...rest } = change;
  const body = JSON.stringify({ ids, ...rest, color_temp: kelvin });
  return listOf(await send("/api/apply", "POST", body)).map((item) => asOutcome(item));
}

/** `apply` for one light. Throws `refused` with the message of the light when it fails. */
export async function applyOne(id: string, change: Change): Promise<void> {
  const [answer] = await apply([id], change);
  if (answer?.ok !== true) throw new Failure("refused", answer?.error ?? "");
}
