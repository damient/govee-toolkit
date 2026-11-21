// The test files run one at a time: two SDKs on port 4002 can take the scan reply of each other.

import assert from "node:assert/strict";
import { type ChildProcess, spawn } from "node:child_process";
import { once } from "node:events";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

import type { Client } from "@modelcontextprotocol/client";
import { z } from "zod";

import { connect, structured } from "./helpers.ts";
import { closeSdk } from "../src/sdk.ts";

const binary =
  process.env.GOVEE_SIM ??
  fileURLToPath(new URL("../../../packages/rust/target/debug/govee-toolkit-sim", import.meta.url));
const present = existsSync(binary) || existsSync(`${binary}.exe`);
if (process.env.GOVEE_SIM !== undefined && !present) throw new Error(`GOVEE_SIM: no binary at ${binary}`);

export const skip = present ? false : "no simulator: run `cargo build -p govee-toolkit-sim`, or set GOVEE_SIM";

// A locally administered address, so no real device carries it.
export const SIM_ID = "02:00:00:4D:43:50";

let sim: ChildProcess | undefined;

export async function startSim(): Promise<Client> {
  sim = spawn(binary, ["--id", SIM_ID], { stdio: ["ignore", "pipe", "inherit"] });
  // The first line of stdout says that the sockets are bound.
  await once(sim.stdout ?? sim, "data");
  return connect();
}

export async function stopSim(client: Client): Promise<void> {
  await client.close();
  await closeSdk();
  sim?.kill("SIGINT");
}

const deviceShape = z.looseObject({
  id: z.string(),
  sku: z.string(),
  modes: z.array(z.looseObject({ mode: z.string() })),
});
export const devicesShape = z.object({ devices: z.array(deviceShape), problems: z.array(z.string()).optional() });
type Found = z.infer<typeof deviceShape>;

let scanned: Promise<Found> | undefined;

/** Scans once for the file: a scan takes the whole scan window. */
export function scanForSim(client: Client): Promise<Found> {
  scanned ??= scan(client);
  return scanned;
}

async function scan(client: Client): Promise<Found> {
  const { devices, problems } = await structured(client, "scan", { modes: ["lan"] }, devicesShape);
  const device = devices.find((d) => d.id === SIM_ID);
  assert.ok(device, `the scan did not find ${SIM_ID}: ${JSON.stringify({ devices, problems })}`);
  return device;
}
