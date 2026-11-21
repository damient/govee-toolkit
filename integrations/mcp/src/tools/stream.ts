import type { McpServer } from "@modelcontextprotocol/server";
import type { Govee, SegmentStream } from "govee-toolkit";
import { z } from "zod";

import { codeError, ok } from "../result.ts";
import { CONTROL, attempt, modeArg, reach, resolutionArg, rgb } from "../sdk.ts";

const MAX_PLAY_MS = 120_000;

const streamInput = z.object({
  targets: z
    .array(z.string())
    .min(1)
    .describe(
      "The devices, each an identity (`AA:BB:CC:DD:EE:FF`) or a name that the configuration gives a device (`name:kitchen`). " +
        "Every device takes the same frames at the same time.",
    ),
  mode: modeArg,
  frames: z
    .array(
      z
        .array(rgb)
        .min(1)
        .describe("One color fills every zone. More colors state each zone in order, one per zone of the stream."),
    )
    .min(1)
    .max(1000),
  hold_ms: z.number().int().min(1).max(10_000).default(500).describe("How long each frame stays, in milliseconds."),
  repeat: z.number().int().min(1).max(1000).default(1).describe("How many times the frames play."),
  resolution: resolutionArg,
  rate: z
    .number()
    .positive()
    .optional()
    .describe("Frames per second on the wire. Without it, the rate that the device file measured."),
  gradient: z.boolean().optional(),
});

const streamRow = z.object({
  id: z.string(),
  zones: z.number(),
  rate_hz: z.number(),
  frames_sent: z.number(),
  frames_superseded: z.number(),
  error: z.string().nullable(),
});

type StreamInput = z.infer<typeof streamInput>;

/** One color goes to `fill`. More colors go to `setAll` as bytes, which cross the binding once. */
type Frame = [number, number, number] | Uint8Array;

export function registerStream(server: McpServer): void {
  server.registerTool(
    "stream",
    {
      title: "Stream frames to the segment channel",
      description:
        "Arm the segment channel of each device, play the frames in turn, each for `hold_ms`, then disarm. " +
          "Each device then goes back to the color it showed before. " +
          "Power the devices on first, with `set`: an armed channel on a dark device paints nothing. " +
          `One call plays for ${MAX_PLAY_MS / 1000} s at most. ` +
          "The answer gives, per device, the frames that reached the wire and the frames that a later frame replaced before they left. " +
          "The firmware acknowledges no frame: a frame sent is not a frame shown.",
      inputSchema: streamInput,
      outputSchema: z.object({ played_ms: z.number(), streams: z.array(streamRow) }),
      annotations: CONTROL,
    },
    (input) =>
      attempt(async (govee) => {
        const total = input.frames.length * input.repeat * input.hold_ms;
        if (total > MAX_PLAY_MS)
          throw codeError("invalid_argument", `the playback lasts ${total} ms, over the ${MAX_PLAY_MS} ms of one call`);

        const frames = input.frames.map(prepare);
        const opened = await openAll(govee, input);
        let playedMs: number;
        try {
          const started = performance.now();
          await play(opened, frames, frames.length * input.repeat, input.hold_ms);
          playedMs = Math.round(performance.now() - started);
        } finally {
          await closeAll(opened);
        }
        return ok({ played_ms: playedMs, streams: opened.map(({ id, stream }) => row(id, stream)) });
      }),
  );
}

interface Opened {
  id: string;
  stream: SegmentStream;
}

async function openAll(govee: Govee, input: StreamInput): Promise<Opened[]> {
  const settled = await Promise.allSettled(
    input.targets.map(async (target) => {
      const [handle] = await reach(govee, target, input.mode);
      const stream = await handle.openStream(input.resolution, input.rate ?? "measured", input.gradient);
      return { id: handle.id, stream };
    }),
  );
  const opened = settled.flatMap((s) => (s.status === "fulfilled" ? [s.value] : []));
  const refused = settled.find((s) => s.status === "rejected");
  if (refused === undefined) return opened;
  await closeAll(opened);
  throw refused.reason;
}

/** `setInterval`, not chained timeouts: the clock does not drift. */
function play(opened: Opened[], frames: Frame[], count: number, holdMs: number): Promise<void> {
  return new Promise((resolve) => {
    let next = 0;
    const tick = (): void => {
      const frame = frames[next % frames.length];
      if (next === count || frame === undefined) {
        clearInterval(timer);
        resolve();
        return;
      }
      for (const { stream } of opened) paint(stream, frame);
      next += 1;
    };
    const timer = setInterval(tick, holdMs);
    tick();
  });
}

function prepare(colors: StreamInput["frames"][number]): Frame {
  const [only] = colors;
  return colors.length === 1 && only !== undefined ? only : Uint8Array.from(colors.flat());
}

function paint(stream: SegmentStream, frame: Frame): void {
  if (frame instanceof Uint8Array) stream.setAll(frame);
  else stream.fill(frame);
}

async function closeAll(opened: Opened[]): Promise<void> {
  await Promise.allSettled(opened.map(({ stream }) => stream.close()));
}

function row(id: string, stream: SegmentStream): z.infer<typeof streamRow> {
  return {
    id,
    zones: stream.zones,
    rate_hz: stream.rateHz,
    frames_sent: stream.framesSent,
    frames_superseded: stream.framesSuperseded,
    error: stream.error ?? null,
  };
}
