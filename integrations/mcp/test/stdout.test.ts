// stdout carries the JSON-RPC stream: one byte of log there breaks the client.

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { LATEST_PROTOCOL_VERSION } from "@modelcontextprotocol/client";
import { Catalog } from "govee-toolkit";
import { z } from "zod";

import { isolated } from "./helpers.ts";

const messageShape = z.looseObject({ jsonrpc: z.literal("2.0"), id: z.number().optional() });

const main = fileURLToPath(new URL("../src/main.ts", import.meta.url));

const requests = [
  {
    method: "initialize",
    params: {
      protocolVersion: LATEST_PROTOCOL_VERSION,
      capabilities: {},
      clientInfo: { name: "govee-toolkit-mcp-test", version: "0.0.0" },
    },
  },
  { method: "tools/list", params: {} },
  { method: "tools/call", params: { name: "describe_device", arguments: { sku: Catalog.embedded().skus()[0] } } },
  { method: "tools/call", params: { name: "describe_device", arguments: { sku: "NOT-A-SKU" } } },
  // Starts the SDK, so what the core writes at startup is covered too.
  { method: "tools/call", params: { name: "doctor", arguments: {} } },
  { method: "resources/list", params: {} },
];

test("stdout carries JSON-RPC and nothing else, and the server exits when stdin closes", { timeout: 30_000 }, async () => {
  const child = spawn(process.execPath, [main], {
    env: { ...process.env, ...isolated },
    stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "";
  child.stdout.setEncoding("utf8").on("data", (chunk: string) => {
    stdout += chunk;
  });
  child.stderr.resume();

  const [first, ...rest] = requests.map((request, id) => JSON.stringify({ jsonrpc: "2.0", id, ...request }));
  child.stdin.write(`${first}\n`);
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })}\n`);
  for (const line of rest) child.stdin.write(`${line}\n`);

  const answered = new Promise<void>((resolve) => {
    child.stdout.on("data", () => {
      if (stdout.split("\n").filter((line) => line.includes('"id"')).length >= requests.length) resolve();
    });
  });
  await answered;
  child.stdin.end();
  const [code] = z.tuple([z.number().nullable()]).rest(z.unknown()).parse(await once(child, "exit"));
  assert.equal(code, 0);

  const lines = stdout.split("\n").filter((line) => line.length > 0);
  const messages = lines.map((line) => messageShape.parse(JSON.parse(line)));
  const ids = messages.flatMap((m) => (m.id === undefined ? [] : [m.id]));
  assert.deepEqual(
    ids.toSorted((a, b) => a - b),
    requests.map((_, id) => id),
  );
  // A tool failure is a result with `isError`: a JSON-RPC error is a fault of the server.
  assert.ok(
    messages.every((m) => !("error" in m)),
    stdout,
  );
});
