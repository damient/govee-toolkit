// A real device answers the scan too: every call targets the simulator.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import type { Client } from "@modelcontextprotocol/client";
import { MODES } from "govee-toolkit";
import { z } from "zod";

import { failureCode, structured } from "./helpers.ts";
import { SIM_ID, devicesShape, scanForSim, skip, startSim, stopSim } from "./sim.ts";

let client: Client;

before(async () => {
  if (skip === false) client = await startSim();
});

after(async () => {
  if (skip === false) await stopSim(client);
});

const commandsShape = z.looseObject({
  commands: z.record(
    z.string(),
    z.record(
      z.string(),
      z.looseObject({
        args: z
          .record(
            z.string(),
            z.looseObject({ role: z.string().nullable(), bound: z.unknown() }),
          )
          .optional(),
      }),
    ),
  ),
});

/** The lowest value that the device file accepts for the argument of this role. */
async function lowestFor(sku: string, role: string): Promise<number> {
  const record = await structured(
    client,
    "describe_device",
    { sku },
    commandsShape,
  );
  const args = Object.values(record.commands.lan ?? {}).flatMap((command) =>
    Object.values(command.args ?? {}),
  );
  const [lowest] = z
    .tuple([z.number(), z.number()])
    .parse(args.find((arg) => arg.role === role)?.bound);
  return lowest;
}

const setShape = z.object({
  members: z.array(z.string()),
  steps: z.array(
    z.object({
      step: z.string(),
      outcomes: z.array(
        z.looseObject({
          id: z.string(),
          ok: z.boolean(),
          mode: z.string().nullable(),
        }),
      ),
    }),
  ),
});
const statusShape = z.looseObject({ id: z.string(), mode: z.string() });

test(
  "scan finds the simulator over lan",
  { skip, timeout: 30_000 },
  async () => {
    const device = await scanForSim(client);
    assert.ok(device.modes.some(({ mode }) => mode === "lan"));
    const { devices } = await structured(
      client,
      "list_known",
      {},
      devicesShape,
    );
    assert.ok(devices.some((d) => d.id === SIM_ID));
  },
);

test(
  "set with power and brightness names the mode that served each step",
  { skip, timeout: 30_000 },
  async () => {
    const device = await scanForSim(client);
    const brightness = await lowestFor(device.sku, "brightness");
    const answer = await structured(
      client,
      "set",
      { target: SIM_ID, mode: "lan", power: true, brightness },
      setShape,
    );
    assert.deepEqual(answer.members, [SIM_ID]);
    assert.deepEqual(
      answer.steps.map((s) => s.step),
      ["reach", "power", "brightness"],
    );
    for (const { step, outcomes } of answer.steps.slice(1)) {
      assert.deepEqual(
        outcomes.map((o) => [o.id, o.ok, o.mode]),
        [[SIM_ID, true, "lan"]],
        step,
      );
    }
  },
);

test(
  "set sends power off after the other steps",
  { skip, timeout: 30_000 },
  async () => {
    const device = await scanForSim(client);
    const brightness = await lowestFor(device.sku, "brightness");
    const answer = await structured(
      client,
      "set",
      { target: SIM_ID, mode: "lan", power: false, brightness },
      setShape,
    );
    assert.deepEqual(
      answer.steps.map((s) => s.step),
      ["reach", "brightness", "power"],
    );
  },
);

const roleShape = z.looseObject({
  commands: z.record(
    z.string(),
    z.record(
      z.string(),
      z.looseObject({ role: z.string().nullable().optional() }),
    ),
  ),
});
const sentShape = z.looseObject({
  id: z.string(),
  mode: z.string(),
  command: z.string(),
});

// No lan entry declares a `reply:` layout.
test(
  "send sends a command whose entry declares no answer",
  { skip, timeout: 30_000 },
  async () => {
    const device = await scanForSim(client);
    const record = await structured(
      client,
      "describe_device",
      { sku: device.sku },
      roleShape,
    );
    const command = Object.entries(record.commands.lan ?? {}).find(
      ([, entry]) => entry.role === "status",
    )?.[0];
    assert.ok(
      command !== undefined,
      "the device file declares no status command over lan",
    );
    const answer = await structured(
      client,
      "send",
      { target: SIM_ID, mode: "lan", command },
      sentShape,
    );
    assert.deepEqual(answer, { id: SIM_ID, mode: "lan", command });
  },
);

test(
  "status reads the simulator over lan",
  { skip, timeout: 30_000 },
  async () => {
    await scanForSim(client);
    const answer = await structured(
      client,
      "status",
      { target: SIM_ID, mode: "lan" },
      statusShape,
    );
    assert.equal(answer.id, SIM_ID);
    assert.equal(answer.mode, "lan");
  },
);

test(
  "a value out of range fails, and nothing is clamped",
  { skip, timeout: 30_000 },
  async () => {
    const device = await scanForSim(client);
    const brightness = (await lowestFor(device.sku, "brightness")) - 1;
    assert.equal(
      await failureCode(client, "set", {
        target: SIM_ID,
        mode: "lan",
        brightness,
      }),
      "out_of_range",
    );
  },
);

test(
  "a mode that the configuration does not enable fails, and says so",
  { skip, timeout: 30_000 },
  async () => {
    await scanForSim(client);
    const other = MODES.find((mode) => mode !== "lan");
    assert.equal(
      await failureCode(client, "set", {
        target: SIM_ID,
        mode: other,
        power: true,
      }),
      "mode_not_enabled",
    );
  },
);
