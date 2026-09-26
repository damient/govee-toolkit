// The control tools. Each one starts the SDK on its first call and reaches the
// devices over the modes that the configuration enables. None of them retries
// on another mode: a binding error is the answer.

import type { McpServer } from "@modelcontextprotocol/server";
import { z } from "zod";

import { vocabulary } from "../catalog.ts";
import { ok, oneOf } from "../result.ts";
import { CONTROL, TARGET, attempt, deviceRow, modeArg, reach, rgb, statusRow } from "../sdk.ts";
import { registerSet } from "./set.ts";

const deviceShape = z.looseObject({ id: z.string(), sku: z.string(), modes: z.array(z.string()) });

export function registerControlTools(server: McpServer): void {
  scan(server);
  listKnown(server);
  status(server);
  registerSet(server);
  send(server);
  identify(server);
  doctor(server);
}

function scan(server: McpServer): void {
  server.registerTool(
    "scan",
    {
      title: "Scan the network for devices",
      description:
        "Run a discovery scan and list the devices that answered, with their health per mode. " +
        "Call it before a command to a device that `list_known` does not list. It takes the scan window of each mode. " +
        "Without `modes`, it scans every mode that this build carries.",
      inputSchema: z.object({ modes: z.array(oneOf(vocabulary.modes)).optional() }),
      outputSchema: z.object({ devices: z.array(deviceShape) }),
      annotations: CONTROL,
    },
    ({ modes }) =>
      attempt(async (govee) => {
        const found = modes === undefined ? await govee.scan() : await govee.scanOn(modes);
        return ok({ devices: found.map((device) => deviceRow(device)) });
      }),
  );
}

function listKnown(server: McpServer): void {
  server.registerTool(
    "list_known",
    {
      title: "List the devices the SDK knows",
      description:
        "List every device that a scan found or that the configuration names, with its health per mode. " +
        "It sends nothing. A device that no scan found yet has no health: call `scan` first.",
      inputSchema: z.object({}),
      outputSchema: z.object({ devices: z.array(deviceShape) }),
      annotations: { ...CONTROL, readOnlyHint: true },
    },
    () => attempt((govee) => ok({ devices: govee.devices().map((device) => deviceRow(device)) })),
  );
}

function status(server: McpServer): void {
  server.registerTool(
    "status",
    {
      title: "Ask a device for its state",
      description:
        "Ask one device for its state and wait for the answer: power, brightness, color and white temperature. " +
        "No firmware fills every field: `null` is a field that the device did not report. " +
        "`raw` holds the whole answer. Fails with `unknown_device` when no scan finds the device.",
      inputSchema: z.object({ target: z.string().describe(TARGET), mode: modeArg }),
      outputSchema: z.looseObject({ id: z.string(), mode: z.string() }),
      annotations: CONTROL,
    },
    ({ target, mode: pinned }) =>
      attempt(async (govee) => {
        const [handle, serving] = await reach(govee, target, pinned);
        return ok(statusRow(await handle.status(), serving));
      }),
  );
}

const argValue = z.union([z.boolean(), z.number(), z.string(), z.array(z.number()), z.array(rgb)]);

function send(server: McpServer): void {
  server.registerTool(
    "send",
    {
      title: "Send one command from the device file",
      description:
        "Send one command to one device, named as its device file names it, with the arguments that the entry declares. " +
        "`describe_device` lists the commands of each mode and their arguments. " +
        "A command that declares a reply is read, and the answer holds the fields it captured. " +
        "A value outside the declared range fails, and nothing is sent. " +
        "A command that takes a secret, such as a network password, fails with `secret_arg`: " +
          "the secret would travel through this conversation.",
      inputSchema: z.object({
        target: z.string().describe(TARGET),
        mode: modeArg,
        command: z.string(),
        args: z.record(z.string(), argValue).optional(),
      }),
      outputSchema: z.looseObject({ id: z.string(), mode: z.string(), command: z.string() }),
      annotations: CONTROL,
    },
    ({ target, mode: pinned, command, args }) =>
      attempt(async (govee) => {
        const [handle] = await reach(govee, target, pinned);
        const done = await handle.invoke(command, args, true);
        const answer = { id: done.id, mode: done.mode, command: done.command };
        return ok(done.fields === null ? answer : { ...answer, fields: done.fields as unknown });
      }),
  );
}

function identify(server: McpServer): void {
  server.registerTool(
    "identify",
    {
      title: "Light the devices one by one",
      description:
        "Walk the devices: black out every one, then light each in green, one after the other, so a person sees " +
        "which fixture each identity drives. Without `targets`, it walks every device that a scan finds. " +
        "Without `mode`, it walks over `lan`. The devices lose the look they held. " +
        "The answer lists the devices lit, the ones that failed, and the ones that still hold the color.",
      inputSchema: z.object({
        targets: z
          .array(z.string())
          .min(1)
          .optional()
          .describe(`Each one is a SKU, which selects among the devices a scan found, or else: ${TARGET}`),
        mode: modeArg,
      }),
      outputSchema: z.object({
        ok: z.boolean(),
        lit: z.array(z.string()),
        failed: z.array(z.string()),
        stayed: z.array(z.string()),
      }),
      annotations: CONTROL,
    },
    ({ targets, mode: pinned }) =>
      attempt(async (govee) => {
        const report = await govee.identify(targets ?? null, pinned === undefined ? {} : { mode: pinned });
        return ok({ ok: report.ok, lit: report.lit, failed: report.failed, stayed: report.stayed });
      }),
  );
}

function doctor(server: McpServer): void {
  server.registerTool(
    "doctor",
    {
      title: "Check the configuration",
      description:
        "List everything wrong with the configuration, one sentence each, with the modes this build carries " +
        "and the configuration in force. Call it when a control tool fails with `mode_not_enabled` or `config`. " +
        "It sends nothing, and the configuration it returns holds no credential.",
      inputSchema: z.object({}),
      outputSchema: z.object({ problems: z.array(z.string()), modes: z.array(z.string()), config: z.unknown() }),
      annotations: { ...CONTROL, readOnlyHint: true },
    },
    () =>
      attempt((govee) =>
        ok({ problems: govee.problems(), modes: govee.modes(), config: govee.config.toJSON() as unknown }),
      ),
  );
}
