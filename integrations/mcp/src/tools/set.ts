// `set`: the role verbs of a group handle. One device is a group of one, as in
// the CLI, so a device and a group answer the same shape.

import type { CallToolResult, McpServer } from "@modelcontextprotocol/server";
import type { GroupHandle, Outcome } from "govee-toolkit";
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
type Step = [string, (group: GroupHandle) => Promise<Outcome[]>];
type Done = { step: string; outcomes: Record<string, unknown>[] }[];

/** The steps in the order they are sent: power on first, power off last. */
function steps(input: SetInput): Step[] {
  const { power, brightness, color, color_temp, segment, music, gradient } = input;
  const list: Step[] = [];
  if (power === true) list.push(["power", (g) => g.power(true)]);
  if (gradient !== undefined) list.push(["gradient", (g) => g.gradient(gradient)]);
  if (brightness !== undefined) list.push(["brightness", (g) => g.brightness(brightness)]);
  if (color_temp !== undefined) list.push(["color_temp", (g) => g.colorTemp(color_temp)]);
  if (color !== undefined) list.push(["color", (g) => g.color(color)]);
  if (segment !== undefined) {
    const { colors, zones, resolution, gradient: blend } = segment;
    const paint = colors.length === 1 && colors[0] !== undefined ? colors[0] : colors;
    list.push(["segment", (g) => g.segment(paint, zones ?? null, resolution, blend ?? null)]);
  }
  if (music !== undefined) {
    const { effect, sensitivity, soft, color: tint } = music;
    list.push(["music", (g) => g.music(effect, sensitivity ?? null, soft ?? null, tint)]);
  }
  if (power === false) list.push(["power", (g) => g.power(false)]);
  return list;
}

export function registerSet(server: McpServer): void {
  server.registerTool(
    "set",
    {
      title: "Set the state of a device or a group",
      description:
        "Set one or more of power, brightness, color, white temperature, segments, a music effect and the gradient, " +
        "on one device or on every member of a group. The steps go out in a fixed order, and the first step that " +
        "fails stops the ones after it. Each outcome names the mode that served it. " +
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
      attempt((govee) => {
        const plan = steps(input);
        if (plan.length === 0) throw codeError("invalid_argument", "name at least one value to set");
        const group = govee.group(input.target, input.mode ?? null);
        return run(group, [["reach", (g) => g.ensureKnown()], ...plan], []);
      }),
  );
}

/** Sends each step after the one before, and stops at the first step that a member fails. */
async function run(group: GroupHandle, [next, ...rest]: Step[], done: Done): Promise<CallToolResult> {
  if (next === undefined) return ok({ members: group.members, steps: done });
  const [step, call] = next;
  const outcomes = await call(group);
  done.push({ step, outcomes: outcomes.map((o) => outcomeRow(o)) });
  const failed = outcomes.filter((o) => !o.ok);
  if (failed.length === 0) return run(group, rest, done);
  // One device fails with its own error, as a command on one device does.
  const first = failure(failed[0]?.error);
  const summary =
    outcomes.length === 1
      ? `${first.code}: ${first.message}`
      : `${first.code}: ${failed.length} of ${outcomes.length} devices failed at ${step}`;
  return { isError: true, content: [{ type: "text", text: `${summary}\n${JSON.stringify({ steps: done })}` }] };
}
