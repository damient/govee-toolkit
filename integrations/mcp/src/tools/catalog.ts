// The read tools. None of them starts the SDK, needs a configuration or
// reaches the network.

import type { CallToolResult, McpServer } from "@modelcontextprotocol/server";
import { z } from "zod";

import { allSkus, describe, dmx, row, vocabulary } from "../catalog.ts";
import { docTopics, readApi, readDoc } from "../data.ts";
import { UNVERIFIED, codeError, fail, ok, oneOf } from "../result.ts";

const READ = { readOnlyHint: true, openWorldHint: false } as const;

const personality = z.looseObject({ personality: z.string() });

export function registerCatalogTools(server: McpServer): void {
  listDevices(server);
  describeDevice(server);
  getDmxProfile(server);
  getApi(server);
  readDocTool(server);
}

interface DeviceFilter {
  mode?: string | undefined;
  capability?: string | undefined;
  support?: string | undefined;
}

function matches(sku: string, { mode, capability, support }: DeviceFilter): boolean {
  const record = describe(sku);
  const scoped = mode === undefined ? undefined : record.modes[mode];
  if (support !== undefined && scoped?.support !== support) return false;
  if (capability === undefined) return true;
  return (scoped?.capabilities ?? record.capabilities).includes(capability);
}

function listDevices(server: McpServer): void {
  server.registerTool(
    "list_devices",
    {
      title: "List the devices in the catalog",
      description:
        "List every device model the catalog describes, one short row per SKU: name, family and the support of each mode. " +
        "Call it to find a SKU, then call `describe_device` for the detail. It reads the embedded catalog, not the network: " +
        "it does not tell which devices are on this network. " +
        "`mode` scopes the two other filters to one mode. `capability` keeps the devices that declare it. " +
        `\`support\` keeps the devices whose support on \`mode\` has that value, and needs \`mode\`. ${UNVERIFIED}`,
      inputSchema: z.object({
        mode: oneOf(vocabulary.modes).optional().describe("The mode that `capability` and `support` read."),
        capability: oneOf(vocabulary.capabilities).optional(),
        support: oneOf(vocabulary.support).optional(),
      }),
      outputSchema: z.object({
        devices: z.array(
          z.object({
            sku: z.string(),
            alias_of: z.string().nullable().describe("The SKU of the device file when `sku` is a verified alias."),
            name: z.string(),
            family: z.string(),
            support: z.record(z.string(), z.string()),
          }),
        ),
      }),
      annotations: READ,
    },
    (filter) => {
      if (filter.support !== undefined && filter.mode === undefined) {
        return fail(codeError("invalid_argument", "`support` needs `mode`"));
      }
      const devices = allSkus()
        .filter((sku) => matches(sku, filter))
        .map((sku) => row(sku));
      return ok({ devices });
    },
  );
}

function describeDevice(server: McpServer): void {
  server.registerTool(
    "describe_device",
    {
      title: "Describe one device model",
      description:
        "The full record of one SKU: capabilities, segments, the support and the commands of each mode, " +
        "what was verified on hardware, and the DMX channel tables under `dmx`. " +
        "It reads the catalog and no device: it does not report the state of a device on this network. " +
        `Fails with \`unknown_sku\` when the catalog declares no such SKU. ${UNVERIFIED}`,
      inputSchema: z.object({ sku: z.string().describe("A SKU, such as one that `list_devices` returns.") }),
      outputSchema: z.looseObject({
        sku: z.string(),
        name: z.string(),
        dmx: z.object({ personalities: z.array(personality) }),
      }),
      annotations: READ,
    },
    ({ sku }) => attempt(() => ok({ ...describe(sku), dmx: dmx(sku) })),
  );
}

function getDmxProfile(server: McpServer): void {
  server.registerTool(
    "get_dmx_profile",
    {
      title: "Get the DMX channel table of a device",
      description:
        "The DMX channel tables that the `govee-dmx` bridge patches for one SKU, one per personality it serves. " +
        "Call it to patch a fixture on a console. It is read only: this server does not run the bridge. " +
        "A personality that the device serves through no mode is absent, and one wider than a universe carries its error. " +
        "Fails with `unknown_sku` for a SKU that the catalog does not declare, and with `personality_not_served` " +
        "for a personality that the SKU does not serve.",
      inputSchema: z.object({
        sku: z.string(),
        personality: oneOf(vocabulary.personalities).optional(),
      }),
      outputSchema: z.object({ sku: z.string(), personalities: z.array(personality) }),
      annotations: READ,
    },
    ({ sku, personality: wanted }) =>
      attempt(() => {
        const personalities = dmx(sku).personalities.filter((p) => wanted === undefined || p.personality === wanted);
        if (wanted !== undefined && personalities.length === 0) {
          throw codeError("personality_not_served", `${sku} serves no \`${wanted}\` personality`);
        }
        return ok({ sku, personalities });
      }),
  );
}

function getApi(server: McpServer): void {
  const api = readApi();
  server.registerTool(
    "get_api",
    {
      title: "Get the method that serves a role",
      description:
        "For each role (power, brightness, color, and the rest), the methods that serve it on each surface: " +
        "the `govee` CLI, Rust, Node and Python. Call it to write code or a command line against this SDK. " +
        "With `language`, it also lists the full public surface of that language under `methods`. " +
        "A role names what a command does; the command that carries it on a device is in `describe_device`.",
      inputSchema: z.object({
        role: oneOf(Object.keys(api.roles).toSorted()).optional(),
        language: oneOf(Object.keys(api.methods).toSorted()).optional(),
      }),
      outputSchema: z.object({
        roles: z.record(z.string(), z.record(z.string(), z.array(z.string()))),
        methods: z.array(z.string()).optional(),
      }),
      annotations: READ,
    },
    ({ role, language }) => {
      const roles = Object.fromEntries(
        Object.entries(api.roles)
          .filter(([name]) => role === undefined || name === role)
          .map(([name, surfaces]) => [
            name,
            language === undefined ? surfaces : { [language]: surfaces[language] ?? [] },
          ]),
      );
      return ok(language === undefined ? { roles } : { roles, methods: api.methods[language] ?? [] });
    },
  );
}

function readDocTool(server: McpServer): void {
  server.registerTool(
    "read_doc",
    {
      title: "Read one documentation page",
      description:
        "One page of the project documentation, as Markdown: how the modes work, the protocol of each mode, " +
        "the DMX bridge, the compatibility tables, the security notes. Call it when a record from `describe_device` " +
        "points at a page such as `docs/protocol/lan.md`: the topic is then `protocol/lan`.",
      inputSchema: z.object({ topic: oneOf(docTopics()) }),
      outputSchema: z.object({ topic: z.string(), text: z.string() }),
      annotations: READ,
    },
    ({ topic }) => {
      const text = readDoc(topic);
      return { content: [{ type: "text", text }], structuredContent: { topic, text } };
    },
  );
}

/** Turns a thrown binding error into a tool failure. */
function attempt(answer: () => CallToolResult): CallToolResult {
  try {
    return answer();
  } catch (error) {
    return fail(error);
  }
}
