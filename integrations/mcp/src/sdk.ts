// One `Govee` for the life of the process, started on the first control call,
// and what every control tool shares. It reads the configuration that the CLI
// reads: `GOVEE_CONFIG` names the file, and the cloud key comes from the `env`
// of the client's MCP configuration.

import type { CallToolResult } from "@modelcontextprotocol/server";
import { Govee } from "govee-toolkit";
import type { Device, DeviceStatus, Health, Outcome, Served } from "govee-toolkit";
import { z } from "zod";

import { vocabulary } from "./catalog.ts";
import { fail, failure, oneOf } from "./result.ts";

let started: Promise<Govee> | undefined;

/** The SDK, started on the first call. A start that fails is tried again on the next call. */
export function sdk(): Promise<Govee> {
  started ??= Govee.start().catch((error: unknown) => {
    started = undefined;
    throw error;
  });
  return started;
}

/** Releases every transport. Without it, `ble` loses the last frame it wrote. */
export async function closeSdk(): Promise<void> {
  const running = started;
  started = undefined;
  const govee = await running?.catch(() => null);
  await govee?.close();
}

/** Starts the SDK, runs the call, and turns a thrown binding error into a tool failure. */
export async function attempt(answer: (govee: Govee) => Promise<CallToolResult>): Promise<CallToolResult> {
  try {
    return await answer(await sdk());
  } catch (error) {
    return fail(error);
  }
}

export const CONTROL = { readOnlyHint: false, destructiveHint: false, openWorldHint: true } as const;

export const TARGET =
  "An identity (`AA:BB:CC:DD:EE:FF`), a name that the configuration gives a device (`name:kitchen`), " +
  "or a group that it gives (`group:ambient`).";

export const modeArg = oneOf(vocabulary.modes)
  .optional()
  .describe(
    "Pins the call to one mode, which the device must enable. Without it, the preference order of the device applies. " +
      "A mode that does not answer fails the call: nothing moves to another mode.",
  );

const byte = z.number().int().min(0).max(255);

export const rgb = z.tuple([byte, byte, byte]).describe("Red, green and blue, 0-255 each.");

function health(value: Health): Record<string, unknown> {
  return { state: value.state, failures: value.failures, available: value.available };
}

export function deviceRow(device: Device): Record<string, unknown> {
  return {
    id: device.id,
    sku: device.sku,
    name: device.name,
    groups: device.groups,
    modes: device.modes,
    health: Object.fromEntries(Object.entries(device.health).map(([mode, h]) => [mode, health(h)])),
  };
}

export function servedRow(served: Served): Record<string, unknown> {
  return { id: served.id, mode: served.mode, command: served.command };
}

export function outcomeRow(outcome: Outcome): Record<string, unknown> {
  return {
    id: outcome.id,
    ok: outcome.ok,
    mode: outcome.mode,
    command: outcome.served?.command ?? null,
    error: outcome.error === null ? null : failure(outcome.error),
  };
}

export function statusRow(status: DeviceStatus, mode: string): Record<string, unknown> {
  return {
    id: status.id,
    mode,
    on: status.on,
    brightness: status.brightness,
    color: status.color,
    color_temp_kelvin: status.colorTempKelvin,
    white: status.isWhite,
    raw: status.raw as unknown,
  };
}
