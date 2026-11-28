import type { Preset } from "../../../core/presets.ts";
import type { Change, Features } from "../../../core/control.ts";
import type { Memory } from "./zones.ts";
import { element } from "../../../core/dom.ts";
import { explain, t } from "../../../core/i18n.ts";

/** What a preset saves beside its name. */
export type Look = Pick<Preset, "color" | "kelvin" | "cells">;

/** The light that the panel shows, as the presets see it. */
export interface PresetLight {
  sku: string;
  features: Features;
  /** The bounds of the brightness, in percent. */
  range: [number, number] | null;
  /** The zones or LEDs, or `null` when the light has none. */
  kept: Memory | null;
  /** `true`: `kept` holds LEDs, else segments. */
  leds: boolean;
  /** The columns of the LED grid, or `null` for a strip. */
  columns: number | null;
}

/** The light can apply `preset`. A color or a white shows for every model that can play it, cells only for their model. */
export function fits(preset: Preset, light: PresetLight): boolean {
  if (preset.cells !== undefined) return preset.sku === light.sku;
  if (preset.color !== undefined) return light.features.color;
  const bounds = light.features.colorTemp;
  const kelvin = preset.kelvin ?? 0;
  return bounds !== null && kelvin >= bounds[0] && kelvin <= bounds[1];
}

/**
 * The change that applies `preset`: a color, a white, a drawing, or one color
 * for each zone. It carries the brightness, in the range of the light. `null`
 * when the light has another number of zones or LEDs than the preset.
 */
function presetChange(preset: Preset, light: PresetLight): Change | null {
  const { range, kept } = light;
  const level = preset.brightness;
  const dim = level === undefined || range === null ? {} : { brightness: Math.min(range[1], Math.max(range[0], level)) };
  if (preset.color !== undefined) return { color: preset.color, ...dim };
  if (preset.kelvin !== undefined) return { kelvin: preset.kelvin, ...dim };
  const cells = preset.cells ?? [];
  if (kept === null || cells.length !== kept.colors.length) return null;
  if (light.leds) return { draw: cells, ...dim };
  return { segment: { colors: cells }, gradient: kept.gradient, ...dim };
}

const WARM = [255, 180, 107];
const NEUTRAL = [255, 244, 229];
const COLD = [205, 220, 255];

/** Send the change of `preset`. `failed` gets a light that cannot apply it. */
export function playPreset(
  preset: Preset,
  light: PresetLight,
  run: (change: Change) => Promise<void>,
  failed: (message: string) => void,
): void {
  const change = presetChange(preset, light);
  if (change === null) {
    failed(t("panel.preset.size"));
    return;
  }
  run(change).catch((failure: unknown) => {
    failed(explain(failure));
  });
}

function mix(from: number[], to: number[], ratio: number): number[] {
  const r = Math.min(1, Math.max(0, ratio));
  return from.map((value, index) => Math.round(value + ((to[index] ?? value) - value) * r));
}

/** An approximate color of a white: warm at 2000 K, neutral at 6500 K, cold at 9000 K. */
function whiteColor(kelvin: number): string {
  const rgb = kelvin <= 6500 ? mix(WARM, NEUTRAL, (kelvin - 2000) / 4500) : mix(NEUTRAL, COLD, (kelvin - 6500) / 2500);
  return `rgb(${rgb.join(" ")})`;
}

/** A solid color, a white, or the cells in rows of `columns`. */
export function snapshot(preset: Preset, columns: number | null): HTMLElement {
  const box = element("span", "panel-snap");
  if (preset.cells === undefined) {
    box.style.background = preset.color ?? whiteColor(preset.kelvin ?? 6500);
    return box;
  }
  box.classList.add("cells");
  // A strip folds into a square.
  box.style.setProperty("--columns", String(columns ?? Math.ceil(Math.sqrt(preset.cells.length))));
  box.append(
    ...preset.cells.map((hex) => {
      const cell = element("span", "");
      cell.style.background = hex;
      return cell;
    }),
  );
  return box;
}
