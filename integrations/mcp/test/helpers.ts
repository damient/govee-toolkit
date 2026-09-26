// What every test shares. `node --test` runs each file in a process of its
// own, so the environment set here reaches the SDK before it starts.

import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { Client, InMemoryTransport } from "@modelcontextprotocol/client";
import type { CallToolResult } from "@modelcontextprotocol/client";
import type { z } from "zod";

import { createServer } from "../src/server.ts";

// Point every path the core reads at an empty temporary directory. A cache
// that a real run wrote puts devices in `list_known` that no test found, and
// the checkout's own `.env` names a configuration through `GOVEE_CONFIG`.
const state = mkdtempSync(join(tmpdir(), "govee-mcp-test-"));
const emptyEnv = join(state, "env");
writeFileSync(emptyEnv, "");

/** The variables that isolate an SDK, for this process or a child. */
export const isolated = {
  XDG_CONFIG_HOME: join(state, "config"),
  XDG_CACHE_HOME: join(state, "cache"),
  GOVEE_ENV_FILE: emptyEnv,
  // A file that is not there is the default configuration, not an error.
  GOVEE_CONFIG: join(state, "config.yaml"),
};
Object.assign(process.env, isolated);

/** A v2 client connected in-process to a new server. */
export async function connect(): Promise<Client> {
  const [clientSide, serverSide] = InMemoryTransport.createLinkedPair();
  await createServer().connect(serverSide);
  const client = new Client({ name: "govee-toolkit-mcp-test", version: "0.0.0" });
  await client.connect(clientSide);
  return client;
}

/**
 * Calls a tool that must succeed, and parses its structured content. The
 * client first validates that content against the `outputSchema` of the
 * tool, and throws when it does not match.
 */
export async function structured<T>(
  client: Client,
  name: string,
  args: Record<string, unknown>,
  shape: z.ZodType<T>,
): Promise<T> {
  const result = await client.callTool({ name, arguments: args });
  assert.notEqual(result.isError, true, `${name}: ${text(result)}`);
  return shape.parse(result.structuredContent);
}

/** Calls a tool that must fail, and returns the binding code the text starts with. */
export async function failureCode(client: Client, name: string, args: Record<string, unknown>): Promise<string> {
  const result = await client.callTool({ name, arguments: args });
  assert.equal(result.isError, true, `${name} did not fail`);
  return text(result).split(":")[0] ?? "";
}

function text(result: CallToolResult): string {
  return result.content.map((block) => (block.type === "text" ? block.text : "")).join("");
}
