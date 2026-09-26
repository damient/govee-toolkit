// `set`: the role verbs of a group handle. One device is a group of one, as in
// the CLI, so a device and a group answer the same shape.

import type { CallToolResult, McpServer } from "@modelcontextprotocol/server";
import type { Applied, GroupHandle } from "govee-toolkit";
import { z } from "zod";

import { codeError, failure, ok } from "../result.ts";
import { CONTROL, TARGET, attempt, modeArg, outcomeRow, rgb } from "../sdk.ts";

const setInput = z.object({
  target: z.string().describe(TARGET),
  mode: modeArg,
  power: z.boolean().optional(),
  brightness: z.number().int().optional().describe("In the unit and the range that the device file declares."),
  color: rgb.optional(),
  color_temp: z.number().int().optional().describe("Kelvin, in the range that the device file declares."),
  segment: z
    .object({
      colors: z.array(rgb).min(1).describe("One color fills every zone. More colors state each zone in order."),
      zones: z.array(z.number().int().min(0)).optional().describe("The zones that take the one color."),
      resolution: z.union([z.number().int().min(1), z.enum(["app", "native", "groups"])]).optional(),
      gradient: z.boolean().optional(),
    })
    .optional(),
  music: z
    .object({
      effect: z.number().int(),
      sensitivity: z.number().int().optional(),
      soft: z.boolean().optional(),
      color: rgb.optional(),
    })
    .optional(),
  gradient: z.boolean().optional(),
});

const outcomeShape = z.object({
  id: z.string(),
  ok: z.boolean(),
  mode: z.string().nullable(),
  command: z.string().nullable(),
  error: z.object({ code: z.string(), message: z.string() }).nullable(),
});

type SetInput = z.infer<typeof setInput>;

/** The verbs of the input, under the keys that `GroupHandle.apply()` takes. The core orders them. */
function verbs(input: SetInput): Parameters<GroupHandle["apply"]>[0] {
  const { power, brightness, color, color_temp: colorTemp, segment, music, gradient } = input;
  const all = { power, brightness, color, colorTemp, segment, music, gradient };
  return Object.fromEntries(Object.entries(all).filter(([, value]) => value !== undefined));
}

export function registerSet(server: McpServer): void {
  server.registerTool(
    "set",
    {
      title: "Set the state of a device or a group",
      description:
        "Set one or more of power, brightness, color, white temperature, segments, a music effect and the gradient, " +
        "on one device or on every member of a group. The steps go out in a fixed order, power on first and power off last. " +
        "A member that fails a step takes no later step, and the other members go on. " +
        "Each outcome names the mode that served it. " +
        "A value outside the range that the device file declares fails with the codec error: nothing is clamped. " +
        "To send a command that no argument here names, call `send`.",
      inputSchema: setInput,
      outputSchema: z.object({
        members: z.array(z.string()),
        steps: z.array(z.object({ step: z.string(), outcomes: z.array(outcomeShape) })),
      }),
      annotations: CONTROL,
    },
    (input) =>
      attempt(async (govee) => {
        const plan = verbs(input);
        if (Object.keys(plan).length === 0) throw codeError("invalid_argument", "name at least one value to set");
        const group = govee.group(input.target, input.mode ?? null);
        return answer(group.members, await group.apply(plan));
      }),
  );
}

/** The scan as the `reach` step, then each verb. A failure names the first error and how many members failed. */
function answer(members: string[], applied: Applied): CallToolResult {
  const steps = [
    { step: "reach", outcomes: applied.reached.map((o) => outcomeRow(o)) },
    ...applied.steps.map(({ step, outcomes }) => ({ step, outcomes: outcomes.map((o) => outcomeRow(o)) })),
  ];
  if (applied.ok) return ok({ members, steps });
  const failed = [applied.reached, ...applied.steps.map((s) => s.outcomes)].flat().filter((o) => !o.ok);
  const first = failure(failed[0]?.error);
  // One device fails with its own error, as a command on one device does.
  const summary =
    members.length === 1
      ? `${first.code}: ${first.message}`
      : `${first.code}: ${new Set(failed.map((o) => o.id)).size} of ${members.length} devices failed`;
  return { isError: true, content: [{ type: "text", text: `${summary}\n${JSON.stringify({ steps })}` }] };
}
