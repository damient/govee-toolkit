import type { CallToolResult } from "@modelcontextprotocol/server";
import { Govee } from "govee-toolkit";
import type { Device, DeviceHandle, DeviceStatus, Outcome } from "govee-toolkit";
import { z } from "zod";

import { vocabulary } from "./catalog.ts";
import { fail, oneOf } from "./result.ts";

let started: Promise<Govee> | undefined;

/** Frees UDP port 4002: the kernel gives a reply to one process on the port only. */
const idleMs = idleSetting(process.env.GOVEE_MCP_IDLE_MS);
let active = 0;
let idleTimer: NodeJS.Timeout | undefined;
let releasing: Promise<void> = Promise.resolve();

function idleSetting(value: string | undefined): number {
  if (value === undefined || value === "") return 30_000;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0) throw new Error(`GOVEE_MCP_IDLE_MS: not a whole number of milliseconds: ${value}`);
  return parsed;
}

/** The SDK, started on the first call. A start that fails is tried again on the next call. */
export function sdk(): Promise<Govee> {
  started ??= Govee.start().catch((error: unknown) => {
    started = undefined;
    throw error;
  });
  return started;
}

/** Releases every transport. The next call acquires each one again and keeps the known devices. */
async function release(): Promise<void> {
  const govee = await started?.catch(() => null);
  await govee?.close();
}

/** Releases every transport and drops the SDK. Without it, `ble` loses the last frame it wrote. */
export async function closeSdk(): Promise<void> {
  clearTimeout(idleTimer);
  await releasing;
  const running = started;
  started = undefined;
  const govee = await running?.catch(() => null);
  await govee?.close();
}

/** Starts the SDK, runs the call, and turns a thrown binding error into a tool failure. */
export async function attempt(
  answer: (govee: Govee) => CallToolResult | Promise<CallToolResult>,
): Promise<CallToolResult> {
  clearTimeout(idleTimer);
  active += 1;
  try {
    // A call that arrives during an idle release waits for it, then binds again.
    await releasing;
    return await answer(await sdk());
  } catch (error) {
    return fail(error);
  } finally {
    active -= 1;
    if (active === 0 && idleMs > 0) {
      idleTimer = setTimeout(() => {
        releasing = release().catch((error: unknown) => {
          console.error("idle release:", error);
        });
      }, idleMs);
      idleTimer.unref();
    }
  }
}

/** The device handle, pinned when `mode` is given, and the mode that serves it. */
export async function reach(govee: Govee, target: string, mode?: string): Promise<[DeviceHandle, string]> {
  const handle = govee.device(target, mode === undefined ? {} : { mode });
  return [handle, await handle.ensureKnown()];
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

export function deviceRow(device: Device): Record<string, unknown> {
  return device.toJSON();
}

export function outcomeRow(outcome: Outcome): Record<string, unknown> {
  return outcome.toJSON();
}

export function statusRow(status: DeviceStatus, mode: string): Record<string, unknown> {
  return { ...status.toJSON(), mode };
}
