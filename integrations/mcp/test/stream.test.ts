// Every call targets the simulator, as in control.test.ts.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import type { Client } from "@modelcontextprotocol/client";
import { z } from "zod";

import { failureCode, structured } from "./helpers.ts";
import { SIM_ID, scanForSim, skip, startSim, stopSim } from "./sim.ts";

let client: Client;

before(async () => {
  if (skip === false) client = await startSim();
});

after(async () => {
  if (skip === false) await stopSim(client);
});

const streamShape = z.object({
  played_ms: z.number(),
  streams: z.array(
    z.object({
      id: z.string(),
      zones: z.number(),
      rate_hz: z.number(),
      frames_sent: z.number(),
      frames_superseded: z.number(),
      error: z.string().nullable(),
    }),
  ),
});

test(
  "stream plays the frames on the segment channel, then disarms",
  { skip, timeout: 30_000 },
  async () => {
    await scanForSim(client);
    const answer = await structured(
      client,
      "stream",
      {
        targets: [SIM_ID],
        mode: "lan",
        frames: [[[255, 0, 0]], [[0, 0, 255]]],
        hold_ms: 100,
        repeat: 2,
      },
      streamShape,
    );
    const [stream] = answer.streams;
    assert.equal(stream?.id, SIM_ID);
    assert.equal(stream.error, null);
    assert.ok(stream.frames_sent > 0, "no frame reached the wire");
    assert.ok(answer.played_ms >= 400);
  },
);

test(
  "a stream longer than one call fails before anything is armed",
  { skip, timeout: 30_000 },
  async () => {
    await scanForSim(client);
    assert.equal(
      await failureCode(client, "stream", {
        targets: [SIM_ID],
        mode: "lan",
        frames: [[[255, 0, 0]]],
        hold_ms: 10_000,
        repeat: 13,
      }),
      "invalid_argument",
    );
  },
);
